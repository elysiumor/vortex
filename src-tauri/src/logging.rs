//! A session log at `%APPDATA%\com.admin.vortex\vortex.log`, written from
//! startup until the app exits. Both Vortex and librqbit instrument themselves
//! with `tracing`, so peer, tracker and piece activity lands here too, which is
//! what makes a torrent problem diagnosable after the fact.
//!
//! The previous run is kept as `vortex.log.1`, so a crash report still has the
//! session that caused it even after the app has been restarted.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::EnvFilter;

/// Default verbosity. librqbit is chatty at debug, so it is held at info
/// unless `VORTEX_LOG` overrides everything.
const DEFAULT_FILTER: &str = "info,ui=debug,vortex_lib=debug,librqbit=info,librqbit_dht=warn";

/// Start file logging and return the path, so the UI can offer to open it.
pub fn init(dir: &Path) -> Option<PathBuf> {
    let path = dir.join("vortex.log");
    // Keep one previous run; a crash is usually reported after a restart.
    let _ = std::fs::rename(&path, dir.join("vortex.log.1"));
    let file = File::create(&path).ok()?;

    let filter = EnvFilter::try_from_env("VORTEX_LOG").unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let result = tracing_subscriber::fmt()
        .with_writer(Mutex::new(file).with_max_level(tracing::Level::TRACE))
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

/// Note the clean shutdown, so a log that simply stops is recognisable as a
/// crash rather than a normal exit.
pub fn closing(reason: &str) {
    tracing::info!(reason, "Vortex closing");
}
