//! File duration detection. Uses ffprobe when available; otherwise parses
//! MKV and MP4 containers directly so most files work with no extra install.

use crate::db;
use crate::AppState;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

const FFPROBE_CANDIDATES: &[&str] = &[
    r"C:\ffmpeg\bin\ffprobe.exe",
    r"C:\Program Files\ffmpeg\bin\ffprobe.exe",
    r"C:\Program Files\FFmpeg\bin\ffprobe.exe",
    r"C:\ProgramData\chocolatey\bin\ffprobe.exe",
];

pub fn detect_ffprobe() -> Option<String> {
    if let Ok(out) = Command::new("where").arg("ffprobe").output() {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = line.trim();
                if Path::new(p).exists() {
                    return Some(p.to_string());
                }
            }
        }
    }
    let mut candidates: Vec<String> = FFPROBE_CANDIDATES.iter().map(|s| s.to_string()).collect();
    if let Ok(home) = std::env::var("USERPROFILE") {
        candidates.push(format!(r"{home}\scoop\shims\ffprobe.exe"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        // winget installs Gyan.FFmpeg under a versioned folder; take any match.
        let base = Path::new(&local).join(r"Microsoft\WinGet\Packages");
        if let Ok(rd) = std::fs::read_dir(&base) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().to_lowercase().contains("ffmpeg") {
                    if let Ok(inner) = std::fs::read_dir(e.path()) {
                        for f in inner.flatten() {
                            candidates.push(f.path().join("bin").join("ffprobe.exe").to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    candidates.into_iter().find(|p| Path::new(p).exists())
}

fn via_ffprobe(ffprobe: &str, path: &Path) -> Option<i64> {
    let out = Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<f64>().ok().map(|d| d.round() as i64).filter(|d| *d > 0)
}

fn via_matroska(path: &Path) -> Option<i64> {
    let f = std::fs::File::open(path).ok()?;
    let mkv = matroska::Matroska::open(f).ok()?;
    mkv.info.duration.map(|d| d.as_secs() as i64).filter(|d| *d > 0)
}

fn via_mp4(path: &Path) -> Option<i64> {
    let f = std::fs::File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    let reader = mp4::Mp4Reader::read_header(std::io::BufReader::new(f), size).ok()?;
    let d = reader.duration().as_secs() as i64;
    (d > 0).then_some(d)
}

pub fn duration_of(path: &Path, ffprobe: Option<&str>) -> Option<i64> {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let native = match ext.as_str() {
        "mkv" | "webm" => via_matroska(path),
        "mp4" | "m4v" | "mov" => via_mp4(path),
        _ => None,
    };
    native.or_else(|| ffprobe.and_then(|f| via_ffprobe(f, path)))
}

#[derive(Serialize, Clone)]
struct Progress {
    done: usize,
    total: usize,
    found: usize,
}

/// Background job: fill in durations for episodes that have none.
pub fn probe_missing(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.probing.swap(true, Ordering::SeqCst) {
        return Err("Duration scan already running".into());
    }
    let (ffprobe, items) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let configured = db::get_setting(&conn, "ffprobe_path").map_err(|e| e.to_string())?;
        let ffprobe = configured.filter(|p| !p.is_empty() && Path::new(p).exists()).or_else(detect_ffprobe);
        (ffprobe, db::episodes_missing_duration(&conn).map_err(|e| e.to_string())?)
    };
    std::thread::spawn(move || {
        let total = items.len();
        let mut found = 0;
        for (i, (id, path)) in items.iter().enumerate() {
            let p = Path::new(path);
            if !p.exists() {
                continue;
            }
            match duration_of(p, ffprobe.as_deref()) {
                Some(d) => {
                    let state = app.state::<AppState>();
                    let guard = state.db.lock();
                    if let Ok(conn) = guard {
                        let _ = db::set_duration(&conn, *id, d);
                        found += 1;
                    }
                }
                None => {
                    let state = app.state::<AppState>();
                    let guard = state.db.lock();
                    if let Ok(conn) = guard {
                        let _ = db::mark_duration_checked(&conn, *id);
                    }
                }
            }
            if i % 10 == 0 {
                let _ = app.emit("durations-progress", Progress { done: i, total, found });
            }
        }
        let _ = app.emit("durations-done", Progress { done: total, total, found });
        app.state::<AppState>().probing.store(false, Ordering::SeqCst);
    });
    Ok(())
}
