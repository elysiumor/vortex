//! Background maintenance: scan, durations, posters, tray refresh. Used by
//! startup, the folder watcher, the tray menu and the Settings button.

use crate::{db, probe, scanner, tmdb, tray, AppState};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// One-at-a-time background work: the library scan, poster fetch and
/// duration probe. A request made while a pass is running is remembered and
/// served by one more pass when it ends, rather than refused. Refusing meant
/// titles added mid-run waited for the next trigger, possibly the next launch.
#[derive(Default)]
pub struct Job {
    running: AtomicBool,
    again: AtomicBool,
}

impl Job {
    /// True when the caller now owns the job and must run it; false when a
    /// pass is already going and will run once more on the caller's behalf.
    pub fn begin(&self) -> bool {
        self.again.store(true, Ordering::SeqCst);
        if self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        self.again.store(false, Ordering::SeqCst);
        true
    }

    /// Take the job only if it is idle, without asking a running pass to go again.
    fn try_own(&self) -> bool {
        !self.running.swap(true, Ordering::SeqCst)
    }

    /// Called by the owner after each pass. True means another pass was
    /// asked for meanwhile and the owner must run it.
    pub fn another_pass(&self) -> bool {
        self.running.store(false, Ordering::SeqCst);
        // Checked after letting go, so a request landing in between is either
        // seen here or wins `begin` itself; it cannot fall through the gap.
        if self.again.load(Ordering::SeqCst) && !self.running.swap(true, Ordering::SeqCst) {
            self.again.store(false, Ordering::SeqCst);
            return true;
        }
        false
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Lets go of the job if the owner panics, which would otherwise leave it
    /// marked running, and every later request ignored, until restart.
    pub fn release_on_panic(&self) -> ReleaseOnPanic<'_> {
        ReleaseOnPanic(self)
    }
}

pub struct ReleaseOnPanic<'a>(&'a Job);

impl Drop for ReleaseOnPanic<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.0.running.store(false, Ordering::SeqCst);
        }
    }
}

/// Scan every library, then run the follow-up jobs. Safe to call from any
/// thread; a request made during a scan is served by one more pass.
pub fn refresh(app: &AppHandle, reason: &str) {
    if app.state::<AppState>().scan_job.begin() {
        let _ = run_scans(app, reason);
    }
}

/// The Settings "Rescan" button and adding a library. Waits for a running
/// scan rather than folding into it, so the numbers returned are this scan's
/// and two scans never write at once.
pub fn scan_now(app: &AppHandle) -> Result<scanner::ScanStats, String> {
    let state = app.state::<AppState>();
    while !state.scan_job.try_own() {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    run_scans(app, "manual")
}

/// Run `f` with no scan running or starting, such as renaming files and
/// their library rows together, which a scan must not see half done. Scans
/// asked for meanwhile are deferred, not dropped.
pub fn exclusive<T>(app: &AppHandle, f: impl FnOnce() -> T) -> T {
    let state = app.state::<AppState>();
    while !state.scan_job.try_own() {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    let out = {
        let _release = state.scan_job.release_on_panic();
        f()
    };
    while state.scan_job.another_pass() {
        let _ = scan_once(app, "watch");
    }
    out
}

/// Runs while owning `scan_job`, and lets go of it at the end.
fn run_scans(app: &AppHandle, reason: &str) -> Result<scanner::ScanStats, String> {
    let state = app.state::<AppState>();
    let _release = state.scan_job.release_on_panic();
    let first = scan_once(app, reason);
    while state.scan_job.another_pass() {
        let _ = scan_once(app, reason);
    }
    first
}

fn scan_once(app: &AppHandle, reason: &str) -> Result<scanner::ScanStats, String> {
    let state = app.state::<AppState>();
    let started = std::time::Instant::now();
    tracing::info!(reason, "scan starting");
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
    match &result {
        Ok(s) => tracing::info!(
            reason,
            ms = started.elapsed().as_millis() as u64,
            files = s.files_seen,
            added = s.added,
            removed = s.removed,
            offline = s.libraries_skipped.len(),
            "scan finished"
        ),
        Err(e) => tracing::error!(reason, "scan failed: {e}"),
    }
    match &result {
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
    result
}

pub fn refresh_async(app: AppHandle, reason: &'static str) {
    std::thread::spawn(move || refresh(&app, reason));
}
