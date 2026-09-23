use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use std::path::Path;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS libraries (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS media_items (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    year INTEGER,
    sort_key TEXT NOT NULL,
    category TEXT,
    UNIQUE(kind, sort_key)
);
CREATE TABLE IF NOT EXISTS episodes (
    id INTEGER PRIMARY KEY,
    media_item_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    library_id INTEGER NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    season INTEGER,
    episode INTEGER,
    size INTEGER NOT NULL,
    modified INTEGER NOT NULL,
    duration_secs INTEGER
);
CREATE INDEX IF NOT EXISTS idx_episodes_item ON episodes(media_item_id);
CREATE TABLE IF NOT EXISTS watch_progress (
    episode_id INTEGER PRIMARY KEY REFERENCES episodes(id) ON DELETE CASCADE,
    position_secs INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    last_watched TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY,
    episode_id INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    at TEXT NOT NULL DEFAULT (datetime('now')),
    position_secs INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    exact INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'player'
);
CREATE INDEX IF NOT EXISTS idx_history_at ON history(at);
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'tag',
    tmdb_collection_id INTEGER,
    poster_url TEXT,
    overview TEXT,
    UNIQUE(kind, name COLLATE NOCASE)
);
CREATE TABLE IF NOT EXISTS item_tags (
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    media_item_id INTEGER NOT NULL REFERENCES media_items(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(tag_id, media_item_id)
);
CREATE TABLE IF NOT EXISTS tmdb_details (
    media_item_id INTEGER PRIMARY KEY REFERENCES media_items(id) ON DELETE CASCADE,
    tmdb_id INTEGER NOT NULL,
    json TEXT NOT NULL,
    fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    // Migration for databases created before the column existed; ignored if present.
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN category TEXT", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN tmdb_id INTEGER", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN poster_path TEXT", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN overview TEXT", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN poster_checked INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN duration_checked INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN title TEXT", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN overview TEXT", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN air_date TEXT", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN still_path TEXT", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN rating REAL", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN added_at TEXT", []);
    let _ = conn.execute("UPDATE episodes SET added_at = datetime(modified, 'unixepoch') WHERE added_at IS NULL", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN rating REAL", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN extra TEXT", []);
    let _ = conn.execute("ALTER TABLE watch_progress ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE episodes ADD COLUMN subtitles TEXT", []);
    let _ = conn.execute("ALTER TABLE media_items ADD COLUMN genres TEXT", []);
    // Backfill genres from cached details for titles matched before the column existed.
    let _ = conn.execute(
        "UPDATE media_items SET genres = (
            SELECT group_concat(json_extract(g.value, '$.name'), ', ')
            FROM tmdb_details d, json_each(json_extract(d.json, '$.genres')) g WHERE d.media_item_id = media_items.id
         ) WHERE genres IS NULL AND id IN (SELECT media_item_id FROM tmdb_details)",
        [],
    );
    // Backfill ratings for titles whose details were cached before the column existed.
    let _ = conn.execute(
        "UPDATE media_items SET rating = (
            SELECT ROUND(json_extract(d.json, '$.vote_average'), 1) FROM tmdb_details d WHERE d.media_item_id = media_items.id
         ) WHERE rating IS NULL AND id IN (SELECT media_item_id FROM tmdb_details)",
        [],
    );
    Ok(conn)
}

