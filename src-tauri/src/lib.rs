mod backup;
mod commands;
mod db;
mod jobs;
mod parser;
mod player;
mod probe;
mod scanner;
mod tmdb;
mod tray;
mod watcher;

use rusqlite::Connection;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{Manager, WindowEvent};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub fetching: AtomicBool,
    pub probing: AtomicBool,
    pub scanning: AtomicBool,
    pub rescan_wanted: AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("vortex.db"))?;
            // Keys saved before the connected flag existed were verified with "Test & save".
            let has_key = db::get_setting(&conn, "tmdb_key")?.map(|k| !k.is_empty()).unwrap_or(false);
            if has_key && db::get_setting(&conn, "tmdb_connected")?.is_none() {
                db::set_setting(&conn, "tmdb_connected", "1")?;
            }
            // Movies matched before collections existed: group them from cached details.
            let _ = db::backfill_tmdb_collections(&conn);
            // First run: pick the first detected player so tracking works out of the box.
            let configured = db::get_setting(&conn, "player_path")?.map(|p| !p.is_empty()).unwrap_or(false);
            if !configured {
                if let Some(p) = player::detect().into_iter().next() {
                    db::set_setting(&conn, "player_kind", &p.kind)?;
                    db::set_setting(&conn, "player_path", &p.path)?;
                }
            }
            app.manage(AppState {
                db: Mutex::new(conn),
                fetching: AtomicBool::new(false),
                probing: AtomicBool::new(false),
                scanning: AtomicBool::new(false),
                rescan_wanted: AtomicBool::new(false),
            });

            let handle = app.handle().clone();
            tray::build(&handle)?;
            if jobs::setting_on(&handle, "rescan_on_startup", true) {
                jobs::refresh_async(handle.clone(), "startup");
            } else {
                let _ = probe::probe_missing(handle.clone());
            }
            watcher::start(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if jobs::setting_on(window.app_handle(), "close_to_tray", true) {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_libraries,
            commands::add_library,
            commands::remove_library,
            commands::scan_libraries,
            commands::list_media,
            commands::get_media_item,
            commands::list_categories,
            commands::set_item_category,
            commands::list_episodes,
            commands::continue_watching,
            commands::set_progress,
            commands::clear_progress,
            commands::hide_from_home,
            commands::hide_all_from_home,
            commands::reset_watch_data,
            commands::set_item_watched,
            commands::play_episode,
            commands::detect_players,
            commands::reveal_path,
            commands::probe_durations,
            commands::detect_ffprobe,
            commands::fetch_posters,
            commands::search_tmdb,
            commands::apply_tmdb_match,
            commands::test_tmdb_key,
            commands::connect_tmdb,
            commands::disconnect_tmdb,
            commands::get_tmdb_status,
            commands::fetch_episode_titles,
            commands::get_details,
            commands::search,
            commands::list_history,
            commands::history_stats,
            commands::delete_history,
            commands::clear_history,
            commands::find_duplicates,
            commands::trash_episode,
            commands::list_tags,
            commands::get_tag,
            commands::create_tag,
            commands::rename_tag,
            commands::delete_tag,
            commands::set_item_tags,
            commands::add_to_collection,
            commands::remove_from_collection,
            commands::set_collection_order,
            commands::collection_items,
            commands::list_drives,
            commands::stats,
            commands::default_ignored_dirs,
            commands::create_backup,
            commands::restore_backup,
            commands::get_settings,
            commands::set_setting,
            commands::quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
