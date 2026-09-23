use crate::db::{
    self, ContinueItem, DuplicateGroup, Episode, HistoryEntry, HistoryStats, Library, MediaItem, SearchResults, Tag,
};
use crate::player::{self, DetectedPlayer};
use crate::probe;
use crate::backup::{self, BackupInfo};
use crate::scanner::{self, ScanStats};
use crate::tmdb::{self, Details, TmdbMatch};
use crate::torrent;
use crate::AppState;
use std::collections::HashMap;
use tauri::{AppHandle, Manager, State};

type R<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

// ---- libraries ----

#[tauri::command]
pub fn list_libraries(state: State<AppState>) -> R<Vec<Library>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_libraries(&conn).map_err(err)
}

#[tauri::command]
pub fn add_library(state: State<AppState>, path: String) -> R<Library> {
    let conn = state.db.lock().map_err(err)?;
    db::add_library(&conn, &path).map_err(err)
}

#[tauri::command]
pub fn remove_library(state: State<AppState>, id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::remove_library(&conn, id).map_err(err)
}

#[tauri::command]
pub async fn scan_libraries(app: AppHandle) -> R<ScanStats> {
    // Own connection, not the shared one: holding that across a scan blocks
    // every synchronous command, and those run on the main thread.
    let stats = blocking(move || {
        let mut conn = db::open(&app.state::<AppState>().db_path).map_err(err)?;
        let stats = scanner::scan_all(&mut conn)?;
        crate::tray::rebuild(&app);
        Ok(stats)
    })
    .await?;
    Ok(stats)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::logging::closing("quit from the app");
    app.exit(0);
}

/// Show the session log in Explorer, for sending on when something breaks.
#[tauri::command]
pub fn reveal_log(app: AppHandle) -> R<()> {
    let dir = app.path().app_data_dir().map_err(err)?;
    reveal_path(dir.join("vortex.log").to_string_lossy().to_string())
}

// ---- browsing ----

#[tauri::command]
pub fn list_media(state: State<AppState>, kind: Option<String>) -> R<Vec<MediaItem>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_media(&conn, kind.as_deref()).map_err(err)
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> R<Vec<String>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_categories(&conn).map_err(err)
}

#[tauri::command]
pub fn set_item_category(state: State<AppState>, media_item_id: i64, category: Option<String>) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    let c = category.as_deref().map(str::trim).filter(|c| !c.is_empty());
    db::set_item_category(&conn, media_item_id, c).map_err(err)
}

#[tauri::command]
pub fn get_media_item(state: State<AppState>, id: i64) -> R<Option<MediaItem>> {
    let conn = state.db.lock().map_err(err)?;
    db::get_media_item(&conn, id).map_err(err)
}

#[tauri::command]
pub fn list_episodes(state: State<AppState>, media_item_id: i64) -> R<Vec<Episode>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_episodes(&conn, media_item_id).map_err(err)
}

#[tauri::command]
pub fn continue_watching(state: State<AppState>) -> R<Vec<ContinueItem>> {
    let conn = state.db.lock().map_err(err)?;
    db::continue_watching(&conn, 20).map_err(err)
}

// ---- progress ----

#[tauri::command]
pub fn set_progress(state: State<AppState>, episode_id: i64, position_secs: i64, completed: bool) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_progress(&conn, episode_id, position_secs, completed).map_err(err)?;
    if completed {
        db::add_history(&conn, episode_id, position_secs, true, false, "manual").map_err(err)?;
    }
    Ok(())
}

// ---- history ----

#[tauri::command]
pub fn list_history(state: State<AppState>, limit: Option<i64>) -> R<Vec<HistoryEntry>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_history(&conn, limit.unwrap_or(300)).map_err(err)
}

#[tauri::command]
pub fn history_stats(state: State<AppState>) -> R<HistoryStats> {
    let conn = state.db.lock().map_err(err)?;
    db::history_stats(&conn).map_err(err)
}

#[tauri::command]
pub fn delete_history(state: State<AppState>, id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::delete_history(&conn, id).map_err(err)
}

#[tauri::command]
pub fn clear_history(state: State<AppState>) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::clear_history(&conn).map_err(err)
}

// ---- duplicates ----

#[tauri::command]
pub fn find_duplicates(state: State<AppState>) -> R<Vec<DuplicateGroup>> {
    let conn = state.db.lock().map_err(err)?;
    db::find_duplicates(&conn).map_err(err)
}

/// Move a file to the Recycle Bin and drop it from the library.
#[tauri::command]
pub fn trash_episode(state: State<AppState>, episode_id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    let ep = db::get_episode(&conn, episode_id).map_err(err)?.ok_or("episode not found")?;
    if std::path::Path::new(&ep.path).exists() {
        trash::delete(&ep.path).map_err(|e| format!("could not move to Recycle Bin: {e}"))?;
    }
    db::delete_episode(&conn, episode_id).map_err(err)
}