#[derive(Serialize, Clone, Debug)]
pub struct Library {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub available: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct MediaItem {
    pub id: i64,
    pub kind: String,
    pub title: String,
    pub year: Option<i32>,
    pub category: Option<String>,
    pub tmdb_id: Option<i64>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    pub rating: Option<f64>,
    pub genres: Option<String>,
    /// User tags, comma-separated.
    pub tags: Option<String>,
    /// Collections this title belongs to, comma-separated.
    pub collections: Option<String>,
    pub episode_count: i64,
    pub watched_count: i64,
    pub last_watched: Option<String>,
    pub added_at: Option<String>,
    pub total_size: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct Episode {
    pub id: i64,
    pub media_item_id: i64,
    pub path: String,
    pub file_name: String,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub size: i64,
    pub modified: i64,
    pub duration_secs: Option<i64>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub still_path: Option<String>,
    pub rating: Option<f64>,
    /// Set for bonus material (featurettes, trailers…); the label groups them.
    pub extra: Option<String>,
    /// Sidecar subtitle files next to the video, "|"-separated full paths.
    pub subtitles: Option<String>,
    pub position_secs: i64,
    pub completed: bool,
    pub last_watched: Option<String>,
    pub available: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ContinueItem {
    pub episode: Episode,
    pub title: String,
    pub kind: String,
    pub year: Option<i32>,
    pub poster_path: Option<String>,
    pub tmdb_id: Option<i64>,
}

fn episode_from_row(r: &Row) -> rusqlite::Result<Episode> {
    let path: String = r.get("path")?;
    let available = Path::new(&path).exists();
    Ok(Episode {
        id: r.get("id")?,
        media_item_id: r.get("media_item_id")?,
        file_name: r.get("file_name")?,
        season: r.get("season")?,
        episode: r.get("episode")?,
        size: r.get("size")?,
        modified: r.get("modified")?,
        duration_secs: r.get("duration_secs")?,
        title: r.get("title")?,
        overview: r.get("overview")?,
        air_date: r.get("air_date")?,
        still_path: r.get("still_path")?,
        rating: r.get("rating")?,
        extra: r.get("extra")?,
        subtitles: r.get("subtitles")?,
        position_secs: r.get::<_, Option<i64>>("position_secs")?.unwrap_or(0),
        completed: r.get::<_, Option<i64>>("completed")?.unwrap_or(0) != 0,
        last_watched: r.get("last_watched")?,
        available,
        path,
    })
}

const EPISODE_SELECT: &str = r#"
SELECT e.id, e.media_item_id, e.path, e.file_name, e.season, e.episode, e.size, e.modified, e.duration_secs,
       e.title, e.overview, e.air_date, e.still_path, e.rating, e.extra, e.subtitles,
       w.position_secs, w.completed, w.last_watched, w.hidden
FROM episodes e LEFT JOIN watch_progress w ON w.episode_id = e.id
"#;

// ---------- libraries ----------

/// Rows only, with `available` left false. Deciding availability means
/// touching the filesystem, which blocks for seconds on a sleeping, network
/// or disconnected drive; callers fill it in via `fill_availability` after
/// releasing the database lock.
pub fn list_libraries_rows(conn: &Connection) -> rusqlite::Result<Vec<Library>> {
    let mut stmt = conn.prepare("SELECT id, path, name FROM libraries ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(Library { id: r.get(0)?, available: false, path: r.get(1)?, name: r.get(2)? })
    })?;
    rows.collect()
}

/// Probe each path. Must not run while the database lock is held.
pub fn fill_availability(libs: &mut [Library]) {
    for lib in libs {
        lib.available = Path::new(&lib.path).is_dir();
    }
}

/// Convenience for callers that already own their connection (a scan), where
/// blocking on a drive costs nobody else anything.
pub fn list_libraries(conn: &Connection) -> rusqlite::Result<Vec<Library>> {
    let mut libs = list_libraries_rows(conn)?;
    fill_availability(&mut libs);
    Ok(libs)
}

pub fn add_library(conn: &Connection, path: &str) -> rusqlite::Result<Library> {
    let name = Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.to_string());
    conn.execute(
        "INSERT OR IGNORE INTO libraries(path, name) VALUES (?1, ?2)",
        params![path, name],
    )?;
    let id: i64 = conn.query_row("SELECT id FROM libraries WHERE path = ?1", [path], |r| r.get(0))?;
    Ok(Library { id, path: path.to_string(), name, available: Path::new(path).is_dir() })
}

pub fn remove_library(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM libraries WHERE id = ?1", [id])?;
    prune_empty_items(conn)
}

pub fn prune_empty_items(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM media_items WHERE id NOT IN (SELECT DISTINCT media_item_id FROM episodes)",
        [],
    )?;
    Ok(())
}

// ---------- scanning upserts ----------

pub fn upsert_media_item(
    conn: &Connection,
    kind: &str,
    title: &str,
    year: Option<i32>,
    sort_key: &str,
    category: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO media_items(kind, title, year, sort_key, category) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(kind, sort_key) DO UPDATE SET
            year = COALESCE(media_items.year, excluded.year),
            category = COALESCE(media_items.category, excluded.category)",
        params![kind, title, year, sort_key, category],
    )?;
    conn.query_row(
        "SELECT id FROM media_items WHERE kind = ?1 AND sort_key = ?2",
        params![kind, sort_key],
        |r| r.get(0),
    )
}

pub struct NewEpisode<'a> {
    pub media_item_id: i64,
    pub library_id: i64,
    pub path: &'a str,
    pub file_name: &'a str,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub size: i64,
    pub modified: i64,
    pub extra: Option<&'a str>,
    pub subtitles: Option<&'a str>,
}

/// Returns true if the row was newly inserted.
pub fn upsert_episode(conn: &Connection, e: &NewEpisode) -> rusqlite::Result<bool> {
    let existing: Option<i64> =
        conn.query_row("SELECT id FROM episodes WHERE path = ?1", [e.path], |r| r.get(0)).optional()?;
    conn.execute(
        "INSERT INTO episodes(media_item_id, library_id, path, file_name, season, episode, size, modified, added_at, extra, subtitles)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), ?9, ?10)
         ON CONFLICT(path) DO UPDATE SET
            media_item_id = excluded.media_item_id, library_id = excluded.library_id,
            file_name = excluded.file_name, season = excluded.season, episode = excluded.episode,
            size = excluded.size, modified = excluded.modified, extra = excluded.extra, subtitles = excluded.subtitles",
        params![e.media_item_id, e.library_id, e.path, e.file_name, e.season, e.episode, e.size, e.modified, e.extra, e.subtitles],
    )?;
    Ok(existing.is_none())
}

// ---------- statistics ----------

#[derive(Serialize, Clone, Debug, Default)]
pub struct Stats {
    pub movies: i64,
    pub series: i64,
    pub episodes: i64,
    pub total_bytes: i64,
    pub watched_movies: i64,
    pub watched_episodes: i64,
    pub hours_total: f64,
    /// ("YYYY-MM", hours) for the last 12 months with activity
    pub hours_by_month: Vec<(String, f64)>,
    /// (genre, titles watched)
    pub top_genres: Vec<(String, i64)>,
    /// (category, bytes)
    pub storage_by_category: Vec<(String, i64)>,
    /// Series with some but not all episodes watched: (id, title, watched, total, last_watched)
    pub in_progress: Vec<(i64, String, i64, i64, Option<String>)>,
    /// Series untouched for 60+ days but not finished: (id, title, days)
    pub stale: Vec<(i64, String, i64)>,
}

