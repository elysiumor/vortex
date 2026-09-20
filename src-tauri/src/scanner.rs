use crate::db::{self, Library, NewEpisode};
use crate::parser::{self, sort_key, Kind, Parsed};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[derive(Serialize, Debug, Default, Clone)]
pub struct ScanStats {
    pub libraries_scanned: usize,
    pub libraries_skipped: Vec<String>,
    pub files_seen: usize,
    pub added: usize,
    pub removed: usize,
    pub renamed: usize,
    pub extras: usize,
    /// Human labels of newly added episodes and movies, first few only.
    pub added_titles: Vec<String>,
}

const SUB_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "sub", "vtt", "idx", "sup"];

/// Subtitle files beside `video` whose name starts with the video's stem:
/// "Movie.srt", "Movie.en.srt", "Movie.eng.forced.ass".
fn sidecar_subtitles(video: &Path, dir_cache: &mut std::collections::HashMap<PathBuf, Vec<PathBuf>>) -> Vec<PathBuf> {
    let Some(dir) = video.parent() else { return Vec::new() };
    let stem = video.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    if stem.is_empty() {
        return Vec::new();
    }
    let entries = dir_cache.entry(dir.to_path_buf()).or_insert_with(|| {
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| SUB_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    });
    entries
        .iter()
        .filter(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with(&stem)).unwrap_or(false))
        .cloned()
        .collect()
}

impl ScanStats {
    pub fn changed(&self) -> bool {
        self.added > 0 || self.removed > 0 || self.renamed > 0
    }
}

const MIN_FILE_BYTES: u64 = 20 * 1024 * 1024; // skip tiny clips and samples

/// Folders never worth crawling, so a whole drive can be added as a library.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    "windows", "program files", "program files (x86)", "programdata", "appdata", "$recycle.bin",
    "system volume information", "recovery", "perflogs", "node_modules", ".git", "$windows.~bt",
    "windows.old", "msocache", "steamapps", "cache", ".cache", "temp", "tmp", ".incomplete",
];

pub fn ignored_dirs(conn: &Connection) -> Vec<String> {
    let mut list: Vec<String> = DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect();
    if let Ok(Some(extra)) = db::get_setting(conn, "ignore_dirs") {
        list.extend(extra.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()));
    }
    list
}

