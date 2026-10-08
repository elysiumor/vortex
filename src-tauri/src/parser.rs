use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub kind: Kind,
    pub title: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    /// True when the file sits inside an extras-style folder (Featurettes,
    /// Behind The Scenes…). Such files are bonus material even if their name
    /// carries an episode code like "s01e02 - behind the scenes".
    pub extra: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Movie,
    Series,
}

impl Kind {
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Movie => "movie",
            Kind::Series => "series",
        }
    }
}

// SxxExx, SxxExx-Exx, 1x02, "Season 1 Episode 2"
static SEASON_EP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[ ._\-\[(])S(\d{1,2})[ ._-]?E(\d{1,3})(?:[ ._-]?E?\d{1,3})*(?:$|[ ._\-\])])").unwrap()
});
static X_EP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[ ._\-\[(])(\d{1,2})x(\d{1,3})(?:$|[ ._\-\])])").unwrap());
static LONG_EP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)Season[ ._]?(\d{1,2})[ ._-]*Episode[ ._]?(\d{1,3})").unwrap());
// Episode without season: "E05", "Ep 05", "Episode 5", " - 05"
static EP_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[ ._\-\[(])(?:E|Ep|Episode)[ ._]?(\d{1,3})(?:$|[ ._\-\])v])").unwrap()
});
static DASH_EP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|\s)-\s*(\d{1,3})(?:v\d)?(?:\s|$|\[|\()").unwrap());
// Season folders: "Season 1", "Series 2", "S01", and names that merely contain
// them such as "Show (2019) Season 1 S01 + Extras (1080p ...)".
static SEASON_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[ ._\-\[(])(?:Season|Series)[ ._]?(\d{1,2})(?:$|[ ._\-\])])").unwrap());
static SEASON_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[ ._\-\[(])S(\d{1,2})(?:$|[ ._\-\])+])").unwrap());
static YEAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[ ._\-\[(])((?:19|20)\d{2})(?:$|[ ._\-\])])").unwrap());
/// Release tags. Several are ordinary words too ("web", "season", "dual",
/// "internal", "complete"), so a title is cut at the first tag that has some
/// title before it, never at one that starts it.
static JUNK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(2160p|1080p|1080i|720p|480p|4k|uhd|hdr|hdr10|dv|x264|x265|h\.?264|h\.?265|hevc|avc|xvid|divx|10bit|8bit|bluray|blu-ray|bdrip|brrip|webrip|web-dl|webdl|web|hdrip|dvdrip|dvd|hdtv|pdtv|cam|ts|tc|hdcam|remux|aac|ac3|eac3|dts|dd5\.?1|ddp5\.?1|5\.1|7\.1|atmos|truehd|yify|yts|rarbg|eztv|ettv|proper|repack|rerip|extended|unrated|remastered|directors\.?cut|internal|limited|multi|dual|dubbed|subbed|amzn|nf|dsnp|hmax|atvp|complete|season|s\d{1,2})\b",
    )
    .unwrap()
});
/// The subset of tags that never occur in a real title. Used on text that
/// ends where a year was found, which is all meant to be title: "Charlotte's
/// Web (2006)" must keep "Web", "A Complete Unknown (2024)" its "Complete".
static TECH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(2160p|1080p|1080i|720p|480p|4k|hdr10|x264|x265|h\.?264|h\.?265|hevc|xvid|divx|10bit|8bit|bluray|blu-ray|bdrip|brrip|webrip|web-dl|webdl|hdrip|dvdrip|hdtv|pdtv|hdcam|remux|aac|ac3|eac3|dd5\.?1|ddp5\.?1|truehd|yify|yts|rarbg|eztv|ettv|amzn|dsnp|hmax|atvp|the\s+complete\s+series|complete\s+series|complete\s+collection|extended\s+(?:edition|cut)|theatrical\s+cut|director'?s\s+cut|directors\s+cut|unrated|remastered)\b",
    )
    .unwrap()
});
static BRACKET_GROUP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\[[^\]]*\]\s*").unwrap());
static MULTI_SPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts", "mpg", "mpeg", "m2ts", "vob", "3gp", "ogv",
];