pub fn stats(conn: &Connection) -> rusqlite::Result<Stats> {
    let mut s = Stats::default();
    s.movies = conn.query_row("SELECT COUNT(*) FROM media_items WHERE kind = 'movie'", [], |r| r.get(0))?;
    s.series = conn.query_row("SELECT COUNT(*) FROM media_items WHERE kind = 'series'", [], |r| r.get(0))?;
    s.episodes = conn.query_row(
        "SELECT COUNT(*) FROM episodes e JOIN media_items m ON m.id = e.media_item_id WHERE m.kind = 'series' AND e.extra IS NULL",
        [], |r| r.get(0),
    )?;
    s.total_bytes = conn.query_row("SELECT COALESCE(SUM(size), 0) FROM episodes", [], |r| r.get(0))?;
    s.watched_movies = conn.query_row(
        "SELECT COUNT(DISTINCT m.id) FROM media_items m JOIN episodes e ON e.media_item_id = m.id
         JOIN watch_progress w ON w.episode_id = e.id WHERE m.kind = 'movie' AND w.completed = 1",
        [], |r| r.get(0),
    )?;
    s.watched_episodes = conn.query_row(
        "SELECT COUNT(*) FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         JOIN watch_progress w ON w.episode_id = e.id WHERE m.kind = 'series' AND e.extra IS NULL AND w.completed = 1",
        [], |r| r.get(0),
    )?;
    let secs: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN h.completed = 1 AND e.duration_secs IS NOT NULL THEN e.duration_secs ELSE h.position_secs END), 0)
         FROM history h JOIN episodes e ON e.id = h.episode_id WHERE h.source = 'player'",
        [], |r| r.get(0),
    )?;
    s.hours_total = (secs / 360.0).round() / 10.0;

    let mut stmt = conn.prepare(
        "SELECT substr(h.at, 1, 7) AS ym,
                SUM(CASE WHEN h.completed = 1 AND e.duration_secs IS NOT NULL THEN e.duration_secs ELSE h.position_secs END)
         FROM history h JOIN episodes e ON e.id = h.episode_id WHERE h.source = 'player'
         GROUP BY ym ORDER BY ym DESC LIMIT 12",
    )?;
    let mut months: Vec<(String, f64)> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, (r.get::<_, f64>(1)? / 360.0).round() / 10.0)))?
        .collect::<Result<_, _>>()?;
    months.reverse();
    s.hours_by_month = months;

    let mut stmt = conn.prepare(
        "SELECT m.genres FROM media_items m WHERE m.genres IS NOT NULL AND EXISTS (
            SELECT 1 FROM episodes e JOIN watch_progress w ON w.episode_id = e.id WHERE e.media_item_id = m.id AND w.completed = 1)",
    )?;
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for g in stmt.query_map([], |r| r.get::<_, String>(0))?.flatten() {
        for name in g.split(", ") {
            *counts.entry(name.to_string()).or_default() += 1;
        }
    }
    let mut genres: Vec<(String, i64)> = counts.into_iter().collect();
    genres.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    genres.truncate(8);
    s.top_genres = genres;

    let mut stmt = conn.prepare(
        "SELECT COALESCE(m.category, 'Uncategorised'), SUM(e.size) FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         GROUP BY 1 ORDER BY 2 DESC LIMIT 10",
    )?;
    s.storage_by_category = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;

    let items = list_media(conn, Some("series"))?;
    let mut in_progress: Vec<(i64, String, i64, i64, Option<String>)> = items
        .iter()
        .filter(|m| m.watched_count > 0 && m.watched_count < m.episode_count)
        .map(|m| (m.id, m.title.clone(), m.watched_count, m.episode_count, m.last_watched.clone()))
        .collect();
    in_progress.sort_by(|a, b| b.4.cmp(&a.4));
    in_progress.truncate(10);
    s.in_progress = in_progress;

    let now: i64 = conn.query_row("SELECT strftime('%s','now')", [], |r| r.get(0))?;
    let mut stale: Vec<(i64, String, i64)> = items
        .iter()
        .filter(|m| m.watched_count > 0 && m.watched_count < m.episode_count)
        .filter_map(|m| {
            let lw = m.last_watched.as_ref()?;
            let t: i64 = conn.query_row("SELECT strftime('%s', ?1)", [lw], |r| r.get(0)).ok()?;
            let days = (now - t) / 86400;
            (days >= 60).then_some((m.id, m.title.clone(), days))
        })
        .collect();
    stale.sort_by(|a, b| b.2.cmp(&a.2));
    stale.truncate(10);
    s.stale = stale;
    Ok(s)
}

/// Carry watch progress and history from one episode row to another
/// (used when a file was renamed or moved and re-appears under a new path).
pub fn move_progress(conn: &Connection, from: i64, to: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO watch_progress(episode_id, position_secs, completed, last_watched)
         SELECT ?2, position_secs, completed, last_watched FROM watch_progress WHERE episode_id = ?1",
        params![from, to],
    )?;
    conn.execute("UPDATE history SET episode_id = ?2 WHERE episode_id = ?1", params![from, to])?;
    conn.execute(
        "UPDATE episodes SET added_at = COALESCE((SELECT added_at FROM episodes WHERE id = ?1), added_at) WHERE id = ?2",
        params![from, to],
    )?;
    conn.execute(
        "UPDATE episodes SET still_path = COALESCE(still_path, (SELECT still_path FROM episodes WHERE id = ?1)),
                             rating = COALESCE(rating, (SELECT rating FROM episodes WHERE id = ?1))
         WHERE id = ?2",
        params![from, to],
    )?;
    conn.execute(
        "UPDATE episodes SET duration_secs = COALESCE((SELECT duration_secs FROM episodes WHERE id = ?1), duration_secs),
                             title = COALESCE(title, (SELECT title FROM episodes WHERE id = ?1)),
                             overview = COALESCE(overview, (SELECT overview FROM episodes WHERE id = ?1)),
                             air_date = COALESCE(air_date, (SELECT air_date FROM episodes WHERE id = ?1))
         WHERE id = ?2",
        params![from, to],
    )?;
    Ok(())
}

