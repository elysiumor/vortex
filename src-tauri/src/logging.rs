//! A session log at `%APPDATA%\com.admin.vortex\vortex.log`, written from
//! startup until the app exits. Both Vortex and librqbit instrument themselves
//! with `tracing`, so peer, tracker and piece activity lands here too, which is
//! what makes a torrent problem diagnosable after the fact.
//!
//! The previous run is kept as `vortex.log.1`, so a crash report still has the
//! session that caused it even after the app has been restarted. A session
//! that outgrows `MAX_BYTES` (one left running in the tray for days) rolls its
//! older half over to `vortex.log.0` rather than filling the disk.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

/// Default verbosity. librqbit is chatty at debug, so it is held at info
/// unless `VORTEX_LOG` overrides everything.
const DEFAULT_FILTER: &str = "info,ui=debug,vortex_lib=debug,librqbit=info,librqbit_dht=warn";

const MAX_BYTES: u64 = 20 * 1024 * 1024;

/// Local wall-clock time. UTC stamps made every report a time-zone puzzle:
/// "it broke at 3 pm" had to be matched against 09:30Z.
struct LocalTime;

impl FormatTime for LocalTime {
    #[cfg(windows)]
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut t) };
        write!(
            w,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds
        )
    }

    #[cfg(not(windows))]
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        tracing_subscriber::fmt::time::SystemTime.format_time(w)
    }
}

/// The log file, rolled to `vortex.log.0` when it reaches `MAX_BYTES`.
struct RollingFile {
    path: PathBuf,
    file: Option<File>,
    written: u64,
}

impl RollingFile {
    fn roll(&mut self) {
        // Closed first: Windows will not rename a file that is open.
        self.file = None;
        let _ = std::fs::rename(&self.path, self.path.with_extension("log.0"));
        self.file = File::create(&self.path).ok();
        self.written = 0;
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "(continued; the earlier part of this session is in vortex.log.0)");
        }
    }
}

impl Write for RollingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written >= MAX_BYTES {
            self.roll();
        }
        let Some(f) = self.file.as_mut() else { return Ok(buf.len()) };
        let n = f.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().map(|f| f.flush()).unwrap_or(Ok(()))
    }
}

/// Start file logging and return the path, so the UI can offer to open it.
pub fn init(dir: &Path) -> Option<PathBuf> {
    let path = dir.join("vortex.log");
    // Keep one previous run; a crash is usually reported after a restart.
    let _ = std::fs::rename(&path, dir.join("vortex.log.1"));
    let _ = std::fs::remove_file(dir.join("vortex.log.0"));
    let file = File::create(&path).ok()?;
    let writer = RollingFile { path: path.clone(), file: Some(file), written: 0 };

    let filter = EnvFilter::try_from_env("VORTEX_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let result = tracing_subscriber::fmt()
        .with_writer(Mutex::new(writer).with_max_level(tracing::Level::TRACE))
        .with_timer(LocalTime)
        .with_ansi(false)
        .with_target(true)
        .with_env_filter(filter)
        .try_init();
    if result.is_err() {
        return None; // something already installed a subscriber
    }

    tracing::info!(version = env!("CARGO_PKG_VERSION"), os = std::env::consts::OS, "Vortex starting");
    Some(path)
}

/// What a bug report needs to be read: the settings that change behaviour
/// and every library with whether it was reachable. Runs on its own thread,
/// since checking a sleeping drive can take seconds. Secrets (the TMDB key,
/// a proxy URL with its credentials) are only ever logged as present or not.
pub fn startup_summary(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<crate::AppState>();
        let (settings, mut libs) = {
            let Ok(conn) = state.db.lock() else { return };
            let settings: std::collections::HashMap<String, String> =
                crate::db::all_settings(&conn).unwrap_or_default().into_iter().collect();
            (settings, crate::db::list_libraries_rows(&conn).unwrap_or_default())
        };
        crate::db::fill_availability(&mut libs);
        let get = |k: &str| settings.get(k).map(String::as_str).unwrap_or("");
        let set = |k: &str| !get(k).trim().is_empty();
        tracing::info!(
            player = get("player_kind"),
            player_path = get("player_path"),
            tmdb_key = set("tmdb_key"),
            tmdb_connected = get("tmdb_connected") == "1",
            ffprobe_path = get("ffprobe_path"),
            rescan_on_startup = get("rescan_on_startup") != "0",
            watch_folders = get("watch_folders") != "0",
            torrent_dir = get("torrent_dir"),
            proxy = set("torrent_proxy"),
            "settings"
        );
        if libs.is_empty() {
            tracing::info!("no libraries yet");
        }
        for lib in &libs {
            tracing::info!(id = lib.id, path = %lib.path, available = lib.available, "library");
        }
    });
}

/// Note the clean shutdown, so a log that simply stops is recognisable as a
/// crash rather than a normal exit.
pub fn closing(reason: &str) {
    tracing::info!(reason, "Vortex closing");
}