#[tauri::command]
pub fn hide_from_home(app: AppHandle, state: State<AppState>, episode_id: i64) -> R<()> {
    {
        let conn = state.db.lock().map_err(err)?;
        db::hide_from_home(&conn, episode_id).map_err(err)?;
    }
    crate::tray::rebuild(&app);
    Ok(())
}

#[tauri::command]
pub fn hide_all_from_home(app: AppHandle, state: State<AppState>) -> R<usize> {
    let n = {
        let conn = state.db.lock().map_err(err)?;
        db::hide_all_from_home(&conn).map_err(err)?
    };
    crate::tray::rebuild(&app);
    Ok(n)
}

#[tauri::command]
pub fn reset_watch_data(app: AppHandle, state: State<AppState>) -> R<()> {
    {
        let conn = state.db.lock().map_err(err)?;
        db::reset_watch_data(&conn).map_err(err)?;
    }
    crate::tray::rebuild(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_progress(state: State<AppState>, episode_id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::clear_progress(&conn, episode_id).map_err(err)
}

#[tauri::command]
pub fn set_item_watched(state: State<AppState>, media_item_id: i64, watched: bool) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_item_watched(&conn, media_item_id, watched).map_err(err)
}

// ---- playback ----

#[tauri::command]
pub fn play_episode(app: AppHandle, episode_id: i64) -> R<bool> {
    player::play(app, episode_id)
}

/// Open Windows Explorer with the file selected.
#[tauri::command]
pub fn reveal_path(path: String) -> R<()> {
    if !std::path::Path::new(&path).exists() {
        return Err("File is not available. Is the drive connected?".into());
    }
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(err)
}

#[tauri::command]
pub fn detect_players() -> Vec<DetectedPlayer> {
    player::detect()
}

// ---- durations ----

#[tauri::command]
pub fn probe_durations(app: AppHandle) -> R<()> {
    probe::probe_missing(app)
}

#[tauri::command]
pub fn detect_ffprobe() -> Option<String> {
    probe::detect_ffprobe()
}

// ---- TMDB ----

#[tauri::command]
pub fn fetch_posters(app: AppHandle, force: bool) -> R<()> {
    tmdb::fetch_missing(app, force)
}

/// The TMDB client is blocking; it must never run (or be dropped) on the async
/// runtime's worker threads, so every network command hops to a blocking thread.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> R<T> + Send + 'static) -> R<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| format!("task failed: {e}"))?
}

#[tauri::command]
pub async fn search_tmdb(app: AppHandle, kind: String, query: String, year: Option<i32>) -> R<Vec<TmdbMatch>> {
    blocking(move || {
        let key = {
            let state = app.state::<AppState>();
            let conn = state.db.lock().map_err(err)?;
            db::get_setting(&conn, "tmdb_key").map_err(err)?.unwrap_or_default()
        };
        if key.trim().is_empty() {
            return Err("Add your TMDB API key in Settings first".into());
        }
        tmdb::search(&key, &kind, &query, year)
    })
    .await
}

#[tauri::command]
pub async fn apply_tmdb_match(app: AppHandle, media_item_id: i64, m: TmdbMatch) -> R<()> {
    blocking(move || tmdb::apply_match(&app, media_item_id, &m)).await
}

#[tauri::command]
pub async fn get_details(app: AppHandle, media_item_id: i64, refresh: bool) -> R<Option<Details>> {
    blocking(move || tmdb::get_details(&app, media_item_id, refresh)).await
}

#[tauri::command]
pub async fn fetch_episode_titles(app: AppHandle, media_item_id: i64) -> R<usize> {
    blocking(move || tmdb::fetch_episode_titles(&app, media_item_id)).await
}

#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> R<SearchResults> {
    let conn = state.db.lock().map_err(err)?;
    db::search(&conn, &query).map_err(err)
}

#[tauri::command]
pub async fn test_tmdb_key(key: String) -> R<String> {
    blocking(move || {
        let results = tmdb::search(&key, "movie", "Inception", Some(2010))?;
        Ok(format!("Key works ({} results for a test search)", results.len()))
    })
    .await
}

/// Verify the key against TMDB and store it only if it works.
#[tauri::command]
pub async fn connect_tmdb(app: AppHandle, key: String) -> R<()> {
    blocking(move || {
        let key = key.trim().to_string();
        if key.is_empty() {
            return Err("Enter a key first".into());
        }
        tmdb::search(&key, "movie", "Inception", Some(2010))?;
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(err)?;
        db::set_setting(&conn, "tmdb_key", &key).map_err(err)?;
        db::set_setting(&conn, "tmdb_connected", "1").map_err(err)
    })
    .await
}