pub fn episode_fingerprints_for_library(conn: &Connection, library_id: i64) -> rusqlite::Result<Vec<(i64, String, i64, i64)>> {
    let mut stmt = conn.prepare("SELECT id, path, size, modified FROM episodes WHERE library_id = ?1")?;
    let rows = stmt.query_map([library_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    rows.collect()
}

pub fn delete_episodes(conn: &Connection, ids: &[i64]) -> rusqlite::Result<()> {
    for id in ids {
        conn.execute("DELETE FROM episodes WHERE id = ?1", [id])?;
    }
    Ok(())
}

// ---------- durations ----------

pub fn episodes_missing_duration(conn: &Connection) -> rusqlite::Result<Vec<(i64, String)>> {
    let mut stmt =
        conn.prepare("SELECT id, path FROM episodes WHERE duration_secs IS NULL AND duration_checked = 0")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}

pub fn set_duration(conn: &Connection, episode_id: i64, secs: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE episodes SET duration_secs = ?2, duration_checked = 1 WHERE id = ?1",
        params![episode_id, secs],
    )?;
    Ok(())
}

pub fn mark_duration_checked(conn: &Connection, episode_id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE episodes SET duration_checked = 1 WHERE id = ?1", [episode_id])?;
    Ok(())
}

// ---------- episode metadata ----------

pub fn seasons_for_item(conn: &Connection, media_item_id: i64) -> rusqlite::Result<Vec<i32>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT season FROM episodes WHERE media_item_id = ?1 AND season IS NOT NULL ORDER BY season",
    )?;
    let rows = stmt.query_map([media_item_id], |r| r.get(0))?;
    rows.collect()
}

pub fn set_episode_meta(
    conn: &Connection,
    media_item_id: i64,
    season: i32,
    episode: i32,
    title: Option<&str>,
    overview: Option<&str>,
    air_date: Option<&str>,
    still_path: Option<&str>,
    rating: Option<f64>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE episodes SET title = ?4, overview = ?5, air_date = ?6, still_path = ?7, rating = ?8
         WHERE media_item_id = ?1 AND season = ?2 AND episode = ?3",
        params![media_item_id, season, episode, title, overview, air_date, still_path, rating],
    )
}

// ---------- TMDB detail cache ----------

pub fn get_details_json(conn: &Connection, media_item_id: i64) -> rusqlite::Result<Option<(i64, String, String)>> {
    conn.query_row(
        "SELECT tmdb_id, json, fetched_at FROM tmdb_details WHERE media_item_id = ?1",
        [media_item_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()
}

pub fn set_details_json(conn: &Connection, media_item_id: i64, tmdb_id: i64, json: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tmdb_details(media_item_id, tmdb_id, json, fetched_at) VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(media_item_id) DO UPDATE SET tmdb_id = excluded.tmdb_id, json = excluded.json, fetched_at = excluded.fetched_at",
        params![media_item_id, tmdb_id, json],
    )?;
    Ok(())
}

// ---------- history ----------

#[derive(Serialize, Clone, Debug)]
pub struct HistoryEntry {
    pub id: i64,
    pub at: String,
    pub position_secs: i64,
    pub completed: bool,
    pub exact: bool,
    pub source: String,
    pub episode: Episode,
    pub item_title: String,
    pub kind: String,
    pub media_item_id: i64,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct HistoryStats {
    pub sessions_30d: i64,
    pub completed_year: i64,
    pub hours_year: f64,
    pub total_sessions: i64,
}

pub fn add_history(
    conn: &Connection,
    episode_id: i64,
    position_secs: i64,
    completed: bool,
    exact: bool,
    source: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO history(episode_id, position_secs, completed, exact, source) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![episode_id, position_secs, completed as i64, exact as i64, source],
    )?;
    Ok(())
}

pub fn list_history(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT h.id, h.at, h.position_secs, h.completed, h.exact, h.source, h.episode_id, m.title, m.kind, m.id
         FROM history h JOIN episodes e ON e.id = h.episode_id JOIN media_items m ON m.id = e.media_item_id
         ORDER BY h.at DESC, h.id DESC LIMIT ?1",
    )?;
    let rows: Vec<(i64, String, i64, i64, i64, String, i64, String, String, i64)> = stmt
        .query_map([limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, at, position_secs, completed, exact, source, episode_id, item_title, kind, media_item_id) in rows {
        if let Some(episode) = get_episode(conn, episode_id)? {
            out.push(HistoryEntry {
                id,
                at,
                position_secs,
                completed: completed != 0,
                exact: exact != 0,
                source,
                episode,
                item_title,
                kind,
                media_item_id,
            });
        }
    }
    Ok(out)
}

pub fn history_stats(conn: &Connection) -> rusqlite::Result<HistoryStats> {
    let sessions_30d =
        conn.query_row("SELECT COUNT(*) FROM history WHERE at >= datetime('now', '-30 days')", [], |r| r.get(0))?;
    let completed_year = conn.query_row(
        "SELECT COUNT(DISTINCT episode_id) FROM history WHERE completed = 1 AND at >= datetime('now', 'start of year')",
        [],
        |r| r.get(0),
    )?;
    // Hours: sum of full durations for completed sessions, else the position reached.
    let secs: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN h.completed = 1 AND e.duration_secs IS NOT NULL THEN e.duration_secs
                                  ELSE h.position_secs END), 0)
         FROM history h JOIN episodes e ON e.id = h.episode_id
         WHERE h.source = 'player' AND h.at >= datetime('now', 'start of year')",
        [],
        |r| r.get(0),
    )?;
    let total_sessions = conn.query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))?;
    Ok(HistoryStats { sessions_30d, completed_year, hours_year: (secs / 360.0).round() / 10.0, total_sessions })
}

