use crate::db;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Clone, Debug)]
pub struct DetectedPlayer {
    pub kind: String,
    pub name: String,
    pub path: String,
}

const CANDIDATES: &[(&str, &str, &[&str])] = &[
    (
        "potplayer",
        "PotPlayer",
        &[
            r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe",
            r"C:\Program Files (x86)\DAUM\PotPlayer\PotPlayerMini.exe",
        ],
    ),
    (
        "vlc",
        "VLC",
        &[r"C:\Program Files\VideoLAN\VLC\vlc.exe", r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe"],
    ),
    (
        "mpc-hc",
        "MPC-HC",
        &[
            r"C:\Program Files\MPC-HC\mpc-hc64.exe",
            r"C:\Program Files (x86)\MPC-HC\mpc-hc.exe",
            r"C:\Program Files (x86)\K-Lite Codec Pack\MPC-HC64\mpc-hc64.exe",
            r"C:\Program Files\K-Lite Codec Pack\MPC-HC64\mpc-hc64.exe",
        ],
    ),
    ("mpv", "mpv", &[r"C:\Program Files\mpv\mpv.exe", r"C:\mpv\mpv.exe"]),
];

pub fn detect() -> Vec<DetectedPlayer> {
    let mut out = Vec::new();
    for (kind, name, paths) in CANDIDATES {
        let mut found = paths.iter().find(|p| Path::new(p).exists()).map(|p| p.to_string());
        if found.is_none() {
            if let Ok(home) = std::env::var("USERPROFILE") {
                let scoop = format!(r"{home}\scoop\shims\{}.exe", if *kind == "mpc-hc" { "mpc-hc64" } else { kind });
                if Path::new(&scoop).exists() {
                    found = Some(scoop);
                }
            }
        }
        if let Some(path) = found {
            out.push(DetectedPlayer { kind: kind.to_string(), name: name.to_string(), path });
        }
    }
    out
}

fn hms(secs: i64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

/// How we talk to the running player to read its real position.
enum Link {
    None,
    VlcHttp { port: u16, password: String },
    MpvPipe { name: String },
    /// PotPlayer answers WM_USER messages on its main window.
    PotWindow,
}

#[cfg(windows)]
mod pot {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SendMessageTimeoutW, SMTO_ABORTIFHUNG, WM_USER,
    };

    const POT_GET_TOTAL_TIME: WPARAM = 0x5002;
    const POT_GET_CURRENT_TIME: WPARAM = 0x5004;
    const POT_GET_PLAY_STATUS: WPARAM = 0x5006; // -1 none, 0 stopped, 1 paused, 2 playing

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn find_window() -> Option<HWND> {
        for class in ["PotPlayer64", "PotPlayer"] {
            let h = unsafe { FindWindowW(wide(class).as_ptr(), std::ptr::null()) };
            if !h.is_null() {
                return Some(h);
            }
        }
        None
    }

    fn send(hwnd: HWND, code: WPARAM) -> Option<isize> {
        let mut result: usize = 0;
        let ok = unsafe { SendMessageTimeoutW(hwnd, WM_USER, code, 0 as LPARAM, SMTO_ABORTIFHUNG, 1000, &mut result) };
        (ok != 0).then_some(result as isize)
    }

    /// Position and length while playing or paused; `Ended` once PotPlayer
    /// reports "stopped" (which is what it does after the file finishes).
    pub fn read() -> super::Reading {
        use super::Reading;
        let Some(hwnd) = find_window() else { return Reading::Silent };
        let Some(status) = send(hwnd, POT_GET_PLAY_STATUS) else { return Reading::Silent };
        match status {
            0 => return Reading::Ended,
            1 | 2 => {}
            _ => return Reading::Silent,
        }
        let Some(pos_ms) = send(hwnd, POT_GET_CURRENT_TIME) else { return Reading::Silent };
        if pos_ms < 0 {
            return Reading::Silent;
        }
        let len = send(hwnd, POT_GET_TOTAL_TIME).filter(|l| *l > 0).map(|l| (l as i64 + 500) / 1000);
        Reading::Playing { pos: (pos_ms as i64 + 500) / 1000, len }
    }
}

#[cfg(not(windows))]
mod pot {
    pub fn read() -> super::Reading {
        super::Reading::Silent
    }
}

/// One poll of the running player.
pub enum Reading {
    Playing { pos: i64, len: Option<i64> },
    /// The player is open but reports the file has stopped.
    Ended,
    /// No answer (player not reachable, still loading, or unsupported).
    Silent,
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port()).unwrap_or(48550)
}

fn nonce() -> String {
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("vx{:x}", n ^ (std::process::id() as u128) << 40)
}

