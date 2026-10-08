//! Watches library folders and rescans when files are added, removed or
//! renamed. Also notices when an external drive comes back and rescans it.

use crate::{db, jobs, scanner, AppState};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::collections::HashSet;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub fn start(app: AppHandle) {
    std::thread::spawn(move || run(app));
}

fn run(app: AppHandle) {
    // Carries the first relevant path of each burst, for the log.
    let (tx, rx) = mpsc::channel::<std::path::PathBuf>();
    // Folder names the scanner skips; refreshed every pass of the loop below.
    let ignore: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(scanner::DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect()));
    let ignore_events = ignore.clone();
    // Library roots, so only folders below a root are matched against the
    // ignore list: a library at "D:\Temp\Movies" must still be watched.
    let roots: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let roots_events = roots.clone();
    let mut debouncer = match new_debouncer(Duration::from_secs(3), move |res: DebounceEventResult| {
        if let Ok(events) = res {
            let ignore = ignore_events.lock().map(|g| g.clone()).unwrap_or_default();
            let roots = roots_events.lock().map(|g| g.clone()).unwrap_or_default();
            if let Some(e) = events.iter().find(|e| is_relevant(&e.path, &ignore, &roots)) {
                let _ = tx.send(e.path.clone());
            }
        }
    }) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut watched: HashSet<String> = HashSet::new();
    // Libraries seen offline at some point. Only one of these coming back
    // needs a scan of its own; one watched for the first time was just
    // scanned by whatever added it (startup or Settings).
    let mut offline: HashSet<String> = HashSet::new();
    loop {
        // Add watches for libraries that are available and not watched yet
        // (covers newly added folders and a reconnected external drive).
        let mut libs = {
            let state = app.state::<AppState>();
            let guard = state.db.lock();
            match guard {
                Ok(conn) => {
                    if let Ok(mut i) = ignore.lock() {
                        *i = scanner::ignored_dirs(&conn);
                    }
                    db::list_libraries_rows(&conn).unwrap_or_default()
                }
                Err(_) => Vec::new(),
            }
        };
        // Probing a sleeping or disconnected drive can block for seconds, and
        // this loop runs every 20s. Doing it under the lock would stall every
        // synchronous command, which Tauri runs on the main thread.
        db::fill_availability(&mut libs);
        let libs = libs;
        if let Ok(mut r) = roots.lock() {
            *r = libs.iter().map(|l| std::path::PathBuf::from(&l.path)).collect();
        }
        let mut newly_available = false;
        for lib in &libs {
            if lib.available && !watched.contains(&lib.path) {
                if debouncer.watcher().watch(Path::new(&lib.path), RecursiveMode::Recursive).is_ok() {
                    // The old test here was always true, so every launch and
                    // every added folder ran a second, redundant full scan.
                    if offline.remove(&lib.path) {
                        tracing::info!(library = %lib.path, "library back online");
                        newly_available = true;
                    }
                    watched.insert(lib.path.clone());
                }
            } else if !lib.available {
                if offline.insert(lib.path.clone()) {
                    tracing::info!(library = %lib.path, "library offline");
                }
                if watched.remove(&lib.path) {
                    let _ = debouncer.watcher().unwatch(Path::new(&lib.path));
                }
            }
        }
        // Drop watches for removed libraries.
        let current: HashSet<String> = libs.iter().map(|l| l.path.clone()).collect();
        offline.retain(|p| current.contains(p));
        for gone in watched.difference(&current).cloned().collect::<Vec<_>>() {
            let _ = debouncer.watcher().unwatch(Path::new(&gone));
            watched.remove(&gone);
        }
        if newly_available && jobs::setting_on(&app, "watch_folders", true) {
            jobs::refresh(&app, "drive");
        }

        // Wait for file events (or time out to re-check drives).
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(path) => {
                let mut more = 0;
                while rx.try_recv().is_ok() {
                    more += 1; // collapse a burst into one scan
                }
                tracing::info!(path = %path.display(), more, "folder change");
                if jobs::setting_on(&app, "watch_folders", true) {
                    jobs::refresh(&app, "watch");
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn is_relevant(p: &Path, ignore: &[String], roots: &[std::path::PathBuf]) -> bool {
    // Torrent pieces land in `.incomplete`, and folders the scanner skips
    // (caches, AppData on a whole-drive library) churn constantly; a scan per
    // write would be wasteful. Only the part below the library root counts,
    // as in the scanner.
    let below = roots.iter().filter_map(|r| p.strip_prefix(r).ok()).min_by_key(|rel| rel.components().count()).unwrap_or(p);
    let skipped = below.components().any(|c| {
        let name = c.as_os_str().to_string_lossy().to_lowercase();
        name == crate::torrent::STAGING_DIR || ignore.contains(&name)
    });
    if skipped {
        return false;
    }
    // Ask the filesystem rather than guess from an "extension": release
    // folders are full of dots ("The.Bear.S03.1080p.WEB.h264-GRP"), and a
    // folder moved in arrives as a single event for the folder itself. A path
    // that is gone may have been either, so let the scan decide.
    crate::parser::is_video(p) || p.is_dir() || !p.exists()
}