pub fn delete_history(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM history WHERE id = ?1", [id])?;
    Ok(())
}

pub fn clear_history(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

// ---------- duplicates ----------

#[derive(Serialize, Clone, Debug)]
pub struct DuplicateGroup {
    pub media_item_id: i64,
    pub item_title: String,
    pub kind: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub files: Vec<Episode>,
}

/// Movies with more than one file, and series episodes that exist more than once.
pub fn find_duplicates(conn: &Connection) -> rusqlite::Result<Vec<DuplicateGroup>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.title, m.kind, m.year, e.season, e.episode
         FROM episodes e JOIN media_items m ON m.id = e.media_item_id
         WHERE e.extra IS NULL
         GROUP BY m.id, CASE WHEN m.kind = 'movie' THEN 0 ELSE e.season END,
                        CASE WHEN m.kind = 'movie' THEN 0 ELSE e.episode END
         HAVING COUNT(*) > 1
         ORDER BY m.title COLLATE NOCASE, e.season, e.episode",
    )?;
    let keys: Vec<(i64, String, String, Option<i32>, Option<i32>, Option<i32>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    for (media_item_id, item_title, kind, year, season, episode) in keys {
        let all: Vec<Episode> = list_episodes(conn, media_item_id)?.into_iter().filter(|e| e.extra.is_none()).collect();
        let files: Vec<Episode> = if kind == "movie" {
            all
        } else {
            all.into_iter().filter(|e| e.season == season && e.episode == episode).collect()
        };
        if files.len() > 1 {
            let (season, episode) = if kind == "movie" { (None, None) } else { (season, episode) };
            out.push(DuplicateGroup { media_item_id, item_title, kind, year, season, episode, files });
        }
    }
    Ok(out)
}

pub fn delete_episode(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM episodes WHERE id = ?1", [id])?;
    prune_empty_items(conn)
}

// ---------- tags & collections ----------

#[derive(Serialize, Clone, Debug)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub tmdb_collection_id: Option<i64>,
    pub poster_url: Option<String>,
    pub overview: Option<String>,
    pub item_count: i64,
    pub watched_count: i64,
    /// Local poster of the first item, for collection cards without a TMDB poster.
    pub first_poster: Option<String>,
    pub first_tmdb_id: Option<i64>,
}

pub fn list_tags(conn: &Connection, kind: &str) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.kind, t.tmdb_collection_id, t.poster_url, t.overview,
                (SELECT COUNT(*) FROM item_tags it WHERE it.tag_id = t.id) AS item_count,
                (SELECT COUNT(*) FROM item_tags it WHERE it.tag_id = t.id AND NOT EXISTS (
                    SELECT 1 FROM episodes e LEFT JOIN watch_progress w ON w.episode_id = e.id
                    WHERE e.media_item_id = it.media_item_id AND e.extra IS NULL AND COALESCE(w.completed, 0) = 0)
                ) AS watched_count,
                (SELECT m.poster_path FROM item_tags it JOIN media_items m ON m.id = it.media_item_id
                  WHERE it.tag_id = t.id AND m.poster_path IS NOT NULL ORDER BY it.position, m.year LIMIT 1),
                (SELECT m.tmdb_id FROM item_tags it JOIN media_items m ON m.id = it.media_item_id
                  WHERE it.tag_id = t.id AND m.poster_path IS NOT NULL ORDER BY it.position, m.year LIMIT 1)
         FROM tags t WHERE t.kind = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([kind], |r| {
        Ok(Tag {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
            tmdb_collection_id: r.get(3)?,
            poster_url: r.get(4)?,
            overview: r.get(5)?,
            item_count: r.get(6)?,
            watched_count: r.get(7)?,
            first_poster: r.get(8)?,
            first_tmdb_id: r.get(9)?,
        })
    })?;
    rows.collect()
}

pub fn get_tag(conn: &Connection, id: i64) -> rusqlite::Result<Option<Tag>> {
    let kind: Option<String> = conn.query_row("SELECT kind FROM tags WHERE id = ?1", [id], |r| r.get(0)).optional()?;
    let Some(kind) = kind else { return Ok(None) };
    Ok(list_tags(conn, &kind)?.into_iter().find(|t| t.id == id))
}

/// Create if missing; returns the id either way.
pub fn ensure_tag(conn: &Connection, kind: &str, name: &str) -> rusqlite::Result<i64> {
    let name = name.trim();
    conn.execute("INSERT OR IGNORE INTO tags(name, kind) VALUES (?1, ?2)", params![name, kind])?;
    conn.query_row("SELECT id FROM tags WHERE kind = ?1 AND name = ?2 COLLATE NOCASE", params![kind, name], |r| r.get(0))
}

pub fn rename_tag(conn: &Connection, id: i64, name: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE tags SET name = ?2 WHERE id = ?1", params![id, name.trim()])?;
    Ok(())
}

pub fn delete_tag(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}

pub fn add_item_tag(conn: &Connection, tag_id: i64, media_item_id: i64) -> rusqlite::Result<()> {
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM item_tags WHERE tag_id = ?1",
        [tag_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO item_tags(tag_id, media_item_id, position) VALUES (?1, ?2, ?3)",
        params![tag_id, media_item_id, next],
    )?;
    Ok(())
}

