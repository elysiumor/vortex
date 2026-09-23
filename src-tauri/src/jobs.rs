//! Background maintenance: scan, durations, posters, tray refresh. Used by
//! startup, the folder watcher, the tray menu and the Settings button.

use crate::{db, probe, scanner, tmdb, tray, AppState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone)]
pub struct ScanDone {
    pub reason: String,
    pub stats: scanner::ScanStats,
}

pub fn setting_on(app: &AppHandle, key: &str, default: bool) -> bool {
    let state = app.state::<AppState>();
    let guard = state.db.lock();
    match guard {
        Ok(conn) => match db::get_setting(&conn, key) {
            Ok(Some(v)) => v == "1",
            _ => default,
        },
        Err(_) => default,
    }
}

/// Scan every library, then run the follow-up jobs. Safe to call from any
/// thread; overlapping requests are coalesced into one extra run.
pub fn refresh(app: &AppHandle, reason: &str) {
    let state = app.state::<AppState>();
    if state.scanning.swap(true, Ordering::SeqCst) {
        state.rescan_wanted.store(true, Ordering::SeqCst);
        return;
    }
    loop {
        state.rescan_wanted.store(false, Ordering::SeqCst);
        let result = {
            // A scan can run for minutes over external drives. Holding the
            // shared connection would block every synchronous command, and
            // those run on the main thread, so the window would freeze for
            // the duration. WAL mode lets the scan have its own connection.
            match db::open(&state.db_path) {
                Ok(mut conn) => scanner::scan_all(&mut conn),
                Err(e) => Err(e.to_string()),
            }
        };
        match result {
            Ok(stats) => {
                let _ = app.emit("scan-done", ScanDone { reason: reason.to_string(), stats: stats.clone() });
                // Windows toast for arrivals noticed by the folder watcher or a reconnected drive.
                if (reason == "watch" || reason == "drive" || reason == "torrent") && !stats.added_titles.is_empty()
                    && setting_on(app, "notify_new", true)
                {
                    use tauri_plugin_notification::NotificationExt;
                    let more = stats.added.saturating_sub(stats.added_titles.len());
                    let mut body = stats.added_titles.join("\n");
                    if more > 0 {
                        body.push_str(&format!("\n… and {more} more"));
                    }
                    let title = if stats.added == 1 { "New in your library".to_string() } else { format!("{} new in your library", stats.added) };
                    let _ = app.notification().builder().title(title).body(body).show();
                }
                if stats.changed() || reason == "startup" {
                    let _ = probe::probe_missing(app.clone());
                    if setting_on(app, "tmdb_connected", false) {
                        let _ = tmdb::fetch_missing(app.clone(), false);
                    }
                }
                tray::rebuild(app);
            }
            Err(e) => {
                let _ = app.emit("scan-error", e);
            }
        }
        if !state.rescan_wanted.load(Ordering::SeqCst) {
            break;
        }
    }
    state.scanning.store(false, Ordering::SeqCst);
}

pub fn refresh_async(app: AppHandle, reason: &'static str) {
    std::thread::spawn(move || refresh(&app, reason));
}
