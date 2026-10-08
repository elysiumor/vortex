//! Rename movie and episode files, and the folders they sit in, to their TMDb
//! names: `Title (Year)\Title (Year).mkv` and
//! `Show (Year)\Season 01\Show (Year) - S01E01 - Episode.mkv`. Everything is
//! planned first and shown to the user; nothing on disk changes until they
//! apply it. Each applied batch is recorded so the last one can be undone.
//!
//! What moves along: subtitles and other sidecars named after the video
//! (`Movie.en.srt`, `Movie.nfo`), and a folder's extras when the folder is
//! renamed. What is left alone: titles without a TMDb match, files on a drive
//! that is not connected, and folders shared with other titles or that are a
//! library themselves (only the file is renamed there).

use crate::{db, jobs, parser, tmdb, AppState};
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tauri::{AppHandle, Manager};

/// One rename on disk, file or folder.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Move {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub folder: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Plan {
    pub media_item_id: i64,
    /// The title as the library shows it now.
    pub title: String,
    /// "Inception (2010)", once known.
    pub target: Option<String>,
    /// TMDb's title as TMDb spells it.
    #[serde(skip)]
    pub target_title: String,
    /// In execution order: files first, then their folder. Empty when the
    /// title is already named this way.
    pub moves: Vec<Move>,
    pub skipped: Option<String>,
    /// Files left as they are, with why.
    pub notes: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Report {
    pub renamed: usize,
    pub unchanged: usize,
    /// (title, reason)
    pub failed: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize, Default)]
struct History {
    /// Most recent last. Each batch is the moves of one apply, in the order
    /// they were made.
    batches: Vec<Vec<Move>>,
}

const HISTORY_FILE: &str = "rename-history.json";
const HISTORY_KEEP: usize = 20;
/// Leaves room under Windows' classic 260-character path limit, which some
/// players still trip over.
const MAX_PATH_CHARS: usize = 250;

static PART: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[ ._\-\[(])(?:cd|disc|disk|part|pt)[ ._-]?(\d{1,2})(?:$|[ ._\-\])])").unwrap()
});
static QUALITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(2160p|1080p|720p|576p|480p)\b").unwrap());

/// A name Windows accepts. "Mission: Impossible" becomes "Mission - Impossible".
pub fn safe_name(s: &str) -> String {
    let s = s.replace(": ", " - ").replace(':', "-");
    let cleaned: String =
        s.chars().filter(|c| !matches!(c, '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*') && !c.is_control()).collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = collapsed.trim_end_matches(['.', ' ']).to_string();
    let upper = out.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((upper.starts_with("COM") || upper.starts_with("LPT")) && upper.len() == 4 && upper.as_bytes()[3].is_ascii_digit());
    if reserved {
        out.push('_');
    }
    out
}

fn base_name(title: &str, year: Option<i32>) -> String {
    match year {
        Some(y) => safe_name(&format!("{title} ({y})")),
        None => safe_name(title),
    }
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

fn ext(p: &Path) -> String {
    p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default()
}

/// Windows paths compare without regard to case.
fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

/// New stems for the main files sharing one folder. A single file takes the
/// base name; several get a part number, a quality, or a counter.
fn stems_for(paths: &[&str], base: &str) -> Vec<String> {
    if paths.len() == 1 {
        return vec![base.to_string()];
    }
    let parts: Vec<Option<String>> = paths
        .iter()
        .map(|p| PART.captures(&stem(Path::new(p))).map(|c| format!("pt{}", c[1].parse::<u32>().unwrap_or(0))))
        .collect();
    let qualities: Vec<Option<String>> =
        paths.iter().map(|p| QUALITY.captures(&file_name(Path::new(p))).map(|c| c[1].to_lowercase())).collect();
    let distinct_quality = {
        let q: Vec<&String> = qualities.iter().flatten().collect();
        q.len() == paths.len() && q.iter().collect::<HashSet<_>>().len() == q.len()
    };
    (0..paths.len())
        .map(|i| match (&parts[i], distinct_quality) {
            (Some(p), _) => format!("{base} - {p}"),
            (None, true) => format!("{base} - {}", qualities[i].as_deref().unwrap_or_default()),
            (None, false) => format!("{base} - {}", i + 1),
        })
        .collect()
}

/// Whether files sharing a folder differ by a part number or a quality, the
/// way `stems_for` names them, rather than only by a counter.
fn told_apart(paths: &[&str]) -> bool {
    let parts: Vec<Option<String>> = paths.iter().map(|p| PART.captures(&stem(Path::new(p))).map(|c| c[1].to_string())).collect();
    let qualities: Vec<Option<String>> = paths.iter().map(|p| QUALITY.captures(&file_name(Path::new(p))).map(|c| c[1].to_lowercase())).collect();
    let distinct = |v: &[Option<String>]| v.iter().all(Option::is_some) && v.iter().collect::<HashSet<_>>().len() == v.len();
    distinct(&parts) || distinct(&qualities)
}

/// Files in `dir` named after `video` ("Movie.en.srt", "Movie.nfo"), with the
/// rest of their name after the video's stem. A file that also matches a
/// longer stem of another video in the folder belongs to that one.
fn sidecars(video: &Path) -> Vec<(PathBuf, String)> {
    let Some(dir) = video.parent() else { return Vec::new() };
    let own = stem(video);
    let all_stems = video_stems_in(dir);
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !parser::is_video(p))
        .filter_map(|p| {
            let name = file_name(&p);
            let rest = named_after(&name, &own)?;
            // "It.Follows.en.srt" starts with "It." too; it belongs to the
            // longer name, whether or not that video is being renamed.
            let longer = all_stems.iter().any(|s| s.chars().count() > own.chars().count() && named_after(&name, s).is_some());
            (!longer).then(|| (p.clone(), rest))
        })
        .collect()
}