pub fn remove_item_tag(conn: &Connection, tag_id: i64, media_item_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM item_tags WHERE tag_id = ?1 AND media_item_id = ?2", params![tag_id, media_item_id])?;
    Ok(())
}

/// Replace a title's plain tags with exactly this set of names.
pub fn set_item_tags(conn: &Connection, media_item_id: i64, names: &[String]) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM item_tags WHERE media_item_id = ?1 AND tag_id IN (SELECT id FROM tags WHERE kind = 'tag')",
        [media_item_id],
    )?;
    for n in names {
        if n.trim().is_empty() {
            continue;
        }
        let id = ensure_tag(conn, "tag", n)?;
        add_item_tag(conn, id, media_item_id)?;
    }
    // Drop tags nobody uses any more.
    conn.execute("DELETE FROM tags WHERE kind = 'tag' AND id NOT IN (SELECT tag_id FROM item_tags)", [])?;
    Ok(())
}

pub fn set_collection_order(conn: &Connection, tag_id: i64, ordered_item_ids: &[i64]) -> rusqlite::Result<()> {
    for (i, item) in ordered_item_ids.iter().enumerate() {
        conn.execute(
            "UPDATE item_tags SET position = ?3 WHERE tag_id = ?1 AND media_item_id = ?2",
            params![tag_id, item, i as i64],
        )?;
    }
    Ok(())
}