fn build_launch(kind: &str, file: &str, start_secs: i64, subtitle: Option<&str>) -> (Vec<String>, Link) {
    let mut args = vec![file.to_string()];
    let mut link = Link::None;
    // PotPlayer and MPC-HC load same-name sidecar subtitles on their own.
    if let Some(sub) = subtitle {
        match kind {
            "vlc" => args.push(format!("--sub-file={sub}")),
            "mpv" => args.push(format!("--sub-file={sub}")),
            "mpc-hc" => {
                args.push("/sub".into());
                args.push(sub.to_string());
            }
            _ => {}
        }
    }
    match kind {
        "potplayer" => {
            if start_secs > 0 {
                args.push(format!("/seek={}", hms(start_secs)));
            }
            link = Link::PotWindow;
        }
        "vlc" => {
            let port = free_port();
            let password = nonce();
            args.extend([
                "--extraintf=http".into(),
                "--http-host=127.0.0.1".into(),
                format!("--http-port={port}"),
                format!("--http-password={password}"),
            ]);
            if start_secs > 0 {
                args.push(format!("--start-time={start_secs}"));
            }
            link = Link::VlcHttp { port, password };
        }
        "mpc-hc" => {
            if start_secs > 0 {
                args.push("/startpos".into());
                args.push(hms(start_secs));
            }
        }
        "mpv" => {
            let name = nonce();
            args.push(format!(r"--input-ipc-server=\\.\pipe\{name}"));
            if start_secs > 0 {
                args.push(format!("--start={start_secs}"));
            }
            link = Link::MpvPipe { name };
        }
        _ => {}
    }
    (args, link)
}

#[derive(Deserialize)]
struct VlcStatus {
    time: Option<f64>,
    length: Option<f64>,
    state: Option<String>,
}

fn read_vlc(port: u16, password: &str) -> Reading {
    let resp = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| {
            c.get(format!("http://127.0.0.1:{port}/requests/status.json")).basic_auth("", Some(password)).send().ok()
        });
    let Some(resp) = resp else { return Reading::Silent };
    let Ok(s) = resp.json::<VlcStatus>() else { return Reading::Silent };
    if s.state.as_deref() == Some("stopped") {
        return Reading::Ended;
    }
    let len = s.length.filter(|l| *l > 0.0).map(|l| l.round() as i64);
    match s.time {
        Some(t) => Reading::Playing { pos: t.round() as i64, len },
        None => Reading::Silent,
    }
}

fn mpv_get(pipe: &mut std::fs::File, prop: &str) -> Option<f64> {
    let cmd = format!("{{\"command\":[\"get_property\",\"{prop}\"]}}\n");
    pipe.write_all(cmd.as_bytes()).ok()?;
    let mut reader = BufReader::new(pipe);
    // mpv may interleave event lines; read until a line with "data".
    for _ in 0..20 {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if let Some(d) = v.get("data").and_then(|d| d.as_f64()) {
                return Some(d);
            }
            if v.get("error").is_some() && v.get("data").is_none() {
                return None;
            }
        }
    }
    None
}

fn read_mpv(name: &str) -> Reading {
    let Ok(mut pipe) = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\{name}")) else {
        return Reading::Silent;
    };
    let Some(pos) = mpv_get(&mut pipe, "time-pos") else { return Reading::Silent };
    let len = mpv_get(&mut pipe, "duration").filter(|d| *d > 0.0).map(|d| d.round() as i64);
    Reading::Playing { pos: pos.round() as i64, len }
}

fn read_link(link: &Link) -> Reading {
    match link {
        Link::None => Reading::Silent,
        Link::VlcHttp { port, password } => read_vlc(*port, password),
        Link::MpvPipe { name } => read_mpv(name),
        Link::PotWindow => pot::read(),
    }
}

fn player_running(sys: &mut sysinfo::System, exe_name: &str) -> bool {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let name = exe_name.to_ascii_lowercase();
    sys.processes().values().any(|p| p.name().to_string_lossy().to_ascii_lowercase() == name)
}

#[derive(Serialize, Clone)]
struct PlaybackEnded {
    episode_id: i64,
    position_secs: i64,
    completed: bool,
    exact: bool,
    /// Next episode in the series, if the finished one was completed.
    next_episode_id: Option<i64>,
    next_label: Option<String>,
    /// True when the app already started the next episode in the open player.
    auto_started: bool,
}

fn label_for(ep: &db::Episode) -> String {
    let code = match (ep.season, ep.episode) {
        (Some(s), Some(e)) => format!("S{s:02}E{e:02}"),
        (_, Some(e)) => format!("E{e:02}"),
        _ => String::new(),
    };
    match &ep.title {
        Some(t) if !code.is_empty() => format!("{code} · {t}"),
        Some(t) => t.clone(),
        None if !code.is_empty() => code,
        None => ep.file_name.clone(),
    }
}