/// Folder names that hold bonus material rather than episodes or the film itself.
const EXTRAS_DIRS: &[&str] = &[
    "featurettes", "featurette", "bonus features", "special features", "behind the scenes", "deleted scenes",
    "bloopers", "making of", "outtakes", "gag reel",
];
/// Also bonus folders inside a title's folder, but plain enough to be a
/// category of their own directly under the library root ("Specials",
/// "Shorts", "Other"), where there is no title for them to belong to.
const GENERIC_EXTRAS_DIRS: &[&str] = &[
    "extras", "extra", "bonus", "specials", "trailers", "trailer", "interviews", "shorts", "other", "others",
    "scenes", "promos", "teasers", "webisodes",
];

pub fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_extras_dir(name: &str) -> bool {
    let n = name.trim().to_lowercase().replace(['_', '.'], " ");
    EXTRAS_DIRS.contains(&n.as_str()) || GENERIC_EXTRAS_DIRS.contains(&n.as_str())
}

fn is_specific_extras_dir(name: &str) -> bool {
    let n = name.trim().to_lowercase().replace(['_', '.'], " ");
    EXTRAS_DIRS.contains(&n.as_str())
}

/// Season number if the folder name is, or contains, a season marker.
pub fn season_from_dir(name: &str) -> Option<i32> {
    SEASON_WORD
        .captures(name)
        .or_else(|| SEASON_CODE.captures(name))
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

pub fn is_season_dir(name: &str) -> bool {
    season_from_dir(name).is_some()
}

/// Normalise a raw title fragment: strip release-group brackets, replace
/// separators with spaces, drop quality tags, trim punctuation.
pub fn clean_title(raw: &str) -> String {
    clean_with(raw, &JUNK)
}

/// For text that ends where a year was found: cut only unambiguous tags.
fn clean_title_before_year(raw: &str) -> String {
    clean_with(raw, &TECH)
}

fn clean_with(raw: &str, tags: &Regex) -> String {
    let s = BRACKET_GROUP.replace(raw, "");
    let s = s.replace(['.', '_'], " ");
    let s = s.trim_start();
    // A tag at the very start is the title itself ("Season of the Witch",
    // "Dual", "Internal Affairs"); cutting there emptied the title and filed
    // the film under its folder's name, merging unrelated films into one.
    let cut = tags
        .find_iter(s)
        .map(|m| m.start())
        .find(|&i| s[..i].chars().any(|c| c.is_alphanumeric()))
        .unwrap_or(s.len());
    let s = MULTI_SPACE.replace_all(&s[..cut], " ");
    s.trim_matches(|c: char| c.is_whitespace() || "-–—([+".contains(c)).to_string()
}

/// Key used for grouping: lowercase alphanumerics only.
pub fn sort_key(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Season, episode and (for "S01E01-E02") the last episode, when a file name
/// spells them out. None for names that only number episodes ("Show - 27").
pub fn episode_code(stem: &str) -> Option<(i32, i32, Option<i32>)> {
    if let Some(c) = SEASON_EP.captures(stem) {
        let season = parse_i32(&c[1])?;
        let first = parse_i32(&c[2])?;
        // Further episodes are numbers after an E or a dash: "S01E01E02",
        // "S01E01-E03", "S01E01-03". Bare dotted numbers that follow
        // ("S01E01.1.23.45") are something else.
        let whole = c.get(0)?.as_str();
        let after = &whole[c.get(2)?.end() - c.get(0)?.start()..];
        static MORE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(?:-\s*E?|E)(\d{1,3})").unwrap());
        let last = MORE.captures_iter(after).filter_map(|m| parse_i32(&m[1])).max().filter(|l| *l > first);
        return Some((season, first, last));
    }
    for re in [&*X_EP, &*LONG_EP] {
        if let Some(c) = re.captures(stem) {
            return Some((parse_i32(&c[1])?, parse_i32(&c[2])?, None));
        }
    }
    None
}

/// The release year a file or folder name carries, if any.
pub fn year_in(name: &str) -> Option<i32> {
    find_year(name).map(|(y, _)| y)
}

/// The release year and where it starts. A year in brackets wins ("Wonder
/// Woman 1984 (2020)"); otherwise the last one inside the name, since a year
/// that belongs to the title comes before the release year
/// ("Blade.Runner.2049.2017.1080p"). A name that starts with a year keeps it
/// as the title ("2012 (2009)", "1917"). The end of a range ("1999-2007") is
/// not a release year.
fn find_year(s: &str) -> Option<(i32, usize)> {
    let mut first = None;
    let mut inner = None;
    let mut bracketed = None;
    // Each search starts right after the previous year, not after its match:
    // the match takes the separator that follows ("2049."), which is also the
    // separator in front of the next year ("2049.2017").
    let mut at = 0;
    while let Some(cap) = YEAR.captures_at(s, at) {
        let m = cap.get(1).unwrap();
        at = m.end();
        let Ok(y) = m.as_str().parse::<i32>() else { continue };
        let before = &s[..m.start()];
        let range_end = before.len() >= 5
            && before.ends_with('-')
            && before[before.len() - 5..before.len() - 1].chars().all(|c| c.is_ascii_digit());
        if range_end {
            continue;
        }
        let in_brackets = before.ends_with(['(', '[']) && s[m.end()..].starts_with([')', ']']);
        if in_brackets {
            bracketed = Some((y, m.start()));
        }
        if m.start() > 0 {
            inner = Some((y, m.start()));
        }
        first.get_or_insert((y, m.start()));
    }
    bracketed.or(inner).or(first)
}

fn parse_i32(s: &str) -> Option<i32> {
    s.parse().ok()
}

/// (title, year) from a folder name such as "Euphoria (2019) Season 1 S01 + Extras (1080p ...)".
fn folder_title(name: &str) -> (String, Option<i32>) {
    match find_year(name) {
        Some((y, pos)) => (clean_title_before_year(&name[..pos]), Some(y)),
        None => (clean_title(name), None),
    }
}

/// Public wrapper so the scanner can name a series after its folder.
pub fn folder_title_of(name: &str) -> (String, Option<i32>) {
    folder_title(name)
}

/// Two titles describe the same thing when one's key is a prefix of the other's
/// ("Euphoria" vs "Euphoria (2019)", "Game of Thrones" vs "Game.of.Thrones").
fn agrees(a: &str, b: &str) -> bool {
    let (ka, kb) = (sort_key(a), sort_key(b));
    !ka.is_empty() && !kb.is_empty() && (ka.starts_with(&kb) || kb.starts_with(&ka))
}

/// True when any folder between the file and the library root is extras-like.
/// Generic names ("Specials", "Other") count only inside a title's folder;
/// directly under the root they are a category, and treating them as bonus
/// material hid every film in them.
pub fn in_extras_dir(path: &Path, library_root: &Path) -> bool {
    let mut cur = path.parent();
    while let Some(p) = cur {
        if p == library_root || !p.starts_with(library_root) {
            return false;
        }
        if let Some(name) = p.file_name().map(|n| n.to_string_lossy()) {
            let top_level = p.parent() == Some(library_root);
            if is_specific_extras_dir(&name) || (!top_level && is_extras_dir(&name)) {
                return true;
            }
        }
        cur = p.parent();
    }
    false
}

/// Parse a video file path, using parent folder names as hints.
/// `library_root` is the scanned root; folders at or above it are ignored.
pub fn parse(path: &Path, library_root: &Path) -> Parsed {
    let mut p = parse_inner(path, library_root);
    p.extra = in_extras_dir(path, library_root);
    p
}

fn parse_inner(path: &Path, library_root: &Path) -> Parsed {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let inside = |p: &Path| p != library_root && p.starts_with(library_root);
    let parent = path.parent().filter(|p| inside(p));
    let parent_name = parent.and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string());
    let grandparent = parent.and_then(|p| p.parent()).filter(|p| inside(p));
    let grandparent_name = grandparent.and_then(|p| p.file_name()).map(|s| s.to_string_lossy().to_string());

    // The folder that most likely carries the series name: the grandparent
    // when the parent is a season folder, otherwise the parent.
    let parent_is_season = parent_name.as_deref().map(is_season_dir).unwrap_or(false);
    let series_folder = if parent_is_season { grandparent_name.as_deref() } else { parent_name.as_deref() };
    let series_folder = series_folder.filter(|n| !is_season_dir(n) && !is_extras_dir(n));

    // 1. Full season + episode in the filename.
    for re in [&*SEASON_EP, &*X_EP, &*LONG_EP] {
        if let Some(cap) = re.captures(&stem) {
            let m = cap.get(0).unwrap();
            let season = parse_i32(&cap[1]);
            let episode = parse_i32(&cap[2]);
            let head = &stem[..m.start()];
            // Cut at the year so "Euphoria (2019) - " becomes "Euphoria".
            let (file_title, file_year) = folder_title(head);
            // Prefer the folder name when it agrees with the filename, so every
            // season of a show groups under one consistent title.
            let (title, year) = match series_folder.map(folder_title) {
                Some((ft, fy)) if !ft.is_empty() && (file_title.is_empty() || agrees(&ft, &file_title)) => {
                    (ft, fy.or(file_year))
                }
                _ if !file_title.is_empty() => (file_title, file_year),
                _ => series_folder.map(folder_title).unwrap_or((stem.clone(), None)),
            };
            return Parsed { kind: Kind::Series, title, year, season, episode, extra: false };
        }
    }

    // 2. Season from folder, episode number from filename.
    let season_from_folder = parent_name.as_deref().and_then(season_from_dir);
    let ep_from_name = EP_ONLY
        .captures(&stem)
        .or_else(|| DASH_EP.captures(&stem))
        .and_then(|c| parse_i32(&c[1]));

    if let Some(season) = season_from_folder {
        let (title, year) = series_folder.map(folder_title).filter(|(t, _)| !t.is_empty()).unwrap_or_else(|| (clean_title(&stem), None));
        // No episode number: bonus material inside a season folder. The scanner
        // attaches it to the series as an extra.
        return Parsed { kind: Kind::Series, title, year, season: Some(season), episode: ep_from_name, extra: false };
    }
    if let (Some(episode), Some(folder)) = (ep_from_name, series_folder) {
        // "Show Name/Show Name - 05.mkv" (common for anime). A season in the
        // name ("Show S2 - 05") counts; it used to become S1E05 and collide
        // with the real first-season episode.
        let (title, year) = folder_title(folder);
        let season = season_from_dir(&stem).unwrap_or(1);
        return Parsed { kind: Kind::Series, title, year, season: Some(season), episode: Some(episode), extra: false };
    }

    // 3. Movie.
    let (year, mut title) = match find_year(&stem) {
        Some((y, pos)) => (Some(y), clean_title_before_year(&stem[..pos])),
        None => (None, clean_title(&stem)),
    };
    if title.is_empty() {
        // Filename was just a year or junk; fall back to the folder name.
        if let Some(folder) = parent_name.as_deref() {
            let (ft, fy) = folder_title(folder);
            return Parsed { kind: Kind::Movie, title: ft, year: year.or(fy), season: None, episode: None, extra: false };
        }
        title = stem.clone();
    }
    Parsed { kind: Kind::Movie, title, year, season: None, episode: None, extra: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(rel: &str) -> Parsed {
        let root = PathBuf::from("D:/Media");
        parse(&root.join(rel), &root)
    }

    #[test]
    fn standard_series_name() {
        let r = p("Breaking Bad/Season 1/Breaking.Bad.S01E02.1080p.BluRay.x264.mkv");
        assert_eq!(r.kind, Kind::Series);
        assert_eq!(r.title, "Breaking Bad");
        assert_eq!((r.season, r.episode), (Some(1), Some(2)));
    }

    #[test]
    fn series_with_year_in_name() {
        let r = p("Shows/The.Office.2005.S03E10.720p.HDTV.mkv");
        assert_eq!(r.title, "The Office", "category folder must not override the filename title");
        assert_eq!(r.year, Some(2005));
        assert_eq!((r.season, r.episode), (Some(3), Some(10)));
    }

    #[test]
    fn x_format() {
        let r = p("Friends/Friends 2x07 The One Where.avi");
        assert_eq!(r.title, "Friends");
        assert_eq!((r.season, r.episode), (Some(2), Some(7)));
    }

    #[test]
    fn multi_episode_file() {
        let r = p("Show/Show.S01E01-E02.mkv");
        assert_eq!((r.season, r.episode), (Some(1), Some(1)));
    }

    #[test]
    fn season_folder_with_episode_only() {
        let r = p("Dark/Season 2/Episode 03.mkv");
        assert_eq!(r.kind, Kind::Series);
        assert_eq!(r.title, "Dark");
        assert_eq!((r.season, r.episode), (Some(2), Some(3)));
    }

    #[test]
    fn anime_dash_number() {
        let r = p("Cowboy Bebop/[Group] Cowboy Bebop - 05 [1080p].mkv");
        assert_eq!(r.kind, Kind::Series);
        assert_eq!(r.title, "Cowboy Bebop");
        assert_eq!((r.season, r.episode), (Some(1), Some(5)));
    }

    #[test]
    fn descriptive_season_folder_groups_under_series_folder() {
        let s1 = p("Euphoria/Euphoria (2019) Season 1 S01 + Extras (1080p AMZN WEB-DL x265 HEVC 10bit EAC3 5.1 t3nzin)/Euphoria (2019) - S01E01 - Pilot (1080p AMZN WEB-DL x265 t3nzin).mkv");
        let s2 = p("Euphoria/Euphoria (2019) Season 2 S02 + Extras (1080p AMZN WEB-DL x265 HEVC 10bit EAC3 5.1 t3nzin)/Euphoria.S02E03.mkv");
        assert_eq!(s1.title, "Euphoria");
        assert_eq!(s2.title, "Euphoria");
        assert_eq!(s1.year, Some(2019));
        assert_eq!((s1.season, s1.episode), (Some(1), Some(1)));
        assert_eq!((s2.season, s2.episode), (Some(2), Some(3)));
    }

    #[test]
    fn featurettes_are_flagged_even_with_episode_codes() {
        let r = p("Euphoria/Euphoria (2019) Season 1 S01 + Extras (1080p)/Featurettes/Behind The Scenes/enter euphoria.mkv");
        assert!(r.extra);
        let r = p("Euphoria/Euphoria (2019) Season 1 S01 + Extras (1080p)/Featurettes/Behind The Scenes/s01e02 - behind the scenes.mkv");
        assert!(r.extra, "episode code in a featurette name must not make it an episode");
        let r = p("Dark/Season 1/Making of Dark.mkv");
        assert!(!r.extra);
        assert_eq!(r.kind, Kind::Series);
        assert_eq!(r.title, "Dark");
        assert_eq!((r.season, r.episode), (Some(1), None));
        let r = p("Dark/Season 1/Dark.S01E01.mkv");
        assert!(!r.extra);
    }

    #[test]
    fn year_in_brackets_before_episode_code_is_stripped() {
        let r = p("Euphoria/Euphoria (2019) - S01E03 - Made You Look (1080p AMZN WEB-DL).mkv");
        assert_eq!(r.title, "Euphoria");
        assert_eq!(r.year, Some(2019));
        // Series folder is the library root itself: filename must still be clean.
        let root = PathBuf::from("D:/Euphoria");
        let r = parse(&root.join("Euphoria (2019) Season 1 S01 + Extras (1080p)/Euphoria (2019) - S01E01 - Pilot.mkv"), &root);
        assert_eq!(r.title, "Euphoria");
        assert_eq!((r.season, r.episode), (Some(1), Some(1)));
    }

    #[test]
    fn season_dir_detection() {
        assert_eq!(season_from_dir("Season 1"), Some(1));
        assert_eq!(season_from_dir("S02"), Some(2));
        assert_eq!(season_from_dir("Euphoria (2019) Season 1 S01 + Extras (1080p)"), Some(1));
        assert_eq!(season_from_dir("Game of Thrones"), None);
        assert_eq!(season_from_dir("Featurettes"), None);
        assert!(is_extras_dir("Behind The Scenes"));
        assert!(is_extras_dir("featurettes"));
        assert!(!is_extras_dir("Season 1"));
    }

    #[test]
    fn movie_dotted() {
        let r = p("Movies/Inception.2010.1080p.BluRay.x264-YIFY.mp4");
        assert_eq!(r.kind, Kind::Movie);
        assert_eq!(r.title, "Inception");
        assert_eq!(r.year, Some(2010));
    }

    #[test]
    fn movie_parens() {
        let r = p("Movies/Interstellar (2014).mkv");
        assert_eq!(r.title, "Interstellar");
        assert_eq!(r.year, Some(2014));
    }

    #[test]
    fn movie_year_title() {
        let r = p("Movies/2012 (2009) 720p.mkv");
        assert_eq!(r.title, "2012");
        assert_eq!(r.year, Some(2009));
    }

    #[test]
    fn movie_in_own_folder_with_junk_filename() {
        let r = p("Movies/The Matrix (1999)/1999.mkv");
        assert_eq!(r.title, "The Matrix");
        assert_eq!(r.year, Some(1999));
    }

    #[test]
    fn movie_no_year() {
        let r = p("Movies/Some Home Video.mp4");
        assert_eq!(r.kind, Kind::Movie);
        assert_eq!(r.title, "Some Home Video");
        assert_eq!(r.year, None);
    }

    #[test]
    fn sort_key_groups_variants() {
        assert_eq!(sort_key("Breaking Bad"), sort_key("breaking.bad"));
    }

    /// Titles made of words that are also release tags used to come out
    /// empty, fall back to the folder name, and merge into one "Movies" item.
    #[test]
    fn titles_that_look_like_release_tags_survive() {
        for (rel, title, year) in [
            ("Movies/Internal Affairs (1990).mkv", "Internal Affairs", 1990),
            ("Movies/Dual (2022).mkv", "Dual", 2022),
            ("Movies/Season of the Witch (2011).mkv", "Season of the Witch", 2011),
            ("Movies/Charlotte's Web (2006).mkv", "Charlotte's Web", 2006),
            ("Movies/A Complete Unknown (2024) 1080p WEB-DL.mkv", "A Complete Unknown", 2024),
            ("Movies/Cam.2018.1080p.NF.WEB-DL.mkv", "Cam", 2018),
        ] {
            let r = p(rel);
            assert_eq!((r.title.as_str(), r.year), (title, Some(year)), "{rel}");
        }
        // Tags after the title are still cut when there is no year to cut at.
        assert_eq!(p("Movies/Some.Film.1080p.WEB-DL.x264.mkv").title, "Some Film");
    }

    /// A year that belongs to the title must not be taken for the release
    /// year: the films below were filed as the earlier film of the same name.
    #[test]
    fn release_year_not_the_year_in_the_title() {
        for (rel, title, year) in [
            ("Movies/Wonder Woman 1984 (2020).mkv", "Wonder Woman 1984", 2020),
            ("Movies/Blade.Runner.2049.2017.1080p.BluRay.mkv", "Blade Runner 2049", 2017),
            ("Movies/1917 (2019).mkv", "1917", 2019),
            ("Movies/2001.A.Space.Odyssey.1968.1080p.mkv", "2001 A Space Odyssey", 1968),
            ("Movies/The Lord of the Rings (Extended Edition) (2001).mkv", "The Lord of the Rings", 2001),
        ] {
            let r = p(rel);
            assert_eq!((r.title.as_str(), r.year), (title, Some(year)), "{rel}");
        }
        assert_eq!(folder_title_of("The.Sopranos.Complete.Series.1999-2007.1080p"), ("The Sopranos".to_string(), Some(1999)));
    }

    #[test]
    fn episode_codes_and_ranges() {
        assert_eq!(episode_code("Show.S01E05.1080p.WEB"), Some((1, 5, None)));
        assert_eq!(episode_code("Show S02E01-E02 720p"), Some((2, 1, Some(2))));
        assert_eq!(episode_code("Show.S02E01E02E03"), Some((2, 1, Some(3))));
        assert_eq!(episode_code("Show 3x07"), Some((3, 7, None)));
        assert_eq!(episode_code("Chernobyl.S01E01.1.23.45"), Some((1, 1, None)), "dotted numbers are not more episodes");
        let r = p("Anime/Oshi no Ko/[SubsPlease] Oshi no Ko S2 - 05 (1080p).mkv");
        assert_eq!((r.season, r.episode), (Some(2), Some(5)), "the season in the name counts");
        assert_eq!(episode_code("Show S01E01-03"), Some((1, 1, Some(3))));
        assert_eq!(episode_code("[SubsPlease] Show - 27 (1080p) [ABCD1234]"), None, "anime numbering names no season");
    }

    #[test]
    fn generic_extras_names_are_categories_at_the_top() {
        let root = PathBuf::from("D:/Media");
        let special = root.join("Specials/Dave Chappelle The Bird Revelation (2017).mkv");
        assert!(!parse(&special, &root).extra, "a top-level Specials folder is a category");
        let inside = root.join("Dark/Specials/Making Dark.mkv");
        assert!(parse(&inside, &root).extra, "inside a title's folder it is bonus material");
        let featurette = root.join("Featurettes/Clip.mkv");
        assert!(parse(&featurette, &root).extra, "specific names count at any depth");
    }
}