pub fn collection_items(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<MediaItem>> {
    let mut stmt = conn.prepare("SELECT media_item_id FROM item_tags WHERE tag_id = ?1 ORDER BY position, media_item_id")?;
    let ids: Vec<i64> = stmt.query_map([tag_id], |r| r.get(0))?.collect::<Result<_, _>>()?;
    let all = list_media(conn, None)?;
    Ok(ids.into_iter().filter_map(|id| all.iter().find(|m| m.id == id).cloned()).collect())
}

/// Attach a movie to its TMDB collection, creating the collection on first
/// sight and keeping members in release order.
pub fn attach_tmdb_collection(
    conn: &Connection,
    media_item_id: i64,
    tmdb_collection_id: i64,
    name: &str,
    poster_url: Option<&str>,
) -> rusqlite::Result<i64> {
    let existing: Option<i64> = conn
        .query_row("SELECT id FROM tags WHERE tmdb_collection_id = ?1", [tmdb_collection_id], |r| r.get(0))
        .optional()?;
    let tag_id = match existing {
        Some(id) => id,
        None => {
            let id = ensure_tag(conn, "collection", name)?;
            conn.execute(
                "UPDATE tags SET tmdb_collection_id = ?2, poster_url = COALESCE(poster_url, ?3) WHERE id = ?1",
                params![id, tmdb_collection_id, poster_url],
            )?;
            id
        }
    };
    add_item_tag(conn, tag_id, media_item_id)?;
    // Re-sort by year so "Alien" precedes "Aliens" regardless of scan order.
    let mut stmt = conn.prepare(
        "SELECT it.media_item_id FROM item_tags it JOIN media_items m ON m.id = it.media_item_id
         WHERE it.tag_id = ?1 ORDER BY m.year, m.title COLLATE NOCASE",
    )?;
    let ordered: Vec<i64> = stmt.query_map([tag_id], |r| r.get(0))?.collect::<Result<_, _>>()?;
    set_collection_order(conn, tag_id, &ordered)?;
    Ok(tag_id)
}

/// Titles whose cached TMDB details mention a collection but which are not in one yet.
pub fn backfill_tmdb_collections(conn: &Connection) -> rusqlite::Result<usize> {
    let mut stmt = conn.prepare(
        "SELECT d.media_item_id,
                json_extract(d.json, '$.belongs_to_collection.id'),
                json_extract(d.json, '$.belongs_to_collection.name'),
                json_extract(d.json, '$.belongs_to_collection.poster_path')
         FROM tmdb_details d
         WHERE json_extract(d.json, '$.belongs_to_collection.id') IS NOT NULL
           AND d.media_item_id NOT IN (
                SELECT it.media_item_id FROM item_tags it JOIN tags t ON t.id = it.tag_id WHERE t.kind = 'collection')",
    )?;
    let rows: Vec<(i64, i64, String, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let n = rows.len();
    for (item, cid, name, poster) in rows {
        let url = poster.map(|p| format!("https://image.tmdb.org/t/p/w342{p}"));
        attach_tmdb_collection(conn, item, cid, &name, url.as_deref())?;
    }
    Ok(n)
}

// ---------- search ----------

#[derive(Serialize, Clone, Debug)]
pub struct EpisodeHit {
    pub episode: Episode,
    pub item_title: String,
    pub kind: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct SearchResults {
    pub items: Vec<MediaItem>,
    pub episodes: Vec<EpisodeHit>,
}

pub fn search(conn: &Connection, query: &str) -> rusqlite::Result<SearchResults> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(SearchResults::default());
    }
    let like = format!("%{}%", q.replace('%', "").replace('_', " "));
    let ql = q.to_lowercase();
    let mut items: Vec<MediaItem> =
        list_media(conn, None)?.into_iter().filter(|m| m.title.to_lowercase().contains(&ql)).collect();
    items.truncate(30);

    let sql = format!(
        "{EPISODE_SELECT} JOIN media_items m ON m.id = e.media_item_id
         WHERE e.title LIKE ?1 OR e.file_name LIKE ?1
         ORDER BY m.title COLLATE NOCASE, e.season, e.episode LIMIT 40"
    );
    let mut stmt = conn.prepare(&sql)?;
    let eps: Vec<Episode> = stmt.query_map([&like], episode_from_row)?.collect::<Result<_, _>>()?;
    let mut episodes = Vec::with_capacity(eps.len());
    for ep in eps {
        let (item_title, kind): (String, String) = conn.query_row(
            "SELECT title, kind FROM media_items WHERE id = ?1",
            [ep.media_item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        episodes.push(EpisodeHit { episode: ep, item_title, kind });
    }
    Ok(SearchResults { items, episodes })
}

// ---------- browsing ----------

pub fn list_media(conn: &Connection, kind: Option<&str>) -> rusqlite::Result<Vec<MediaItem>> {
    let sql = r#"
        SELECT m.id, m.kind, m.title, m.year, m.category, m.tmdb_id, m.poster_path, m.overview, m.rating, m.genres,
               (SELECT group_concat(t.name, ', ') FROM item_tags it JOIN tags t ON t.id = it.tag_id
                 WHERE it.media_item_id = m.id AND t.kind = 'tag') AS tags,
               (SELECT group_concat(t.name, ', ') FROM item_tags it JOIN tags t ON t.id = it.tag_id
                 WHERE it.media_item_id = m.id AND t.kind = 'collection') AS collections,
               SUM(CASE WHEN e.extra IS NULL THEN 1 ELSE 0 END) AS episode_count,
               SUM(CASE WHEN e.extra IS NULL THEN COALESCE(w.completed, 0) ELSE 0 END) AS watched_count,
               MAX(w.last_watched) AS last_watched,
               MAX(e.added_at) AS added_at,
               SUM(e.size) AS total_size
        FROM media_items m
        JOIN episodes e ON e.media_item_id = m.id
        LEFT JOIN watch_progress w ON w.episode_id = e.id
        WHERE (?1 IS NULL OR m.kind = ?1)
        GROUP BY m.id
        ORDER BY m.title COLLATE NOCASE
    "#;
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([kind], |r| {
        Ok(MediaItem {
            id: r.get(0)?,
            kind: r.get(1)?,
            title: r.get(2)?,
            year: r.get(3)?,
            category: r.get(4)?,
            tmdb_id: r.get(5)?,
            poster_path: r.get(6)?,
            overview: r.get(7)?,
            rating: r.get(8)?,
            genres: r.get(9)?,
            tags: r.get(10)?,
            collections: r.get(11)?,
            episode_count: r.get::<_, Option<i64>>(12)?.unwrap_or(0),
            watched_count: r.get::<_, Option<i64>>(13)?.unwrap_or(0),
            last_watched: r.get(14)?,
            added_at: r.get(15)?,
            total_size: r.get::<_, Option<i64>>(16)?.unwrap_or(0),
        })
    })?;
    rows.collect()
}

pub fn list_categories(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT category FROM media_items WHERE category IS NOT NULL ORDER BY category COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    rows.collect()
}

pub fn set_item_category(conn: &Connection, media_item_id: i64, category: Option<&str>) -> rusqlite::Result<()> {
    conn.execute("UPDATE media_items SET category = ?2 WHERE id = ?1", params![media_item_id, category])?;
    Ok(())
}

pub fn items_missing_poster(conn: &Connection, include_checked: bool) -> rusqlite::Result<Vec<MediaItem>> {
    Ok(list_media(conn, None)?
        .into_iter()
        .filter(|m| m.poster_path.is_none())
        .filter(|m| include_checked || !poster_checked(conn, m.id).unwrap_or(false))
        .collect())
}

fn poster_checked(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    conn.query_row("SELECT poster_checked FROM media_items WHERE id = ?1", [id], |r| r.get::<_, i64>(0))
        .map(|v| v != 0)
}

pub fn mark_poster_checked(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("UPDATE media_items SET poster_checked = 1 WHERE id = ?1", [id])?;
    Ok(())
}

pub fn set_tmdb(
    conn: &Connection,
    id: i64,
    tmdb_id: Option<i64>,
    poster_path: Option<&str>,
    overview: Option<&str>,
    rating: Option<f64>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE media_items SET tmdb_id = ?2, poster_path = ?3, overview = ?4, rating = ?5, poster_checked = 1 WHERE id = ?1",
        params![id, tmdb_id, poster_path, overview, rating],
    )?;
    Ok(())
}

pub fn set_item_rating(conn: &Connection, id: i64, rating: Option<f64>) -> rusqlite::Result<()> {
    conn.execute("UPDATE media_items SET rating = ?2 WHERE id = ?1", params![id, rating])?;
    Ok(())
}

pub fn set_item_genres(conn: &Connection, id: i64, genres: Option<&str>) -> rusqlite::Result<()> {
    conn.execute("UPDATE media_items SET genres = ?2 WHERE id = ?1", params![id, genres])?;
    Ok(())
}

pub fn get_media_item(conn: &Connection, id: i64) -> rusqlite::Result<Option<MediaItem>> {
    let items = list_media(conn, None)?;
    Ok(items.into_iter().find(|m| m.id == id))
}

pub fn list_episodes(conn: &Connection, media_item_id: i64) -> rusqlite::Result<Vec<Episode>> {
    let sql = format!(
        "{EPISODE_SELECT} WHERE e.media_item_id = ?1 ORDER BY e.season, e.episode, e.file_name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([media_item_id], episode_from_row)?;
    rows.collect()
}

pub fn get_episode(conn: &Connection, id: i64) -> rusqlite::Result<Option<Episode>> {
    let sql = format!("{EPISODE_SELECT} WHERE e.id = ?1");
    conn.query_row(&sql, [id], episode_from_row).optional()
}

/// The episode that follows this one in season/episode order (series only).
pub fn next_episode(conn: &Connection, episode_id: i64) -> rusqlite::Result<Option<Episode>> {
    let Some(ep) = get_episode(conn, episode_id)? else { return Ok(None) };
    let kind: String = conn.query_row("SELECT kind FROM media_items WHERE id = ?1", [ep.media_item_id], |r| r.get(0))?;
    if kind != "series" {
        return Ok(None);
    }
    if ep.extra.is_some() {
        return Ok(None);
    }
    let all: Vec<Episode> = list_episodes(conn, ep.media_item_id)?.into_iter().filter(|e| e.extra.is_none()).collect();
    let pos = all.iter().position(|e| e.id == ep.id).unwrap_or(all.len());
    Ok(all.into_iter().skip(pos + 1).find(|e| e.available))
}

/// Items the user is part-way through, plus the "next up" episode for series
/// where the last watched episode was completed.
pub fn continue_watching(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<ContinueItem>> {
    // Most recently touched episode per media item.
    let sql = format!(
        r#"{EPISODE_SELECT}
        WHERE w.last_watched = (
            SELECT MAX(w2.last_watched) FROM watch_progress w2
            JOIN episodes e2 ON e2.id = w2.episode_id
            WHERE e2.media_item_id = e.media_item_id
        )
        ORDER BY w.last_watched DESC, e.season DESC, e.episode DESC LIMIT ?1"#
    );
    let mut stmt = conn.prepare(&sql)?;
    let recent: Vec<Episode> = stmt.query_map([limit], episode_from_row)?.collect::<Result<_, _>>()?;

    // Episodes dismissed from the Home page with the X button.
    let mut hidden_stmt = conn.prepare("SELECT episode_id FROM watch_progress WHERE hidden = 1")?;
    let hidden: std::collections::HashSet<i64> =
        hidden_stmt.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    drop(hidden_stmt);
    let mut out = Vec::new();
    let mut seen_items = std::collections::HashSet::new();
    for ep in recent {
        if !seen_items.insert(ep.media_item_id) {
            continue; // ties on last_watched: keep only the first per item
        }
        if hidden.contains(&ep.id) {
            continue;
        }
        let (title, kind, year, poster_path, tmdb_id): (String, String, Option<i32>, Option<String>, Option<i64>) =
            conn.query_row(
                "SELECT title, kind, year, poster_path, tmdb_id FROM media_items WHERE id = ?1",
                [ep.media_item_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )?;
        if !ep.completed {
            out.push(ContinueItem { episode: ep, title, kind, year, poster_path, tmdb_id });
            continue;
        }
        if kind == "movie" {
            continue;
        }
        // Completed: find the next unwatched episode after it in the series.
        if ep.extra.is_some() {
            continue;
        }
        let all: Vec<Episode> = list_episodes(conn, ep.media_item_id)?.into_iter().filter(|x| x.extra.is_none()).collect();
        let pos = all.iter().position(|x| x.id == ep.id).unwrap_or(0);
        if let Some(next) = all.iter().skip(pos + 1).find(|x| !x.completed) {
            out.push(ContinueItem { episode: next.clone(), title, kind, year, poster_path, tmdb_id });
        }
    }
    Ok(out)
}

// ---------- progress ----------

pub fn set_progress(conn: &Connection, episode_id: i64, position_secs: i64, completed: bool) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO watch_progress(episode_id, position_secs, completed, last_watched)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(episode_id) DO UPDATE SET position_secs = excluded.position_secs,
            completed = excluded.completed, last_watched = excluded.last_watched, hidden = 0",
        params![episode_id, position_secs, completed as i64],
    )?;
    Ok(())
}

pub fn touch_progress(conn: &Connection, episode_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO watch_progress(episode_id, last_watched) VALUES (?1, datetime('now'))
         ON CONFLICT(episode_id) DO UPDATE SET last_watched = excluded.last_watched, hidden = 0",
        [episode_id],
    )?;
    Ok(())
}

/// Hide one Continue Watching entry. Progress is untouched; playing the title
/// again brings it back.
pub fn hide_from_home(conn: &Connection, episode_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO watch_progress(episode_id, hidden) VALUES (?1, 1)
         ON CONFLICT(episode_id) DO UPDATE SET hidden = 1",
        [episode_id],
    )?;
    Ok(())
}

pub fn hide_all_from_home(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute("UPDATE watch_progress SET hidden = 1 WHERE hidden = 0", [])
}

/// Wipe watched marks, paused positions and the history log. Library, posters
/// and settings stay.
pub fn reset_watch_data(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM watch_progress", [])?;
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

pub fn clear_progress(conn: &Connection, episode_id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM watch_progress WHERE episode_id = ?1", [episode_id])?;
    Ok(())
}

pub fn set_item_watched(conn: &Connection, media_item_id: i64, watched: bool) -> rusqlite::Result<()> {
    if watched {
        conn.execute(
            "INSERT INTO watch_progress(episode_id, position_secs, completed, last_watched)
             SELECT id, 0, 1, datetime('now') FROM episodes WHERE media_item_id = ?1
             ON CONFLICT(episode_id) DO UPDATE SET completed = 1",
            [media_item_id],
        )?;
    } else {
        conn.execute(
            "DELETE FROM watch_progress WHERE episode_id IN (SELECT id FROM episodes WHERE media_item_id = ?1)",
            [media_item_id],
        )?;
    }
    Ok(())
}

// ---------- settings ----------

pub fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn all_settings(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
    rows.collect()
}