/// What follows `stem` in `name`, if `name` starts with it (ignoring case)
/// and the stem ends there: "Movie.en.srt" and "Movie_eng.srt" are named after
/// "Movie", "Movies.txt" is not. Compared character by character, since
/// lowercasing can change a name's length in bytes ("İ").
fn named_after(name: &str, stem: &str) -> Option<String> {
    let mut chars = name.chars();
    for s in stem.chars() {
        let c = chars.next()?;
        if !c.to_lowercase().eq(s.to_lowercase()) {
            return None;
        }
    }
    let rest: String = chars.collect();
    match rest.chars().next() {
        Some(c) if !c.is_alphanumeric() => Some(rest),
        _ => None,
    }
}

/// Stems of every video file in a folder, this title's or not.
fn video_stems_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_file() && parser::is_video(p)).map(|p| stem(&p)).collect())
        .unwrap_or_default()
}

/// What TMDb says about a matched title: its name and year, and for a series
/// how many episodes each season has.
struct TmdbInfo {
    tmdb_id: i64,
    title: String,
    year: Option<i32>,
    /// Season number to episode count. Empty for movies.
    seasons: BTreeMap<i32, i32>,
}

/// From the cached details when there are any (no network for a
/// whole-library preview), else fetched.
fn tmdb_info(app: &AppHandle, media_item_id: i64, kind: &str, tmdb_id: i64) -> Result<Option<TmdbInfo>, String> {
    let cached = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        db::get_details_json(&conn, kind, tmdb_id).map_err(|e| e.to_string())?
    };
    let year_of = |d: &str| d.get(..4).and_then(|y| y.parse::<i32>().ok());
    if let Some((json, _)) = cached {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            let title = v["title"].as_str().or(v["name"].as_str()).map(str::to_string);
            let date = v["release_date"].as_str().or(v["first_air_date"].as_str()).unwrap_or("");
            let seasons: BTreeMap<i32, i32> = v["seasons"]
                .as_array()
                .map(|a| a.iter().filter_map(|s| Some((s["season_number"].as_i64()? as i32, s["episode_count"].as_i64()? as i32))).collect())
                .unwrap_or_default();
            if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
                return Ok(Some(TmdbInfo { tmdb_id, title: t, year: year_of(date), seasons }));
            }
        }
    }
    Ok(tmdb::get_details(app, media_item_id, false)?.map(|d| TmdbInfo {
        tmdb_id,
        year: d.release_date.as_deref().and_then(year_of),
        seasons: d.seasons.iter().map(|s| (s.season_number, s.episode_count)).collect(),
        title: d.title,
    }))
}

/// The plan for one title. Never touches the disk beyond reading folders.
pub fn plan_item(app: &AppHandle, media_item_id: i64) -> Plan {
    let mut plan = Plan {
        media_item_id,
        title: String::new(),
        target: None,
        target_title: String::new(),
        moves: Vec::new(),
        skipped: None,
        notes: Vec::new(),
    };
    if let Err(e) = plan_into(app, &mut plan) {
        plan.moves.clear();
        plan.skipped = Some(e);
    }
    plan
}

fn plan_into(app: &AppHandle, plan: &mut Plan) -> Result<(), String> {
    let id = plan.media_item_id;
    let (item, episodes, roots) = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let item = db::get_media_item(&conn, id).map_err(|e| e.to_string())?.ok_or("This title is no longer in the library.")?;
        let episodes = db::list_episodes(&conn, id).map_err(|e| e.to_string())?;
        let roots: Vec<String> = db::list_libraries_rows(&conn).map_err(|e| e.to_string())?.into_iter().map(|l| l.path).collect();
        (item, episodes, roots)
    };
    let mut episodes = episodes;
    db::fill_available(episodes.iter_mut());
    plan.title = item.title.clone();
    let tmdb_id = item.tmdb_id.ok_or("Not matched to TMDb. Use Fix match first.")?;
    let info = tmdb_info(app, id, &item.kind, tmdb_id)?.ok_or("No TMDb details for this title.")?;
    let base = base_name(&info.title, info.year);
    if base.is_empty() {
        return Err("TMDb's title has no usable characters.".into());
    }
    plan.target = Some(base.clone());
    plan.target_title = info.title.clone();

    let mut moves = match item.kind.as_str() {
        "movie" => movie_moves(app, plan, &item, &episodes, &roots, &info, &base)?,
        "series" => series_moves(app, &item, &episodes, &roots, &info, &base)?,
        other => return Err(format!("Cannot rename a {other}.")),
    };
    moves.retain(|m| m.from != m.to);
    reads_back(app, &item, &info, &roots, &moves)?;

    // Collisions: two files aiming at one name, or a name already taken.
    let mut targets = HashSet::new();
    for m in &moves {
        if !targets.insert(m.to.to_lowercase()) {
            return Err(format!("Two files would both become {}", file_name(Path::new(&m.to))));
        }
        let (from, to) = (Path::new(&m.from), Path::new(&m.to));
        if to.exists() && !same_path(from, to) {
            return Err(format!("{} already exists.", m.to));
        }
    }
    if let Some(long) = moves.iter().map(|m| final_path(&m.to, &moves)).find(|p| p.chars().count() > MAX_PATH_CHARS) {
        return Err(format!("The new path would be too long for Windows: {long}"));
    }
    plan.moves = moves;
    Ok(())
}