#[tauri::command]
pub fn disconnect_tmdb(state: State<AppState>) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_setting(&conn, "tmdb_key", "").map_err(err)?;
    db::set_setting(&conn, "tmdb_connected", "0").map_err(err)
}

/// Settings for the UI: the TMDB key itself never leaves the backend.
#[tauri::command]
pub fn get_tmdb_status(state: State<AppState>) -> R<Option<String>> {
    let conn = state.db.lock().map_err(err)?;
    let key = db::get_setting(&conn, "tmdb_key").map_err(err)?.unwrap_or_default();
    let connected = db::get_setting(&conn, "tmdb_connected").map_err(err)?.as_deref() == Some("1");
    if key.is_empty() || !connected {
        return Ok(None);
    }
    let tail: String = key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
    Ok(Some(format!("••••{tail}")))
}

// ---- tags & collections ----

#[tauri::command]
pub fn list_tags(state: State<AppState>, kind: String) -> R<Vec<Tag>> {
    let conn = state.db.lock().map_err(err)?;
    db::list_tags(&conn, &kind).map_err(err)
}

#[tauri::command]
pub fn get_tag(state: State<AppState>, id: i64) -> R<Option<Tag>> {
    let conn = state.db.lock().map_err(err)?;
    db::get_tag(&conn, id).map_err(err)
}

#[tauri::command]
pub fn create_tag(state: State<AppState>, kind: String, name: String) -> R<i64> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    let conn = state.db.lock().map_err(err)?;
    db::ensure_tag(&conn, &kind, &name).map_err(err)
}

#[tauri::command]
pub fn rename_tag(state: State<AppState>, id: i64, name: String) -> R<()> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    let conn = state.db.lock().map_err(err)?;
    db::rename_tag(&conn, id, &name).map_err(err)
}

#[tauri::command]
pub fn delete_tag(state: State<AppState>, id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::delete_tag(&conn, id).map_err(err)
}

#[tauri::command]
pub fn set_item_tags(state: State<AppState>, media_item_id: i64, names: Vec<String>) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_item_tags(&conn, media_item_id, &names).map_err(err)
}

#[tauri::command]
pub fn add_to_collection(state: State<AppState>, tag_id: i64, media_item_id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::add_item_tag(&conn, tag_id, media_item_id).map_err(err)
}

#[tauri::command]
pub fn remove_from_collection(state: State<AppState>, tag_id: i64, media_item_id: i64) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::remove_item_tag(&conn, tag_id, media_item_id).map_err(err)
}

#[tauri::command]
pub fn set_collection_order(state: State<AppState>, tag_id: i64, item_ids: Vec<i64>) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_collection_order(&conn, tag_id, &item_ids).map_err(err)
}

#[tauri::command]
pub fn collection_items(state: State<AppState>, tag_id: i64) -> R<Vec<MediaItem>> {
    let conn = state.db.lock().map_err(err)?;
    db::collection_items(&conn, tag_id).map_err(err)
}

#[tauri::command]
pub fn stats(state: State<AppState>) -> R<db::Stats> {
    let conn = state.db.lock().map_err(err)?;
    db::stats(&conn).map_err(err)
}

// ---- drives ----

#[derive(serde::Serialize)]
pub struct Drive {
    pub path: String,
    pub name: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub removable: bool,
}