#[cfg(windows)]
fn is_hidden_or_system(entry: &walkdir::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    entry.metadata().map(|m| m.file_attributes() & (HIDDEN | SYSTEM) != 0).unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden_or_system(entry: &walkdir::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

pub fn scan_all(conn: &mut Connection) -> Result<ScanStats, String> {
    let libs = db::list_libraries(conn).map_err(|e| e.to_string())?;
    let ignore = ignored_dirs(conn);
    let mut stats = ScanStats::default();
    for lib in libs {
        if !lib.available {
            stats.libraries_skipped.push(lib.path.clone());
            continue;
        }
        scan_library(conn, &lib, &ignore, &mut stats)?;
        stats.libraries_scanned += 1;
    }
    db::prune_empty_items(conn).map_err(|e| e.to_string())?;
    Ok(stats)
}

/// Category = the first folder under the library root, unless that folder is
/// the title's own folder or a season folder, in which case the library name
/// is used. `Root/Anime/Show/ep.mkv` -> "Anime"; `Root/Show/Season 1/ep.mkv` -> library name.
pub fn category_for(path: &Path, root: &Path, library_name: &str, title: &str) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let dirs: Vec<String> = rel
        .parent()?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let first = match dirs.first() {
        Some(f) => f,
        None => return Some(library_name.to_string()),
    };
    let title_key = sort_key(title);
    let first_key = sort_key(&parser::clean_title(first));
    let is_own_folder = (!title_key.is_empty() && first_key.starts_with(&title_key)) || parser::is_season_dir(first);
    if is_own_folder {
        Some(library_name.to_string())
    } else {
        Some(first.clone())
    }
}

struct Seen {
    path: PathBuf,
    size: u64,
    modified: i64,
    parsed: Parsed,
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

/// Ancestor folders of `path` strictly inside `root`, nearest first.
fn ancestors_inside(path: &Path, root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut cur = path.parent();
    while let Some(p) = cur {
        if p == root || !p.starts_with(root) {
            break;
        }
        out.push(p.to_path_buf());
        cur = p.parent();
    }
    out
}

/// Label for an extra: the folders between the owner and the file, with
/// season folders shortened. "Season 1 › Featurettes › Behind The Scenes".
fn extra_label(owner: &Path, path: &Path) -> String {
    let rel = path.parent().and_then(|p| p.strip_prefix(owner).ok());
    let parts: Vec<String> = rel
        .map(|r| {
            r.components()
                .map(|c| {
                    let n = c.as_os_str().to_string_lossy().to_string();
                    match parser::season_from_dir(&n) {
                        Some(s) => format!("Season {s}"),
                        None => n,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        "Extras".to_string()
    } else {
        parts.join(" › ")
    }
}

/// The series that has real episodes under `folder`, if any.
fn series_under(conn: &Connection, folder: &Path) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT e.media_item_id, e.path FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         WHERE m.kind = 'series' AND e.extra IS NULL AND e.path LIKE ?1 LIMIT 50",
    )?;
    let like = format!("{}%", folder.to_string_lossy());
    let rows: Vec<(i64, String)> = stmt.query_map([&like], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    Ok(rows.into_iter().find(|(_, p)| Path::new(p).starts_with(folder)).map(|(id, _)| id))
}

/// The one series with episodes under `folder`, or None if there are several or none.
fn sole_series_under(conn: &Connection, folder: &Path) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT e.media_item_id FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         WHERE m.kind = 'series' AND e.extra IS NULL AND e.path LIKE ?1 LIMIT 2",
    )?;
    let like = format!("{}%", folder.to_string_lossy());
    let ids: Vec<i64> = stmt.query_map([&like], |r| r.get(0))?.collect::<Result<_, _>>()?;
    Ok(if ids.len() == 1 { Some(ids[0]) } else { None })
}

/// The movie whose main file sits directly in `folder`, if any.
fn movie_in(conn: &Connection, folder: &Path) -> rusqlite::Result<Option<i64>> {
    let mut stmt = conn.prepare(
        "SELECT e.media_item_id, e.path FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         WHERE m.kind = 'movie' AND e.extra IS NULL AND e.path LIKE ?1 LIMIT 50",
    )?;
    let like = format!("{}%", folder.to_string_lossy());
    let rows: Vec<(i64, String)> = stmt.query_map([&like], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    Ok(rows.into_iter().find(|(_, p)| Path::new(p).parent() == Some(folder)).map(|(id, _)| id))
}

fn scan_library(conn: &mut Connection, lib: &Library, ignore: &[String], stats: &mut ScanStats) -> Result<(), String> {
    let root = Path::new(&lib.path);
    let mut seen_paths: HashSet<String> = HashSet::new();
    // (size, modified) -> new episode id, for matching renamed files afterwards.
    let mut inserted: Vec<(i64, i64, i64)> = Vec::new();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let before = db::episode_fingerprints_for_library(&tx, lib.id).map_err(|e| e.to_string())?;

    // Pass 1: collect files. Real episodes are written immediately; everything
    // else waits until we know which folders hold series and movies.
    let mut episodes: Vec<Seen> = Vec::new();
    let mut others: Vec<Seen> = Vec::new();
    let walker = WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| {
        if e.depth() == 0 || !e.file_type().is_dir() {
            return true;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        !ignore.iter().any(|i| i == &name) && !is_hidden_or_system(e)
    });
    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() || !parser::is_video(path) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name_lower = file_name(path).to_lowercase();
        if meta.len() < MIN_FILE_BYTES || name_lower.contains("sample") {
            continue;
        }
        stats.files_seen += 1;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let parsed = parser::parse(path, root);
        let seen = Seen { path: path.to_path_buf(), size: meta.len(), modified, parsed };
        if seen.parsed.kind == Kind::Series && seen.parsed.episode.is_some() && !seen.parsed.extra {
            episodes.push(seen);
        } else {
            others.push(seen);
        }
    }

    let mut sub_cache: std::collections::HashMap<PathBuf, Vec<PathBuf>> = std::collections::HashMap::new();
    let mut write = |tx: &Connection, s: &Seen, kind: &str, title: &str, year: Option<i32>, item_id: Option<i64>,
                     season: Option<i32>, episode: Option<i32>, extra: Option<&str>| -> Result<(), String> {
        let subs = sidecar_subtitles(&s.path, &mut sub_cache);
        let subs_str = (!subs.is_empty()).then(|| subs.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>().join("|"));
        let item_id = match item_id {
            Some(id) => id,
            None => {
                let category = category_for(&s.path, root, &lib.name, title);
                db::upsert_media_item(tx, kind, title, year, &sort_key(title), category.as_deref()).map_err(|e| e.to_string())?
            }
        };
        let path_str = s.path.to_string_lossy().to_string();
        let fname = file_name(&s.path);
        let inserted_now = db::upsert_episode(
            tx,
            &NewEpisode {
                media_item_id: item_id,
                library_id: lib.id,
                path: &path_str,
                file_name: &fname,
                season,
                episode,
                size: s.size as i64,
                modified: s.modified,
                extra,
                subtitles: subs_str.as_deref(),
            },
        )
        .map_err(|e| e.to_string())?;
        if inserted_now {
            let new_id: i64 =
                tx.query_row("SELECT id FROM episodes WHERE path = ?1", [&path_str], |r| r.get(0)).map_err(|e| e.to_string())?;
            inserted.push((new_id, s.size as i64, s.modified));
            stats.added += 1;
            if extra.is_none() && stats.added_titles.len() < 6 {
                let label = match (season, episode) {
                    (Some(se), Some(ep)) => format!("{title} S{se:02}E{ep:02}"),
                    _ => title.to_string(),
                };
                stats.added_titles.push(label);
            }
        }
        if extra.is_some() {
            stats.extras += 1;
        }
        seen_paths.insert(path_str);
        Ok(())
    };

    for s in &episodes {
        let p = &s.parsed;
        write(&tx, s, "series", &p.title, p.year, None, p.season, p.episode, None)?;
    }

    // Pass 2: plain movies first (so featurettes can find them), then the rest.
    others.sort_by_key(|s| s.parsed.extra || s.parsed.kind == Kind::Series);
    for s in &others {
        let p = &s.parsed;
        let in_season = s.path.parent().map(|d| parser::is_season_dir(&file_name(d))).unwrap_or(false);
        let bonus = p.extra || (p.kind == Kind::Series && p.episode.is_none()) || in_season;

        // Bonus material under a folder that already holds real episodes.
        let mut attached = false;
        let ancestors = ancestors_inside(&s.path, root);
        if bonus {
            for (i, dir) in ancestors.iter().enumerate() {
                if let Some(series_id) = series_under(&tx, dir).map_err(|e| e.to_string())? {
                    // Label relative to the series' top folder, so season folders
                    // show up in it ("Season 1 › Featurettes").
                    let mut owner = dir.clone();
                    for higher in &ancestors[i + 1..] {
                        match series_under(&tx, higher).map_err(|e| e.to_string())? {
                            Some(id) if id == series_id => owner = higher.clone(),
                            _ => break,
                        }
                    }
                    let label = extra_label(&owner, &s.path);
                    write(&tx, s, "series", &p.title, p.year, Some(series_id), None, None, Some(&label))?;
                    attached = true;
                    break;
                }
            }
        }
        if attached {
            continue;
        }
        if bonus {
            // The folder that owns this bonus material: the first ancestor that
            // is neither an extras folder nor a season folder. The library root
            // itself counts, for people who add a show's folder as a library.
            let owner = ancestors
                .iter()
                .find(|d| !parser::is_extras_dir(&file_name(d)) && !parser::is_season_dir(&file_name(d)))
                .cloned()
                .unwrap_or_else(|| root.to_path_buf());
            let label = extra_label(&owner, &s.path);
            if let Some(movie_id) = movie_in(&tx, &owner).map_err(|e| e.to_string())? {
                write(&tx, s, "movie", &p.title, p.year, Some(movie_id), None, None, Some(&label))?;
                continue;
            }
            if owner == root {
                // Library root is the show's folder: if exactly one series lives
                // here, this belongs to it (e.g. season 2 has only extras on disk).
                if let Some(series_id) = sole_series_under(&tx, root).map_err(|e| e.to_string())? {
                    write(&tx, s, "series", &p.title, p.year, Some(series_id), None, None, Some(&label))?;
                    continue;
                }
            }
            // No episodes or film found yet (e.g. only extras of a season on
            // disk): file it under a series named after the owner folder, which
            // merges with the real series when its episodes arrive.
            let (title, year) = parser::folder_title_of(&file_name(&owner));
            let (title, year) = if title.is_empty() { (p.title.clone(), p.year) } else { (title, year) };
            write(&tx, s, "series", &title, year, None, None, None, Some(&label))?;
            continue;
        }
        write(&tx, s, "movie", &p.title, p.year, None, None, None, None)?;
    }

    // Rows whose file is gone. If a new file has the same size and modified
    // time it was renamed or moved: carry the progress over instead of losing it.
    let mut gone: Vec<i64> = Vec::new();
    for (old_id, old_path, size, modified) in before {
        if seen_paths.contains(&old_path) {
            continue;
        }
        if let Some(idx) = inserted.iter().position(|(_, s, m)| *s == size && *m == modified) {
            let (new_id, _, _) = inserted.remove(idx);
            db::move_progress(&tx, old_id, new_id).map_err(|e| e.to_string())?;
            stats.renamed += 1;
            stats.added -= 1;
            // A rename is not a new arrival; drop the last announced label if it was this one.
            if stats.added_titles.len() > stats.added {
                stats.added_titles.pop();
            }
        } else {
            stats.removed += 1;
        }
        gone.push(old_id);
    }
    db::delete_episodes(&tx, &gone).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make(root: &Path, rel: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let f = fs::File::create(&p).unwrap();
        f.set_len(MIN_FILE_BYTES + 1).unwrap();
    }

    #[test]
    fn scan_groups_series_and_movies_and_tracks_progress() {
        let root = std::env::temp_dir().join(format!("vortex-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        make(&root, "Shows/Breaking Bad/Season 1/Breaking.Bad.S01E01.mkv");
        make(&root, "Shows/Breaking Bad/Season 1/Breaking.Bad.S01E02.mkv");
        make(&root, "Shows/Breaking Bad/Season 2/breaking bad s02e01.mp4");
        make(&root, "Movies/Inception.2010.1080p.mkv");
        make(&root, "Movies/sample.mkv");
        fs::write(root.join("Movies/notes.txt"), "x").unwrap();

        let mut conn = db::open(&root.join("test.db")).unwrap();
        db::add_library(&conn, root.to_str().unwrap()).unwrap();
        let stats = scan_all(&mut conn).unwrap();
        assert_eq!(stats.files_seen, 4);
        assert_eq!(stats.added, 4);

        let series = db::list_media(&conn, Some("series")).unwrap();
        assert_eq!(series.len(), 1, "all Breaking Bad variants should group into one series");
        assert_eq!(series[0].episode_count, 3);
        let movies = db::list_media(&conn, Some("movie")).unwrap();
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "Inception");
        assert_eq!(series[0].category.as_deref(), Some("Shows"));
        assert_eq!(movies[0].category.as_deref(), Some("Movies"));
        let lib_name = root.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(
            category_for(&root.join("Dark/Season 1/Dark.S01E01.mkv"), &root, &lib_name, "Dark").as_deref(),
            Some(lib_name.as_str())
        );
        assert_eq!(
            category_for(&root.join("Anime/Cowboy Bebop/ep.mkv"), &root, &lib_name, "Cowboy Bebop").as_deref(),
            Some("Anime")
        );
        assert_eq!(
            category_for(&root.join("Interstellar (2014)/movie.mkv"), &root, &lib_name, "Interstellar").as_deref(),
            Some(lib_name.as_str())
        );

        // Watch E01 fully, then continue-watching should suggest E02.
        let eps = db::list_episodes(&conn, series[0].id).unwrap();
        db::set_progress(&conn, eps[0].id, 0, true).unwrap();
        let cont = db::continue_watching(&conn, 10).unwrap();
        assert_eq!(cont.len(), 1);
        assert_eq!(cont[0].episode.id, eps[1].id);

        // Pause E02 part way; it should now be the continue item with a position.
        db::set_progress(&conn, eps[1].id, 600, false).unwrap();
        let cont = db::continue_watching(&conn, 10).unwrap();
        assert_eq!(cont[0].episode.position_secs, 600);

        // Renaming a file keeps its progress.
        fs::rename(
            root.join("Shows/Breaking Bad/Season 1/Breaking.Bad.S01E02.mkv"),
            root.join("Shows/Breaking Bad/Season 1/Breaking Bad - S01E02 - Cat's in the Bag.mkv"),
        )
        .unwrap();
        let stats = scan_all(&mut conn).unwrap();
        assert_eq!((stats.added, stats.removed, stats.renamed), (0, 0, 1));
        let eps = db::list_episodes(&conn, series[0].id).unwrap();
        let e02 = eps.iter().find(|e| e.episode == Some(2)).unwrap();
        assert!(e02.file_name.contains("Cat's in the Bag"));
        assert_eq!(e02.position_secs, 600, "progress must survive a rename");

        // Rescan is idempotent; deleting a file removes its row.
        fs::remove_file(root.join("Shows/Breaking Bad/Season 2/breaking bad s02e01.mp4")).unwrap();
        let stats = scan_all(&mut conn).unwrap();
        assert_eq!(stats.added, 0);
        assert_eq!(stats.removed, 1);
        assert_eq!(db::list_media(&conn, Some("series")).unwrap()[0].episode_count, 2);

        drop(conn);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extras_attach_to_their_series_and_movies() {
        let root = std::env::temp_dir().join(format!("vortex-extras-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let s1 = "Euphoria/Euphoria (2019) Season 1 S01 + Extras (1080p AMZN WEB-DL x265 HEVC 10bit EAC3 5.1 t3nzin)";
        let s2 = "Euphoria/Euphoria (2019) Season 2 S02 + Extras (1080p AMZN WEB-DL x265 HEVC 10bit EAC3 5.1 t3nzin)";
        make(&root, &format!("{s1}/Euphoria (2019) - S01E01 - Pilot.mkv"));
        make(&root, &format!("{s1}/Euphoria (2019) - S01E02 - Stuntin.mkv"));
        make(&root, &format!("{s1}/Featurettes/Behind The Scenes/Behind The Scenes.mkv"));
        make(&root, &format!("{s1}/Featurettes/Behind The Scenes/s01e02 - behind the scenes.mkv"));
        make(&root, &format!("{s1}/Featurettes/Trailers/visualizer.mkv"));
        make(&root, &format!("{s1}/Featurettes/Other/Scenes.mkv"));
        // Season 2 has only extras on disk, no episodes.
        make(&root, &format!("{s2}/Featurettes/Behind The Scenes/enter euphoria.mkv"));
        make(&root, "Dark/Season 1/Dark.S01E01.mkv");
        make(&root, "Dark/Season 1/Making of Dark.mkv");
        make(&root, "Movies/Inception (2010)/Inception.2010.1080p.mkv");
        make(&root, "Movies/Inception (2010)/Featurettes/Dream Logic.mkv");
        make(&root, "Movies/Standalone.2015.mkv");

        let mut conn = db::open(&root.join("test.db")).unwrap();
        db::add_library(&conn, root.to_str().unwrap()).unwrap();
        let stats = scan_all(&mut conn).unwrap();
        assert_eq!(stats.extras, 7);

        let series = db::list_media(&conn, Some("series")).unwrap();
        let titles: Vec<&str> = series.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, vec!["Dark", "Euphoria"], "featurettes must not become their own titles");
        let euphoria = series.iter().find(|m| m.title == "Euphoria").unwrap();
        assert_eq!(euphoria.year, Some(2019));
        assert_eq!(euphoria.episode_count, 2, "extras are not counted as episodes");
        let eps = db::list_episodes(&conn, euphoria.id).unwrap();
        let extras: Vec<(String, String)> =
            eps.iter().filter_map(|e| e.extra.clone().map(|x| (x, e.file_name.clone()))).collect();
        assert_eq!(extras.len(), 5);
        assert!(extras.iter().any(|(l, f)| l == "Season 1 › Featurettes › Behind The Scenes" && f == "Behind The Scenes.mkv"));
        assert!(extras.iter().any(|(l, f)| l == "Season 1 › Featurettes › Behind The Scenes" && f == "s01e02 - behind the scenes.mkv"));
        assert!(extras.iter().any(|(l, f)| l == "Season 2 › Featurettes › Behind The Scenes" && f == "enter euphoria.mkv"));

        // A show's own folder added as the library root: extras still attach.
        let show_root = std::env::temp_dir().join(format!("vortex-showroot-{}", std::process::id()));
        let _ = fs::remove_dir_all(&show_root);
        make(&show_root, "Euphoria (2019) Season 1 S01 + Extras (1080p)/Euphoria (2019) - S01E01 - Pilot.mkv");
        make(&show_root, "Euphoria (2019) Season 1 S01 + Extras (1080p)/Featurettes/Behind The Scenes/s01e02 - behind the scenes.mkv");
        make(&show_root, "Euphoria (2019) Season 2 S02 + Extras (1080p)/Featurettes/Behind The Scenes/enter euphoria.mkv");
        {
            let mut c2 = db::open(&show_root.join("t.db")).unwrap();
            db::add_library(&c2, show_root.to_str().unwrap()).unwrap();
            scan_all(&mut c2).unwrap();
            let s = db::list_media(&c2, Some("series")).unwrap();
            assert_eq!(s.len(), 1, "one series even when its folder is the library root");
            assert_eq!(s[0].episode_count, 1);
            let eps = db::list_episodes(&c2, s[0].id).unwrap();
            assert_eq!(eps.iter().filter(|e| e.extra.is_some()).count(), 2);
            assert!(db::list_media(&c2, Some("movie")).unwrap().is_empty());
        }
        let _ = fs::remove_dir_all(&show_root);
        let dark = series.iter().find(|m| m.title == "Dark").unwrap();
        let dark_eps = db::list_episodes(&conn, dark.id).unwrap();
        assert_eq!(dark_eps.iter().filter(|e| e.extra.is_some()).count(), 1);
        assert_eq!(dark_eps.iter().find(|e| e.extra.is_some()).unwrap().extra.as_deref(), Some("Season 1"));

        let movies = db::list_media(&conn, Some("movie")).unwrap();
        let titles: Vec<&str> = movies.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, vec!["Inception", "Standalone"]);
        let inception = &movies[0];
        assert_eq!(inception.episode_count, 1);
        let files = db::list_episodes(&conn, inception.id).unwrap();
        assert_eq!(files.iter().find(|e| e.extra.is_some()).unwrap().extra.as_deref(), Some("Featurettes"));

        // No duplicate groups: the featurette is not a second copy of the film.
        assert!(db::find_duplicates(&conn).unwrap().is_empty());
        // Next episode skips extras.
        let e01 = eps.iter().find(|e| e.episode == Some(1)).unwrap();
        assert_eq!(db::next_episode(&conn, e01.id).unwrap().unwrap().episode, Some(2));

        drop(conn);
        let _ = fs::remove_dir_all(&root);
    }
}