fn movie_moves(
    app: &AppHandle,
    plan: &mut Plan,
    item: &db::MediaItem,
    episodes: &[db::Episode],
    roots: &[String],
    info: &TmdbInfo,
    base: &str,
) -> Result<Vec<Move>, String> {
    let year = info.year;
    let main: Vec<&db::Episode> = episodes.iter().filter(|e| e.extra.is_none()).collect();
    if main.is_empty() {
        return Err("No movie file.".into());
    }
    // Titles are grouped by name alone, so a remake can sit with the original
    // ("Dune" 1984 and 2021). A file whose own name carries a year more than
    // one off TMDb's is another film: it keeps its name, and no folder is
    // renamed, since that would carry it along.
    let year_of_file = |path: &str| {
        let p = Path::new(path);
        parser::year_in(&stem(p)).or_else(|| p.parent().and_then(|d| parser::year_in(&file_name(d))))
    };
    let (main, other_films): (Vec<&db::Episode>, Vec<&db::Episode>) = main.into_iter().partition(|e| match (year, year_of_file(&e.path)) {
        (Some(want), Some(got)) => (want - got).abs() <= 1,
        _ => true,
    });
    for e in &other_films {
        let got = year_of_file(&e.path).map(|y| y.to_string()).unwrap_or_default();
        plan.notes.push(format!("{} is left as it is: its name says {got}, TMDb says {}.", e.file_name, year.unwrap_or_default()));
    }
    if main.is_empty() {
        return Err(format!("No file here is from {}. Check the match with Fix match.", year.unwrap_or_default()));
    }
    if let Some(off) = main.iter().find(|e| !e.available) {
        return Err(format!("Not available: {}. Is the drive connected?", off.path));
    }

    // Files, grouped by the folder they share.
    let mut by_dir: HashMap<PathBuf, Vec<&db::Episode>> = HashMap::new();
    for e in &main {
        let dir = Path::new(&e.path).parent().map(Path::to_path_buf).unwrap_or_default();
        by_dir.entry(dir).or_default().push(e);
    }
    let mut moves: Vec<Move> = Vec::new();
    for files in by_dir.values() {
        let mut files = files.clone();
        files.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        for (f, new_stem) in files.iter().zip(stems_for(&paths, base)) {
            push_file(&mut moves, Path::new(&f.path), &new_stem);
        }
    }

    // The folder too, when it is this title's alone.
    if by_dir.len() == 1 && other_films.is_empty() {
        let dir = by_dir.keys().next().cloned().unwrap_or_default();
        if let Some(m) = own_folder_move(app, &dir, item, info, roots, base)? {
            moves.push(m);
        }
    }
    Ok(moves)
}

/// A video and its sidecars, renamed to `new_stem` in the same folder.
fn push_file(moves: &mut Vec<Move>, from: &Path, new_stem: &str) {
    for (side, rest) in sidecars(from) {
        moves.push(Move {
            from: side.to_string_lossy().into(),
            to: side.with_file_name(format!("{new_stem}{rest}")).to_string_lossy().into(),
            folder: false,
        });
    }
    let to = from.with_file_name(format!("{new_stem}{}", ext(from)));
    moves.push(Move { from: from.to_string_lossy().into(), to: to.to_string_lossy().into(), folder: false });
}

/// Rename the title's own folder to `base`, when it is named after the title
/// (a category folder holding one film, "Movies", keeps its name), is not a
/// library, and holds nothing of any other title.
/// A folder whose name reads as this title ("Inception.2010.1080p" for
/// Inception), as opposed to a category or library folder ("Movies", "TV").
fn named_after_title(dir: &Path, item: &db::MediaItem, info: &TmdbInfo) -> bool {
    let (folder_title, _) = parser::folder_title_of(&file_name(dir));
    names_agree(&folder_title, &item.title) || names_agree(&folder_title, &info.title)
}

fn own_folder_move(app: &AppHandle, dir: &Path, item: &db::MediaItem, info: &TmdbInfo, roots: &[String], base: &str) -> Result<Option<Move>, String> {
    if !named_after_title(dir, item, info) || file_name(dir) == base || !folder_is_own(app, dir, item.id, roots)? {
        return Ok(None);
    }
    let new_dir = dir.with_file_name(base);
    if new_dir.exists() && !same_path(&new_dir, dir) {
        return Ok(None);
    }
    Ok(Some(Move { from: dir.to_string_lossy().into(), to: new_dir.to_string_lossy().into(), folder: true }))
}

/// TMDb's season and episode for a file. Numbers TMDb has are kept. Anime is
/// often numbered straight through ("Show - 27"); a number past the end of
/// its season is then counted across TMDb's seasons instead. Not when the
/// name spelled out the season (S01E27 means what it says), and inside a
/// season folder only if the count lands in that same season.
fn place(seasons: &BTreeMap<i32, i32>, season: i32, episode: i32, spelled_out: bool, in_season_folder: bool) -> Option<(i32, i32)> {
    match seasons.get(&season) {
        Some(&count) if episode >= 1 && episode <= count => return Some((season, episode)),
        // TMDb lists no seasons at all: nothing to check against.
        _ if seasons.is_empty() => return Some((season, episode)),
        _ => {}
    }
    if spelled_out || season == 0 || episode < 1 {
        return None;
    }
    let mut left = episode;
    for (&s, &count) in seasons.iter().filter(|(s, _)| **s >= 1) {
        if left <= count {
            return (!in_season_folder || s == season).then_some((s, left));
        }
        left -= count;
    }
    None
}

/// Longest an episode title may make a file name, so a wordy one does not
/// push the path past what Windows allows.
const MAX_EPISODE_TITLE: usize = 80;

