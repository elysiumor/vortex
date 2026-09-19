//! Backup = one zip holding a consistent snapshot of the database plus the
//! cached posters and backdrops. Restore swaps the live database for the one
//! in the zip.

use crate::{db, tray, AppState};
use rusqlite::Connection;
use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use zip::write::SimpleFileOptions;

#[derive(Serialize, Clone)]
pub struct BackupInfo {
    pub path: String,
    pub bytes: u64,
    pub posters: usize,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

pub fn create(app: &AppHandle, dest: &Path) -> Result<BackupInfo, String> {
    let dir = data_dir(app)?;
    let snapshot = dir.join("backup-snapshot.db");
    let _ = std::fs::remove_file(&snapshot);
    {
        // VACUUM INTO writes a complete, consistent copy including everything still in the WAL.
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().to_string()]).map_err(|e| e.to_string())?;
    }

    let file = File::create(dest).map_err(|e| format!("cannot write {}: {e}", dest.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("vortex.db", opts).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    File::open(&snapshot).and_then(|mut f| f.read_to_end(&mut buf)).map_err(|e| e.to_string())?;
    zip.write_all(&buf).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&snapshot);

    let mut posters = 0;
    let posters_dir = dir.join("posters");
    if let Ok(rd) = std::fs::read_dir(&posters_dir) {
        // Images are already compressed; store them as-is.
        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = format!("posters/{}", entry.file_name().to_string_lossy());
            zip.start_file(name, stored).map_err(|e| e.to_string())?;
            let mut data = Vec::new();
            if File::open(&p).and_then(|mut f| f.read_to_end(&mut data)).is_ok() {
                zip.write_all(&data).map_err(|e| e.to_string())?;
                posters += 1;
            }
        }
    }
    zip.finish().map_err(|e| e.to_string())?;

    let bytes = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    let state = app.state::<AppState>();
    if let Ok(conn) = state.db.lock() {
        let _ = db::set_setting(&conn, "last_backup", &crate::tmdb::today());
        let _ = db::set_setting(&conn, "last_backup_path", &dest.to_string_lossy());
    }
    Ok(BackupInfo { path: dest.to_string_lossy().to_string(), bytes, posters })
}

pub fn restore(app: &AppHandle, src: &Path) -> Result<(), String> {
    let dir = data_dir(app)?;
    let file = File::open(src).map_err(|e| format!("cannot open backup: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("not a valid backup zip: {e}"))?;

    // Extract the database to a temp file and sanity-check it first.
    let incoming = dir.join("restore-incoming.db");
    {
        let mut entry = zip.by_name("vortex.db").map_err(|_| "backup does not contain vortex.db")?;
        let mut out = File::create(&incoming).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    {
        let check = Connection::open(&incoming).map_err(|e| format!("backup database unreadable: {e}"))?;
        let n: i64 = check
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('libraries','episodes','watch_progress')", [], |r| r.get(0))
            .map_err(|e| format!("backup database unreadable: {e}"))?;
        if n < 3 {
            let _ = std::fs::remove_file(&incoming);
            return Err("backup database is missing expected tables".into());
        }
    }

    // Swap the live database under the lock so nothing else touches it mid-way.
    let live = dir.join("vortex.db");
    {
        let state = app.state::<AppState>();
        let mut guard = state.db.lock().map_err(|e| e.to_string())?;
        let old = std::mem::replace(&mut *guard, Connection::open_in_memory().map_err(|e| e.to_string())?);
        drop(old);
        for suffix in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(dir.join(format!("vortex.db{suffix}")));
        }
        let keep_old = dir.join("vortex.db.before-restore");
        let _ = std::fs::remove_file(&keep_old);
        let _ = std::fs::rename(&live, &keep_old);
        std::fs::rename(&incoming, &live).map_err(|e| format!("could not replace database: {e}"))?;
        *guard = db::open(&live).map_err(|e| e.to_string())?;
    }

    // Posters: overwrite whatever the zip has; leave other cached images alone.
    let posters_dir = dir.join("posters");
    let _ = std::fs::create_dir_all(&posters_dir);
    for i in 0..zip.len() {
        let mut entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();
        let Some(file_name) = name.strip_prefix("posters/") else { continue };
        if file_name.is_empty() || file_name.contains('/') || file_name.contains("..") {
            continue;
        }
        if let Ok(mut out) = File::create(posters_dir.join(file_name)) {
            let _ = std::io::copy(&mut entry, &mut out);
        }
    }

    let _ = app.emit("library-restored", ());
    tray::rebuild(app);
    Ok(())
}