/// Launch the configured player and track playback until it closes.
/// VLC and mpv report their real position; other players are estimated from
/// elapsed time. Emits `playback-ended` afterwards. Returns `false` when no
/// player is configured and the file was opened with the OS default app.
pub fn play(app: AppHandle, episode_id: i64) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let (episode, player_kind, player_path) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let ep = db::get_episode(&conn, episode_id).map_err(|e| e.to_string())?.ok_or("episode not found")?;
        let kind = db::get_setting(&conn, "player_kind").map_err(|e| e.to_string())?;
        let path = db::get_setting(&conn, "player_path").map_err(|e| e.to_string())?;
        db::touch_progress(&conn, episode_id).map_err(|e| e.to_string())?;
        (ep, kind, path)
    };

    if !Path::new(&episode.path).exists() {
        return Err("File is not available. Is the drive connected?".into());
    }

    let start = if episode.completed { 0 } else { episode.position_secs };

    let (mut child, exe_name, link) = match (player_kind.as_deref(), player_path.as_deref()) {
        (Some(kind), Some(path)) if !path.is_empty() && Path::new(path).exists() => {
            // First sidecar subtitle, preferring an English one if several exist.
            let subtitle = episode.subtitles.as_deref().and_then(|s| {
                let list: Vec<&str> = s.split('|').filter(|p| Path::new(p).exists()).collect();
                list.iter()
                    .find(|p| {
                        let l = p.to_lowercase();
                        l.contains(".en.") || l.contains(".eng.") || l.contains("english")
                    })
                    .or(list.first())
                    .map(|p| p.to_string())
            });
            let (args, link) = build_launch(kind, &episode.path, start, subtitle.as_deref());
            let child = Command::new(path).args(args).spawn().map_err(|e| format!("failed to start player: {e}"))?;
            let exe = Path::new(path).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            (child, exe, link)
        }
        _ => {
            tauri_plugin_opener::open_path(&episode.path, None::<&str>).map_err(|e| e.to_string())?;
            return Ok(false);
        }
    };

    let started = Instant::now();
    let mut duration = episode.duration_secs;
    std::thread::spawn(move || {
        let mut sys = sysinfo::System::new();
        let mut exact_pos: Option<i64> = None;
        let mut max_pos: i64 = 0;
        let mut seen_playing = false;
        let mut ended_in_player = false;
        let mut last_write = Instant::now();
        let mut child_gone = false;

        loop {
            std::thread::sleep(Duration::from_secs(2));

            if !child_gone {
                if let Ok(Some(_)) = child.try_wait() {
                    child_gone = true;
                }
            }
            // Single-instance players hand the file to an existing window and
            // exit at once; keep tracking while any process of that name lives.
            let alive = if child_gone { player_running(&mut sys, &exe_name) } else { true };
            if !alive {
                break;
            }

            match read_link(&link) {
                Reading::Playing { pos, len } => {
                    seen_playing = true;
                    exact_pos = Some(pos);
                    max_pos = max_pos.max(pos);
                    if duration.is_none() {
                        duration = len;
                    }
                    // Persist every 15 s so a crash or power loss keeps most progress.
                    if last_write.elapsed() >= Duration::from_secs(15) {
                        let state = app.state::<AppState>();
                        let guard = state.db.lock();
                        if let Ok(conn) = guard {
                            let _ = db::set_progress(&conn, episode_id, pos, false);
                            if let (Some(d), None) = (len, episode.duration_secs) {
                                let _ = db::set_duration(&conn, episode_id, d);
                            }
                        }
                        last_write = Instant::now();
                    }
                }
                Reading::Ended if seen_playing => {
                    // Only treat "stopped" as the end of the episode if we got
                    // near the end; a manual Stop half-way keeps tracking.
                    let near_end = match duration {
                        Some(d) => max_pos >= (d as f64 * 0.9) as i64,
                        None => true,
                    };
                    if near_end {
                        ended_in_player = true;
                        break;
                    }
                }
                _ => {}
            }
        }

        let elapsed = started.elapsed().as_secs() as i64;
        let exact = exact_pos.is_some();
        if !exact && elapsed < 15 {
            return; // opened and closed immediately; nothing to record
        }
        let mut position = if ended_in_player { max_pos } else { exact_pos.unwrap_or(start + elapsed) };
        let mut completed = ended_in_player;
        if let Some(d) = duration {
            position = position.min(d);
            if position >= (d as f64 * 0.9) as i64 {
                completed = true;
            }
        }

        let state = app.state::<AppState>();
        let autoplay = db_flag(&app, "autoplay_next", true);
        let next = {
            let guard = state.db.lock();
            match guard {
                Ok(conn) => {
                    let _ = db::set_progress(&conn, episode_id, position, completed);
                    let _ = db::add_history(&conn, episode_id, position, completed, exact, "player");
                    if completed && autoplay {
                        db::next_episode(&conn, episode_id).ok().flatten()
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        };

        // Episode finished with the player still open: go straight to the next one.
        let auto_started = match (&next, ended_in_player) {
            (Some(n), true) => play(app.clone(), n.id).is_ok(),
            _ => false,
        };
        let _ = app.emit(
            "playback-ended",
            PlaybackEnded {
                episode_id,
                position_secs: position,
                completed,
                exact,
                next_episode_id: next.as_ref().map(|n| n.id),
                next_label: next.as_ref().map(label_for),
                auto_started,
            },
        );
        crate::tray::rebuild(&app);
    });
    Ok(true)
}

fn db_flag(app: &AppHandle, key: &str, default: bool) -> bool {
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