#[tauri::command]
pub fn list_drives() -> Vec<Drive> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut out: Vec<Drive> = disks
        .iter()
        .map(|d| Drive {
            path: d.mount_point().to_string_lossy().to_string(),
            name: d.name().to_string_lossy().to_string(),
            total_bytes: d.total_space(),
            free_bytes: d.available_space(),
            removable: d.is_removable(),
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

#[tauri::command]
pub fn default_ignored_dirs() -> Vec<String> {
    scanner::DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect()
}

// ---- backup ----

#[tauri::command]
pub async fn create_backup(app: AppHandle, path: String) -> R<BackupInfo> {
    blocking(move || backup::create(&app, std::path::Path::new(&path))).await
}

#[tauri::command]
pub async fn restore_backup(app: AppHandle, path: String) -> R<()> {
    blocking(move || backup::restore(&app, std::path::Path::new(&path))).await
}

// ---- settings ----

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> R<HashMap<String, String>> {
    let conn = state.db.lock().map_err(err)?;
    let mut map: HashMap<String, String> = db::all_settings(&conn).map_err(err)?.into_iter().collect();
    // Never send the raw key to the UI; it only needs to know whether one exists.
    if let Some(k) = map.get_mut("tmdb_key") {
        *k = if k.is_empty() { String::new() } else { "set".into() };
    }
    Ok(map)
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> R<()> {
    let conn = state.db.lock().map_err(err)?;
    db::set_setting(&conn, &key, &value).map_err(err)
}

// ---- torrents ----

/// Starts the engine when a download folder is configured, so the Downloads
/// page shows live state as soon as it opens.
#[tauri::command]
pub async fn torrent_status(app: AppHandle) -> R<torrent::Status> {
    blocking(move || {
        let error = if torrent::read_config(&app).dir.is_empty() { None } else { torrent::ensure(&app).err() };
        Ok(torrent::status(&app, error))
    })
    .await
}

#[tauri::command]
pub async fn torrent_restart(app: AppHandle) -> R<torrent::Status> {
    blocking(move || torrent::restart(&app)).await
}

#[tauri::command]
pub async fn torrent_inspect(app: AppHandle, source: String) -> R<torrent::Preview> {
    blocking(move || torrent::ensure(&app)?.inspect(&source)).await
}

#[tauri::command]
pub async fn torrent_add(app: AppHandle, source: String, files: Vec<usize>, needed_bytes: u64, save_in: String, subfolder: Option<String>) -> R<usize> {
    blocking(move || torrent::ensure(&app)?.add(&app, &source, files, needed_bytes, torrent::Dest { save_in, subfolder, ..Default::default() })).await
}

#[tauri::command]
pub async fn torrent_list(app: AppHandle) -> R<Vec<torrent::TorrentRow>> {
    blocking(move || Ok(torrent::current(&app).map(|e| e.list()).unwrap_or_default())).await
}

#[tauri::command]
pub async fn torrent_pause(app: AppHandle, id: usize) -> R<()> {
    blocking(move || torrent::ensure(&app)?.pause(id)).await
}

#[tauri::command]
pub async fn torrent_resume(app: AppHandle, id: usize) -> R<()> {
    blocking(move || torrent::ensure(&app)?.resume(id)).await
}

#[tauri::command]
pub async fn torrent_remove(app: AppHandle, id: usize, delete_files: bool) -> R<()> {
    blocking(move || torrent::ensure(&app)?.remove(id, delete_files)).await
}

/// Stream a file from a running torrent in the configured player.
#[tauri::command]
pub async fn torrent_play(app: AppHandle, id: usize, file: usize) -> R<bool> {
    blocking(move || {
        let url = torrent::ensure(&app)?.stream_url(id, file)?;
        player::play_url(&app, &url)
    })
    .await
}

#[tauri::command]
pub async fn torrent_detail(app: AppHandle, id: usize) -> R<torrent::TorrentDetail> {
    blocking(move || torrent::ensure(&app)?.detail(id)).await
}

#[tauri::command]
pub async fn torrent_session_status(app: AppHandle) -> R<Option<torrent::SessionStatus>> {
    blocking(move || Ok(torrent::current(&app).map(|e| e.session_status()))).await
}

/// One-step stream: resolve, pick the largest video, buffer, play. Blocks
/// while buffering (up to 90 s), so the UI shows a spinner meanwhile.
#[tauri::command]
pub async fn torrent_stream(app: AppHandle, source: String) -> R<torrent::StreamStarted> {
    blocking(move || torrent::ensure(&app)?.stream(&app, &source)).await
}

/// Stream a torrent already in the list (buffers first, resumes position).
#[tauri::command]
pub async fn torrent_stream_existing(app: AppHandle, id: usize) -> R<torrent::StreamStarted> {
    blocking(move || torrent::ensure(&app)?.stream_existing(&app, id)).await
}

#[tauri::command]
pub async fn torrent_keep(app: AppHandle, id: usize) -> R<()> {
    blocking(move || torrent::ensure(&app)?.keep(&app, id)).await
}

#[tauri::command]
pub async fn torrent_discard(app: AppHandle, id: usize) -> R<()> {
    blocking(move || torrent::ensure(&app)?.discard(id)).await
}

/// Register or release the `magnet:` link association for Vortex.
#[tauri::command]
pub fn set_magnet_handler(app: AppHandle, state: State<AppState>, on: bool) -> R<()> {
    {
        let conn = state.db.lock().map_err(err)?;
        db::set_setting(&conn, "magnet_handler", if on { "1" } else { "0" }).map_err(err)?;
    }
    crate::deeplink::set_magnet_handler(&app, on)
}

/// Magnet links that opened the app before the Downloads page was listening.
#[tauri::command]
pub fn pending_open_urls(app: AppHandle) -> Vec<String> {
    crate::deeplink::take_pending(&app)
}

/// Bridge from the WebView into the Rust log. The backend cannot see the
/// renderer, and that is exactly where slow commands, uncaught errors and
/// dropped frames show up, so the UI reports them here.
#[tauri::command]
pub fn log_frontend(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "ui", "{message}"),
        "warn" => tracing::warn!(target: "ui", "{message}"),
        _ => tracing::info!(target: "ui", "{message}"),
    }
}
