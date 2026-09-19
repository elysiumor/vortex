//! Watches library folders and rescans when files are added, removed or
//! renamed. Also notices when an external drive comes back and rescans it.

use crate::{db, jobs, AppState};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub fn start(app: AppHandle) {
    std::thread::spawn(move || run(app));
}

fn run(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();
    let mut debouncer = match new_debouncer(Duration::from_secs(3), move |res: DebounceEventResult| {
        if let Ok(events) = res {
            if events.iter().any(|e| is_video_or_dir(&e.path)) {
                let _ = tx.send(());
            }
        }
    }) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut watched: HashSet<String> = HashSet::new();
    loop {
        // Add watches for libraries that are available and not watched yet
        // (covers newly added folders and a reconnected external drive).
        let libs = {
            let state = app.state::<AppState>();
            let guard = state.db.lock();
            match guard {
                Ok(conn) => db::list_libraries(&conn).unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        };
        let mut newly_available = false;
        for lib in &libs {
            if lib.available && !watched.contains(&lib.path) {
                if debouncer.watcher().watch(Path::new(&lib.path), RecursiveMode::Recursive).is_ok() {
                    if !watched.is_empty() || !watched.contains(&lib.path) {
                        newly_available = true;
                    }
                    watched.insert(lib.path.clone());
                }
            } else if !lib.available && watched.contains(&lib.path) {
                let _ = debouncer.watcher().unwatch(Path::new(&lib.path));
                watched.remove(&lib.path);
            }
        }
        // Drop watches for removed libraries.
        let current: HashSet<String> = libs.iter().map(|l| l.path.clone()).collect();
        for gone in watched.difference(&current).cloned().collect::<Vec<_>>() {
            let _ = debouncer.watcher().unwatch(Path::new(&gone));
            watched.remove(&gone);
        }
        if newly_available && jobs::setting_on(&app, "watch_folders", true) {
            jobs::refresh(&app, "drive");
        }

        // Wait for file events (or time out to re-check drives).
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(()) => {
                while rx.try_recv().is_ok() {} // collapse a burst into one scan
                if jobs::setting_on(&app, "watch_folders", true) {
                    jobs::refresh(&app, "watch");
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn is_video_or_dir(p: &Path) -> bool {
    p.extension().is_none() || crate::parser::is_video(p)
}