fn series_moves(app: &AppHandle, item: &db::MediaItem, episodes: &[db::Episode], roots: &[String], info: &TmdbInfo, base: &str) -> Result<Vec<Move>, String> {
    let main: Vec<&db::Episode> = episodes.iter().filter(|e| e.extra.is_none()).collect();
    if main.is_empty() {
        return Err("No episode files.".into());
    }
    if let Some(off) = main.iter().find(|e| !e.available) {
        return Err(format!("Not available: {}. Is the drive connected?", off.path));
    }

    // Where each file sits on TMDb. All or nothing: a series renamed only in
    // part would come back from the next scan as two titles.
    let mut placed: Vec<(&db::Episode, i32, i32, Option<i32>)> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for e in &main {
        let (Some(season), Some(episode)) = (e.season, e.episode) else {
            problems.push(format!("{} has no episode number", e.file_name));
            continue;
        };
        let code = parser::episode_code(&stem(Path::new(&e.path)));
        let in_season_folder = Path::new(&e.path).parent().map(|d| parser::is_season_dir(&file_name(d))).unwrap_or(false);
        match place(&info.seasons, season, episode, code.is_some(), in_season_folder) {
            Some((s, ep)) => {
                let last = code.and_then(|(_, first, last)| last.map(|l| ep + (l - first)));
                placed.push((e, s, ep, last));
            }
            None => problems.push(format!("{}: TMDb has no season {season} episode {episode}", e.file_name)),
        }
    }
    if !problems.is_empty() {
        let more = problems.len().saturating_sub(2);
        let mut why = problems.into_iter().take(2).collect::<Vec<_>>().join("; ");
        if more > 0 {
            why.push_str(&format!(" (and {more} more)"));
        }
        return Err(format!("{why}. Nothing in this series was renamed; check the match or the numbering."));
    }

    // Episode names: the library's, then TMDb's for seasons it has none for
    // (anime numbered straight through never matched those seasons).
    let mut names: HashMap<(i32, i32), String> = HashMap::new();
    for e in episodes {
        if let (Some(s), Some(ep), Some(t)) = (e.season, e.episode, e.title.as_ref()) {
            names.insert((s, ep), t.clone());
        }
    }
    let mut fetched: HashSet<i32> = HashSet::new();
    for &(_, s, ep, _) in &placed {
        if !names.contains_key(&(s, ep)) && fetched.insert(s) {
            if let Ok(found) = tmdb::season_names(app, info.tmdb_id, s) {
                for (n, t) in found {
                    names.entry((s, n)).or_insert(t);
                }
            }
        }
    }

    // New names, then told apart within each folder where two copies of one
    // episode would collide.
    let mut groups: HashMap<(PathBuf, String), Vec<&db::Episode>> = HashMap::new();
    for &(e, s, ep, last) in &placed {
        let code = match last {
            Some(l) if l > ep => format!("S{s:02}E{ep:02}-E{l:02}"),
            _ => format!("S{s:02}E{ep:02}"),
        };
        let title = names
            .get(&(s, ep))
            .filter(|_| last.is_none())
            .map(|t| safe_name(&t.chars().take(MAX_EPISODE_TITLE).collect::<String>()))
            .filter(|t| !t.is_empty());
        let new_stem = match title {
            Some(t) => format!("{base} - {code} - {t}"),
            None => format!("{base} - {code}"),
        };
        let dir = Path::new(&e.path).parent().map(Path::to_path_buf).unwrap_or_default();
        groups.entry((dir, new_stem)).or_default().push(e);
    }
    let mut moves: Vec<Move> = Vec::new();
    for ((_, new_stem), mut files) in groups {
        files.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        // Two files for one episode that only a counter could tell apart are
        // more likely a numbering mix-up than two copies; do not guess.
        if paths.len() > 1 && !told_apart(&paths) {
            return Err(format!(
                "{} would all become {new_stem}. Nothing in this series was renamed; check their numbering.",
                files.iter().map(|f| f.file_name.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        for (f, s) in files.iter().zip(stems_for(&paths, &new_stem)) {
            push_file(&mut moves, Path::new(&f.path), &s);
        }
    }
    moves.sort_by(|a, b| a.from.to_lowercase().cmp(&b.from.to_lowercase()));

    // Season folders holding one season become "Season 01" ("Specials" for
    // season 0). Two folders that would get the same name both keep theirs.
    let mut seasons_in: HashMap<PathBuf, HashSet<i32>> = HashMap::new();
    for &(e, s, _, _) in &placed {
        let dir = Path::new(&e.path).parent().map(Path::to_path_buf).unwrap_or_default();
        seasons_in.entry(dir).or_default().insert(s);
    }
    let mut season_moves: Vec<Move> = Vec::new();
    for (dir, seasons) in &seasons_in {
        if seasons.len() != 1 || !parser::is_season_dir(&file_name(dir)) {
            continue;
        }
        // Only inside the show's own folder. "The.Bear.S03.1080p" sitting
        // directly in "TV" is the only folder carrying the show's name;
        // renaming it to "Season 03" would lose that.
        if !dir.parent().map(|p| named_after_title(p, item, info)).unwrap_or(false) {
            continue;
        }
        let s = seasons.iter().next().copied().unwrap_or(0);
        let name = if s == 0 { "Specials".to_string() } else { format!("Season {s:02}") };
        if file_name(dir) == name || !folder_is_own(app, dir, item.id, roots)? {
            continue;
        }
        let new_dir = dir.with_file_name(&name);
        if new_dir.exists() && !same_path(&new_dir, dir) {
            continue;
        }
        season_moves.push(Move { from: dir.to_string_lossy().into(), to: new_dir.to_string_lossy().into(), folder: true });
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for m in &season_moves {
        *seen.entry(m.to.to_lowercase()).or_default() += 1;
    }
    season_moves.retain(|m| seen.get(&m.to.to_lowercase()) == Some(&1));
    season_moves.sort_by(|a, b| a.from.cmp(&b.from));
    moves.extend(season_moves);

    // The show's own folder: the one above the season folders (or holding the
    // episodes directly), when every episode is under that one folder.
    let show_dirs: HashSet<PathBuf> = placed
        .iter()
        .filter_map(|(e, ..)| {
            let parent = Path::new(&e.path).parent()?;
            if parser::is_season_dir(&file_name(parent)) {
                parent.parent().map(Path::to_path_buf)
            } else {
                Some(parent.to_path_buf())
            }
        })
        .collect();
    if show_dirs.len() == 1 {
        let dir = show_dirs.into_iter().next().unwrap_or_default();
        if let Some(m) = own_folder_move(app, &dir, item, info, roots, base)? {
            moves.push(m);
        }
    }
    Ok(moves)
}

/// The new name must read back as this same title on the next scan. If it
/// reads as another kind, another year, or lands on a different title already
/// in the library ("Gojira" renamed to "Godzilla (1954)" beside "Godzilla
/// (2014)"), the scan would merge two films; refuse instead.
fn reads_back(app: &AppHandle, item: &db::MediaItem, info: &TmdbInfo, roots: &[String], moves: &[Move]) -> Result<(), String> {
    let Some(video) = moves.iter().find(|m| !m.folder && parser::is_video(Path::new(&m.to))) else { return Ok(()) };
    let path = final_path(&video.to, moves);
    let lower = path.to_lowercase();
    let root = roots
        .iter()
        .filter(|r| lower.starts_with(&format!("{}{}", r.trim_end_matches(['\\', '/']).to_lowercase(), std::path::MAIN_SEPARATOR)))
        .max_by_key(|r| r.len());
    let Some(root) = root else { return Ok(()) };
    let parsed = parser::parse(Path::new(&path), Path::new(root));
    let kind_ok = matches!((item.kind.as_str(), parsed.kind), ("movie", parser::Kind::Movie) | ("series", parser::Kind::Series));
    let year_ok = item.kind != "movie" || info.year.is_none() || parsed.year == info.year;
    if !kind_ok || !year_ok || parsed.title.is_empty() {
        return Err(format!("The new name would read back as \"{}\"{}, not as this title.", parsed.title, parsed.year.map(|y| format!(" ({y})")).unwrap_or_default()));
    }
    let key = parser::sort_key(&parsed.title);
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let other: Option<(String, Option<i32>)> = conn
        .query_row(
            "SELECT title, year FROM media_items WHERE kind = ?1 AND sort_key = ?2 AND id != ?3",
            params![item.kind, key, item.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((title, year)) = other {
        return Err(format!(
            "After renaming it would be filed together with \"{title}\"{} already in your library. Rename that one first, or check the match.",
            year.map(|y| format!(" ({y})")).unwrap_or_default()
        ));
    }
    Ok(())
}

/// "Inception" and "Inception 2010 1080p" name the same film; "Movies" does not.
fn names_agree(a: &str, b: &str) -> bool {
    let (ka, kb) = (parser::sort_key(a), parser::sort_key(b));
    !ka.is_empty() && !kb.is_empty() && (ka.starts_with(&kb) || kb.starts_with(&ka))
}

/// Where a file ends up once the folder rename at the end has happened too.
fn final_path(path: &str, moves: &[Move]) -> String {
    let mut p = path.to_string();
    for m in moves {
        let prefix = format!("{}{}", m.from, std::path::MAIN_SEPARATOR);
        if let Some(rest) = p.strip_prefix(&prefix) {
            p = format!("{}{}{rest}", m.to, std::path::MAIN_SEPARATOR);
        }
    }
    p
}

/// A folder may be renamed only when nothing else lives in it: it is not a
/// library itself, and no other title has files under it.
fn folder_is_own(app: &AppHandle, dir: &Path, media_item_id: i64, roots: &[String]) -> Result<bool, String> {
    // Not a library, and not a folder a library sits in: renaming it would
    // leave that library pointing at a folder that is gone.
    let dir_key = format!("{}{}", dir.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase(), std::path::MAIN_SEPARATOR);
    let holds_library = roots.iter().any(|r| {
        let root_key = format!("{}{}", r.trim_end_matches(['\\', '/']).to_lowercase(), std::path::MAIN_SEPARATOR);
        root_key.starts_with(&dir_key)
    });
    if dir.parent().is_none() || holds_library {
        return Ok(false);
    }
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let prefix = format!("{}{}", dir.to_string_lossy(), std::path::MAIN_SEPARATOR);
    let others: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE substr(lower(path), 1, length(?1)) = lower(?1) AND media_item_id != ?2",
            params![prefix, media_item_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(others == 0)
}

/// Plans for the given titles, or every movie and series when `ids` is None.
pub fn preview(app: &AppHandle, ids: Option<Vec<i64>>) -> Result<Vec<Plan>, String> {
    let ids = match ids {
        Some(ids) => ids,
        None => {
            let state = app.state::<AppState>();
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            db::list_media(&conn, None).map_err(|e| e.to_string())?.into_iter().map(|m| m.id).collect()
        }
    };
    Ok(ids.into_iter().map(|id| plan_item(app, id)).collect())
}

/// Plan again (the disk may have changed since the preview) and carry out
/// the plans for these titles. Runs while holding the scan job, so no scan
/// can read the library half renamed.
pub fn apply(app: &AppHandle, ids: Vec<i64>) -> Result<Report, String> {
    jobs::exclusive(app, || {
        let mut report = Report::default();
        let mut batch: Vec<Move> = Vec::new();
        for id in ids {
            let plan = plan_item(app, id);
            if let Some(why) = plan.skipped {
                report.failed.push((plan.title, why));
                continue;
            }
            if plan.moves.is_empty() {
                report.unchanged += 1;
                continue;
            }
            match carry_out(app, &plan.moves, Some((plan.media_item_id, plan.target_title.as_str()))) {
                Ok(()) => {
                    tracing::info!(title = %plan.title, target = plan.target.as_deref().unwrap_or(""), moves = plan.moves.len(), "renamed");
                    report.renamed += 1;
                    batch.extend(plan.moves);
                }
                Err(e) => {
                    tracing::warn!(title = %plan.title, "rename failed, nothing changed: {e}");
                    report.failed.push((plan.title, e));
                }
            }
        }
        if !batch.is_empty() {
            let mut history = load_history(app);
            history.batches.push(batch);
            let excess = history.batches.len().saturating_sub(HISTORY_KEEP);
            history.batches.drain(..excess);
            save_history(app, &history)?;
        }
        Ok(report)
    })
}

/// Reverse the most recent applied batch.
pub fn undo(app: &AppHandle) -> Result<usize, String> {
    jobs::exclusive(app, || {
        let mut history = load_history(app);
        let batch = history.batches.pop().ok_or("Nothing to undo.")?;
        let reversed: Vec<Move> =
            batch.iter().rev().map(|m| Move { from: m.to.clone(), to: m.from.clone(), folder: m.folder }).collect();
        for m in &reversed {
            if Path::new(&m.to).exists() && !same_path(Path::new(&m.from), Path::new(&m.to)) {
                return Err(format!("Cannot undo: {} exists again.", m.to));
            }
        }
        carry_out(app, &reversed, None).map_err(|e| format!("Cannot undo: {e}"))?;
        save_history(app, &history)?;
        tracing::info!(moves = reversed.len(), "rename undone");
        Ok(reversed.len())
    })
}

pub fn can_undo(app: &AppHandle) -> bool {
    !load_history(app).batches.is_empty()
}

/// Rename on disk in order, putting everything back if one step fails, then
/// bring the library's paths and titles up to date.
/// `display`: the title to show for this item afterwards (TMDb's), when the
/// rename was made from it.
fn carry_out(app: &AppHandle, moves: &[Move], display: Option<(i64, &str)>) -> Result<(), String> {
    rename_on_disk(moves)?;
    if let Err(e) = update_library(app, moves, display) {
        // The files moved but the library could not follow: put them back,
        // or the next scan would see new names and split the title.
        let back: Vec<Move> = moves.iter().rev().map(|m| Move { from: m.to.clone(), to: m.from.clone(), folder: m.folder }).collect();
        return match rename_on_disk(&back) {
            Ok(()) => Err(format!("the library could not be updated ({e}); the files were put back")),
            Err(undo) => Err(format!("the library could not be updated ({e}), and putting the files back failed: {undo}")),
        };
    }
    Ok(())
}

fn update_library(app: &AppHandle, moves: &[Move], display: Option<(i64, &str)>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut items: HashSet<i64> = HashSet::new();
    for m in moves {
        update_paths(&tx, m, &mut items).map_err(|e| e.to_string())?;
    }
    // Read the title from a file that was renamed: a remake left as it was
    // still carries the old name, and would key the title wrongly.
    let renamed = moves.iter().filter(|m| !m.folder && parser::is_video(Path::new(&m.to))).map(|m| final_path(&m.to, moves)).next();
    for id in items {
        let shown = display.filter(|(item, _)| *item == id).map(|(_, t)| t);
        resync_title(&tx, id, shown, renamed.as_deref()).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

/// All or nothing: a failed step (the file is open in a player, a name was
/// taken meanwhile) puts back every step before it.
fn rename_on_disk(moves: &[Move]) -> Result<(), String> {
    let mut done: Vec<&Move> = Vec::new();
    for m in moves {
        let (from, to) = (Path::new(&m.from), Path::new(&m.to));
        // Checked again right before the move: Windows' rename replaces an
        // existing file without asking.
        let blocked = to.exists() && !same_path(from, to);
        let result = if blocked { Err(format!("{} already exists", m.to)) } else { std::fs::rename(from, to).map_err(|e| e.to_string()) };
        if let Err(e) = result {
            let stuck: Vec<String> = done.iter().rev().filter(|d| std::fs::rename(&d.to, &d.from).is_err()).map(|d| d.to.clone()).collect();
            if stuck.is_empty() {
                return Err(format!("{}: {e}", file_name(from)));
            }
            return Err(format!("{}: {e}; could not put back: {}", file_name(from), stuck.join(", ")));
        }
        done.push(m);
    }
    Ok(())
}

/// Point the library at a file or folder's new path, subtitles included.
fn update_paths(conn: &Connection, m: &Move, items: &mut HashSet<i64>) -> rusqlite::Result<()> {
    let sep = std::path::MAIN_SEPARATOR;
    let file: Option<i64> = conn.query_row("SELECT media_item_id FROM episodes WHERE path = ?1", [&m.from], |r| r.get(0)).optional()?;
    if let Some(item) = file {
        conn.execute(
            "UPDATE episodes SET path = ?2, file_name = ?3 WHERE path = ?1",
            params![m.from, m.to, file_name(Path::new(&m.to))],
        )?;
        items.insert(item);
    } else {
        let (old, new) = (format!("{}{sep}", m.from), format!("{}{sep}", m.to));
        let mut stmt = conn.prepare("SELECT DISTINCT media_item_id FROM episodes WHERE substr(path, 1, length(?1)) = ?1")?;
        items.extend(stmt.query_map([&old], |r| r.get::<_, i64>(0))?.collect::<Result<Vec<_>, _>>()?);
        conn.execute(
            "UPDATE episodes SET path = ?2 || substr(path, length(?1) + 1) WHERE substr(path, 1, length(?1)) = ?1",
            params![old, new],
        )?;
        conn.execute("UPDATE episodes SET subtitles = replace(subtitles, ?1, ?2) WHERE instr(subtitles, ?1) > 0", params![old, new])?;
    }
    // A sidecar subtitle renamed on its own.
    conn.execute("UPDATE episodes SET subtitles = replace(subtitles, ?1, ?2) WHERE instr(subtitles, ?1) > 0", params![m.from, m.to])?;
    Ok(())
}

/// Keep the title keyed the way the next scan will read its new file name.
/// Titles are grouped by the name parsed from their files, so without this a
/// renamed film came back as a new, unmatched title, losing its poster, tags
/// and collections.
fn resync_title(conn: &Connection, media_item_id: i64, display: Option<&str>, prefer: Option<&str>) -> rusqlite::Result<()> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT e.path, l.path, m.kind FROM episodes e JOIN libraries l ON l.id = e.library_id
             JOIN media_items m ON m.id = e.media_item_id
             WHERE e.media_item_id = ?1 AND e.extra IS NULL
             ORDER BY e.path = ?2 DESC, e.path LIMIT 1",
            params![media_item_id, prefer.unwrap_or_default()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((path, root, kind)) = row else { return Ok(()) };
    let parsed = parser::parse(Path::new(&path), Path::new(&root));
    let same_kind = matches!((kind.as_str(), parsed.kind), ("movie", parser::Kind::Movie) | ("series", parser::Kind::Series));
    if !same_kind || parsed.title.is_empty() {
        return Ok(());
    }
    let key = parser::sort_key(&parsed.title);
    // TMDb's own spelling ("Mission: Impossible") when it keys the same as the
    // file name it became; the year is left as it was.
    let title = display.filter(|d| parser::sort_key(d) == key).unwrap_or(&parsed.title);
    // What TMDb said is remembered under the title's key; carry it over so a
    // folder switched back in later still finds it under the new name.
    let old_key: String = conn.query_row("SELECT sort_key FROM media_items WHERE id = ?1", [media_item_id], |r| r.get(0))?;
    // OR IGNORE: another title already holding that key means the next scan
    // merges the two (the plan refuses that case for different films).
    let changed = conn.execute(
        "UPDATE OR IGNORE media_items SET title = ?2, sort_key = ?3 WHERE id = ?1",
        params![media_item_id, title, key],
    )?;
    if changed == 1 {
        db::rekey_memory(conn, &kind, &old_key, &key)?;
    }
    Ok(())
}

fn history_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join(HISTORY_FILE))
}

fn load_history(app: &AppHandle) -> History {
    history_path(app)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_history(app: &AppHandle, h: &History) -> Result<(), String> {
    let path = history_path(app).ok_or("no app data folder")?;
    let json = serde_json::to_vec_pretty(h).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("could not record the rename for undo: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_windows_accepts() {
        assert_eq!(safe_name("Mission: Impossible - Dead Reckoning"), "Mission - Impossible - Dead Reckoning");
        assert_eq!(safe_name("What If...?"), "What If");
        assert_eq!(safe_name("Face/Off"), "FaceOff");
        assert_eq!(safe_name("CON"), "CON_");
        assert_eq!(base_name("Se7en", Some(1995)), "Se7en (1995)");
        assert_eq!(base_name("Untitled", None), "Untitled");
    }

    #[test]
    fn episodes_are_placed_on_tmdb_seasons() {
        let seasons: BTreeMap<i32, i32> = [(0, 5), (1, 13), (2, 13)].into_iter().collect();
        assert_eq!(place(&seasons, 1, 5, true, false), Some((1, 5)), "numbers TMDb has are kept");
        assert_eq!(place(&seasons, 1, 16, false, false), Some((2, 3)), "anime counted straight through");
        assert_eq!(place(&seasons, 1, 16, true, false), None, "S01E16 means what it says");
        assert_eq!(place(&seasons, 2, 26, false, true), Some((2, 13)), "straight-through number inside its own season folder");
        assert_eq!(place(&seasons, 1, 20, false, true), None, "the count must land in the folder's season");
        assert_eq!(place(&seasons, 1, 40, false, false), None, "past TMDb's last episode");
        assert_eq!(place(&seasons, 0, 3, true, false), Some((0, 3)), "specials");
        assert_eq!(place(&BTreeMap::new(), 4, 9, true, false), Some((4, 9)), "no season list to check against");
    }

    #[test]
    fn a_series_keeps_its_place_in_the_library_after_renaming() {
        let path = std::env::temp_dir().join(format!("vortex-rename-tv-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = db::open(&path).unwrap();
        let lib = db::add_library(&conn, r"F:\Anime").unwrap();
        conn.execute("INSERT INTO media_items (id, kind, title, sort_key) VALUES (1, 'series', 'Shingeki no Kyojin', 'shingekinokyojin')", []).unwrap();
        conn.execute(
            "INSERT INTO episodes (id, media_item_id, library_id, path, file_name, season, episode, size, modified)
             VALUES (1, 1, ?1, ?2, '[Group] Shingeki no Kyojin - 01 [1080p].mkv', 1, 1, 1, 1)",
            params![lib.id, r"F:\Anime\Shingeki no Kyojin\S1\[Group] Shingeki no Kyojin - 01 [1080p].mkv"],
        )
        .unwrap();
        let moves = [
            Move {
                from: r"F:\Anime\Shingeki no Kyojin\S1\[Group] Shingeki no Kyojin - 01 [1080p].mkv".into(),
                to: r"F:\Anime\Shingeki no Kyojin\S1\Attack on Titan (2013) - S01E01 - To You, in 2000 Years.mkv".into(),
                folder: false,
            },
            Move { from: r"F:\Anime\Shingeki no Kyojin\S1".into(), to: r"F:\Anime\Shingeki no Kyojin\Season 01".into(), folder: true },
            Move { from: r"F:\Anime\Shingeki no Kyojin".into(), to: r"F:\Anime\Attack on Titan (2013)".into(), folder: true },
        ];
        let mut items = HashSet::new();
        for m in &moves {
            update_paths(&conn, m, &mut items).unwrap();
        }
        let renamed = final_path(&moves[0].to, &moves);
        assert_eq!(renamed, r"F:\Anime\Attack on Titan (2013)\Season 01\Attack on Titan (2013) - S01E01 - To You, in 2000 Years.mkv");
        resync_title(&conn, 1, Some("Attack on Titan"), Some(&renamed)).unwrap();
        let p: String = conn.query_row("SELECT path FROM episodes WHERE id = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(p, renamed);
        let (title, key): (String, String) = conn.query_row("SELECT title, sort_key FROM media_items WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((title.as_str(), key.as_str()), ("Attack on Titan", "attackontitan"), "keyed the way the next scan reads the new names");
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn only_a_folder_named_after_the_film_is_renamed() {
        let title_of = |folder: &str| parser::folder_title_of(folder).0;
        assert!(names_agree(&title_of("Inception.2010.1080p.BluRay.x264"), "Inception"));
        assert!(names_agree(&title_of("Inception (2010)"), "Inception"));
        assert!(!names_agree(&title_of("Movies"), "Inception"));
        assert!(!names_agree(&title_of("Hollywood Classic"), "Casablanca"));
    }

    #[test]
    fn several_files_in_one_folder_get_told_apart() {
        let base = "Kill Bill (2003)";
        assert_eq!(
            stems_for(&[r"D:\M\kill.bill.cd1.avi", r"D:\M\kill.bill.cd2.avi"], base),
            vec!["Kill Bill (2003) - pt1", "Kill Bill (2003) - pt2"]
        );
        assert_eq!(
            stems_for(&[r"D:\M\Kill.Bill.2003.1080p.mkv", r"D:\M\Kill.Bill.2003.2160p.mkv"], base),
            vec!["Kill Bill (2003) - 1080p", "Kill Bill (2003) - 2160p"]
        );
        assert_eq!(stems_for(&[r"D:\M\a.mkv", r"D:\M\b.mkv"], base), vec!["Kill Bill (2003) - 1", "Kill Bill (2003) - 2"]);
    }

    #[test]
    fn sidecars_follow_their_video() {
        let dir = std::env::temp_dir().join(format!("vortex-rename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in [
            "Movie.mkv", "Movie.en.srt", "Movie_eng.srt", "Movie.nfo", "Movie.Extended.mkv", "Movie.Extended.srt", "Movies.txt", "Other.srt",
            "It.mkv", "It.Follows.mkv", "It.Follows.en.srt", "Italian.Job.srt",
        ] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let found = |video: &str| {
            let mut v: Vec<String> = sidecars(&dir.join(video)).into_iter().map(|(_, rest)| rest).collect();
            v.sort();
            v
        };
        assert_eq!(found("Movie.mkv"), vec![".en.srt", ".nfo", "_eng.srt"], "the Extended cut's subtitle stays with the Extended cut; Movies.txt is another name");
        assert!(found("It.mkv").is_empty(), "It.Follows' subtitle and Italian.Job.srt are not It's");
        // Lowercasing "İ" adds a byte; the rest of the name must still be cut at the right place.
        assert_eq!(named_after("İstanbul.en.srt", "İstanbul").as_deref(), Some(".en.srt"));
        assert_eq!(named_after("istanbul.en.srt", "İstanbul"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_step_puts_everything_back() {
        let dir = std::env::temp_dir().join(format!("vortex-rename-disk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Old Folder")).unwrap();
        for f in ["Old Folder/movie.mkv", "Old Folder/movie.srt", "Old Folder/Taken.mkv"] {
            std::fs::write(dir.join(f), b"x").unwrap();
        }
        let p = |rel: &str| dir.join(rel).to_string_lossy().to_string();
        let mv = |from: &str, to: &str, folder: bool| Move { from: p(from), to: p(to), folder };

        // The second step aims at a name that is taken: the first is undone.
        let moves = [mv("Old Folder/movie.srt", "Old Folder/Taken.srt", false), mv("Old Folder/movie.mkv", "Old Folder/Taken.mkv", false)];
        assert!(rename_on_disk(&moves).is_err());
        assert!(dir.join("Old Folder/movie.srt").exists() && dir.join("Old Folder/movie.mkv").exists());
        assert!(!dir.join("Old Folder/Taken.srt").exists());

        // A change of case alone is a rename, not a collision.
        let moves = [mv("Old Folder/movie.mkv", "Old Folder/Movie.mkv", false), mv("Old Folder", "New Folder", true)];
        rename_on_disk(&moves).unwrap();
        let names: Vec<String> = std::fs::read_dir(dir.join("New Folder")).unwrap().flatten().map(|e| file_name(&e.path())).collect();
        assert!(names.contains(&"Movie.mkv".to_string()), "got {names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn library_paths_follow_a_folder_rename() {
        let path = std::env::temp_dir().join(format!("vortex-rename-db-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let conn = db::open(&path).unwrap();
        let lib = db::add_library(&conn, r"F:\Movies").unwrap();
        conn.execute("INSERT INTO media_items (id, kind, title, sort_key) VALUES (1, 'movie', 'Inception', 'inception')", []).unwrap();
        conn.execute(
            "INSERT INTO episodes (id, media_item_id, library_id, path, file_name, size, modified, subtitles)
             VALUES (1, 1, ?1, ?2, 'Inception.2010.1080p.mkv', 1, 1, ?3)",
            params![lib.id, r"F:\Movies\Inception.2010.1080p\Inception.2010.1080p.mkv", r"F:\Movies\Inception.2010.1080p\Inception.2010.1080p.en.srt"],
        )
        .unwrap();
        let moves = [
            Move { from: r"F:\Movies\Inception.2010.1080p\Inception.2010.1080p.en.srt".into(), to: r"F:\Movies\Inception.2010.1080p\Inception (2010).en.srt".into(), folder: false },
            Move { from: r"F:\Movies\Inception.2010.1080p\Inception.2010.1080p.mkv".into(), to: r"F:\Movies\Inception.2010.1080p\Inception (2010).mkv".into(), folder: false },
            Move { from: r"F:\Movies\Inception.2010.1080p".into(), to: r"F:\Movies\Inception (2010)".into(), folder: true },
        ];
        let mut items = HashSet::new();
        for m in &moves {
            update_paths(&conn, m, &mut items).unwrap();
        }
        for id in &items {
            resync_title(&conn, *id, Some("Inception"), None).unwrap();
        }
        let (p, f, subs): (String, String, String) =
            conn.query_row("SELECT path, file_name, subtitles FROM episodes WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        assert_eq!(p, r"F:\Movies\Inception (2010)\Inception (2010).mkv");
        assert_eq!(f, "Inception (2010).mkv");
        assert_eq!(subs, r"F:\Movies\Inception (2010)\Inception (2010).en.srt");
        let (title, key): (String, String) = conn.query_row("SELECT title, sort_key FROM media_items WHERE id = 1", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        assert_eq!((title.as_str(), key.as_str()), ("Inception", "inception"));
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }
}
