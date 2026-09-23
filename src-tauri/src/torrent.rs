//! BitTorrent downloads and streaming on top of librqbit. There is no search
//! and no index: the user pastes a magnet link or picks a .torrent file.
//!
//! Privacy: with a SOCKS5 proxy configured, every socket the engine opens goes
//! through it. DHT, uTP, local discovery, incoming connections and UDP
//! trackers are switched off in that mode because librqbit cannot route them
//! through a proxy. Without a proxy the engine behaves like any ordinary
//! client and the UI says so plainly.
//!
//! Files download into `<folder>/.incomplete` (hidden, and ignored by the
//! scanner) and are moved into place when the torrent finishes, so the folder
//! watcher picks them up like any other arrival. The staging folder is removed
//! once the last torrent has left it.

use crate::{db, jobs, parser, AppState};
use librqbit::api::TorrentIdOrHash;
use librqbit::http_api::{HttpApi, HttpApiOptions};
use librqbit::limits::LimitsConfig;
use librqbit::{
    torrent_from_bytes, AddTorrent, AddTorrentOptions, AddTorrentResponse, Api, ConnectionOptions, DhtSessionConfig,
    ListenerOptions, ManagedTorrent, Session, SessionOptions, SessionPersistenceConfig, TorrentStatsState,
};
use serde::Serialize;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::runtime::Runtime;
use librqbit_dualstack_sockets::{BindOpts, TcpListener as DualstackTcpListener};

pub const STAGING_DIR: &str = ".incomplete";

#[derive(Serialize, Clone, Default)]
pub struct Config {
    pub proxy: String,
    pub dir: String,
    pub down_kbps: u32,
    pub up_kbps: u32,
}

#[derive(Serialize, Clone)]
pub struct Status {
    pub running: bool,
    pub protected: bool,
    pub config: Config,
    pub error: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PreviewFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub video: bool,
}

#[derive(Serialize, Clone)]
pub struct Preview {
    pub name: String,
    pub info_hash: String,
    pub total_bytes: u64,
    pub files: Vec<PreviewFile>,
}

#[derive(Serialize, Clone)]
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub done: u64,
    pub included: bool,
    pub video: bool,
}

#[derive(Serialize, Clone)]
pub struct TorrentRow {
    pub id: usize,
    pub name: String,
    pub info_hash: String,
    /// "checking", "downloading", "seeding", "paused" or "error"
    pub state: String,
    pub error: Option<String>,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub uploaded_bytes: u64,
    pub down_mbps: f64,
    pub up_mbps: f64,
    pub peers: usize,
    pub eta: Option<String>,
    pub finished: bool,
    /// Started with "Stream" and not yet kept or discarded.
    pub ephemeral: bool,
    pub files: Vec<TorrentFile>,
}

#[derive(Serialize, Clone)]
pub struct TorrentDone {
    pub name: String,
    pub moved: Vec<String>,
}

/// Where a torrent's files go when it finishes: chosen per torrent in the
/// add dialog, like uTorrent's "Save in" + "Create subfolder" + "Name".
#[derive(Serialize, serde::Deserialize, Clone, Default)]
pub struct Dest {
    pub save_in: String,
    /// Folder created inside `save_in`; `None` puts the files there directly.
    pub subfolder: Option<String>,
    /// Started with "Stream": stays in staging until the user keeps or
    /// discards it, and is never moved into the library on its own.
    #[serde(default)]
    pub ephemeral: bool,
    /// Where the player was when the stream last closed, for resume.
    #[serde(default)]
    pub position_secs: i64,
    #[serde(default)]
    pub duration_secs: Option<i64>,
    /// Set once the torrent finished and moved: the folder or file it became.
    #[serde(default)]
    pub completed_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct StreamStarted {
    pub id: usize,
    pub name: String,
    pub file: usize,
    /// Resumed from this many seconds in.
    pub start_secs: i64,
    /// False when no player is configured and the URL went to the default app.
    pub tracked: bool,
    /// The torrent had already finished and moved: this played the library file.
    pub from_library: bool,
}

#[derive(Serialize, Clone)]
pub struct StreamEnded {
    pub id: usize,
    pub name: String,
    pub position_secs: i64,
    pub duration_secs: Option<i64>,
    pub finished: bool,
    /// Ask Keep / Discard only for streams; ordinary downloads carry on.
    pub ephemeral: bool,
    /// True when the position came from the player rather than a clock.
    pub exact: bool,
}

/// Head buffer before the player is launched: whichever comes first.
const STREAM_HEAD_BYTES: u64 = 8 * 1024 * 1024;
const STREAM_HEAD_FRACTION: f64 = 0.02;
const STREAM_HEAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

pub struct Engine {
    rt: Runtime,
    session: Arc<Session>,
    stream_port: u16,
    proxy: Option<String>,
    dir: PathBuf,
    /// info_hash → destination, saved to `dests.json` so it survives restarts.
    dests: std::sync::Mutex<std::collections::HashMap<String, Dest>>,
    dests_file: PathBuf,
    api: Api,
    /// When each torrent was added this run, for "time elapsed".
    added: std::sync::Mutex<std::collections::HashMap<usize, std::time::Instant>>,
    limits: (u32, u32),
}

#[derive(Serialize, Clone)]
pub struct PeerRow {
    pub addr: String,
    pub client: Option<String>,
    pub state: String,
    pub kind: Option<String>,
    pub downloaded: u64,
    pub uploaded: u64,
}

#[derive(Serialize, Clone)]
pub struct TrackerRow {
    pub url: String,
    /// "http", "https", "udp" or "other"
    pub protocol: String,
    /// False when the engine will not contact it (UDP behind a proxy).
    pub active: bool,
}

/// uTorrent's Info tab: transfer counters plus what the .torrent says about itself.
#[derive(Serialize, Clone)]
pub struct TorrentDetail {
    pub id: usize,
    pub elapsed_secs: u64,
    pub downloaded: u64,
    pub remaining: u64,
    pub wasted: u64,
    pub uploaded: u64,
    pub down_mbps: f64,
    pub up_mbps: f64,
    pub down_limit_kbps: u32,
    pub up_limit_kbps: u32,
    pub share_ratio: f64,
    pub status: String,
    pub error: Option<String>,
    /// Peer counts by state as librqbit reports them (live, seen, connecting, …).
    pub peer_counts: serde_json::Value,
    pub save_as: String,
    pub total_size: u64,
    pub piece_count: u32,
    pub piece_length: u32,
    pub created_on: Option<i64>,
    pub created_by: Option<String>,
    pub comment: Option<String>,
    pub info_hash: String,
    pub peers: Vec<PeerRow>,
    pub trackers: Vec<TrackerRow>,
}

/// Bottom status bar: DHT and whole-session transfer.
#[derive(Serialize, Clone)]
pub struct SessionStatus {
    pub dht: Option<String>,
    pub down_mbps: f64,
    pub up_mbps: f64,
    pub downloaded_total: u64,
    pub uploaded_total: u64,
    pub peers_live: u64,
    pub uptime_secs: u64,
    pub protected: bool,
}

fn anyhow_str(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// Create the staging folder, out of the user's way. Hiding it keeps Explorer
/// tidy and, on Windows, is also what makes the scanner skip it.
fn make_staging(dir: &Path) -> Result<PathBuf, String> {
    let staging = dir.join(STAGING_DIR);
    std::fs::create_dir_all(&staging).map_err(|e| format!("cannot create {}: {e}", staging.display()))?;
    hide(&staging);
    Ok(staging)
}

#[cfg(windows)]
fn hide(p: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};
    let wide: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe { SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN) };
}

#[cfg(not(windows))]
fn hide(_p: &Path) {}

/// Drop the staging folder once the last torrent has left it. Fails harmlessly
/// while others are still downloading, and `make_staging` recreates it.
fn tidy_staging(staging: &Path) {
    if staging.file_name().map(|n| n == STAGING_DIR).unwrap_or(false) {
        let _ = std::fs::remove_dir(staging);
    }
}

pub fn read_config(app: &AppHandle) -> Config {
    let state = app.state::<AppState>();
    let Ok(conn) = state.db.lock() else { return Config::default() };
    let get = |k: &str| db::get_setting(&conn, k).ok().flatten().unwrap_or_default();
    Config {
        proxy: get("torrent_proxy").trim().to_string(),
        dir: get("torrent_dir"),
        down_kbps: get("torrent_down_kbps").parse().unwrap_or(0),
        up_kbps: get("torrent_up_kbps").parse().unwrap_or(0),
    }
}

/// Where librqbit keeps its session and resume data between runs.
fn persist_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("torrents"))
}

pub fn has_saved_session(app: &AppHandle) -> bool {
    persist_dir(app).map(|d| std::fs::read_dir(d).map(|mut i| i.next().is_some()).unwrap_or(false)).unwrap_or(false)
}

/// Everything that cannot go through a SOCKS proxy is off when one is set:
/// no DHT, no listener (TCP or uTP), no local discovery. UDP trackers are
/// handled at add time, see `Engine::to_add`.
fn session_options(proxy: Option<String>, persist: PathBuf, down_kbps: u32, up_kbps: u32) -> SessionOptions {
    let kbps = |v: u32| NonZeroU32::new(v.saturating_mul(1024));
    // DHT state lives next to the session file, not in rqbit's shared OS default.
    let dht = DhtSessionConfig {
        persistence: Some(librqbit::dht::DhtPersistenceConfig { config_filename: Some(persist.join("dht.json")), ..Default::default() }),
        ..Default::default()
    };
    SessionOptions {
        dht: if proxy.is_some() { None } else { Some(dht) },
        listen: if proxy.is_some() { None } else { Some(ListenerOptions::default()) },
        disable_local_service_discovery: proxy.is_some(),
        connect: Some(ConnectionOptions { proxy_url: proxy, enable_tcp: true, peer_opts: None }),
        persistence: Some(SessionPersistenceConfig::Json { folder: Some(persist) }),
        fastresume: true,
        ratelimits: LimitsConfig { download_bps: kbps(down_kbps), upload_bps: kbps(up_kbps) },
        ..Default::default()
    }
}

impl Engine {
    pub fn start(app: &AppHandle, cfg: &Config) -> Result<Arc<Engine>, String> {
        if cfg.dir.trim().is_empty() {
            return Err("Choose a download folder in Settings first.".into());
        }
        let dir = PathBuf::from(cfg.dir.trim());
        if !dir.is_dir() {
            return Err(format!("Download folder is not available: {}", dir.display()));
        }
        let staging = make_staging(&dir)?;
        let persist = persist_dir(app).ok_or("no app data dir")?;
        std::fs::create_dir_all(&persist).map_err(|e| e.to_string())?;

        let proxy = (!cfg.proxy.is_empty()).then(|| cfg.proxy.clone());
        if let Some(p) = &proxy {
            if !p.starts_with("socks5://") && !p.starts_with("socks5h://") {
                return Err("Proxy must be a socks5:// URL, e.g. socks5://10.64.0.1:1080".into());
            }
        }
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("vortex-torrent")
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;

        let opts = session_options(proxy.clone(), persist.clone(), cfg.down_kbps, cfg.up_kbps);
        let session = rt.block_on(Session::new_with_opts(staging, opts)).map_err(anyhow_str)?;

        // Read-only local HTTP endpoint: it exists so players can open
        // http://127.0.0.1:<port>/torrents/<id>/stream/<file>/<name> with range requests.
        let listener = {
            let _guard = rt.enter();
            let opts = BindOpts { request_dualstack: false, ..Default::default() };
            DualstackTcpListener::bind_tcp("127.0.0.1:0".parse().unwrap(), opts).map_err(|e| e.to_string())?
        };
        let stream_port = listener.bind_addr().port();
        let http = HttpApi::new(Api::new(session.clone(), None, None), Some(HttpApiOptions { read_only: true, ..Default::default() }));
        rt.spawn(async move {
            let _ = http.make_http_api_and_run(listener, None).await;
        });

        let dests_file = persist.join("dests.json");
        let dests = std::fs::read(&dests_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let api = Api::new(session.clone(), None, None);
        let engine = Arc::new(Engine {
            rt,
            session,
            stream_port,
            proxy,
            dir,
            dests: std::sync::Mutex::new(dests),
            dests_file,
            api,
            added: std::sync::Mutex::new(Default::default()),
            limits: (cfg.down_kbps, cfg.up_kbps),
        });
        // Torrents remembered from the last run: resume watching for completion.
        // Streams the user never kept or discarded wait, paused, for a decision.
        let handles: Vec<Arc<ManagedTorrent>> = engine.session.with_torrents(|it| it.map(|(_, t)| t.clone()).collect());
        for h in handles {
            let ephemeral = engine.dest_for(&h.info_hash().as_string()).map(|d| d.ephemeral).unwrap_or(false);
            if ephemeral {
                let _ = engine.rt.block_on(engine.session.pause(&h));
            } else {
                engine.watch(app.clone(), h);
            }
        }
        Ok(engine)
    }

    pub fn protected(&self) -> bool {
        self.proxy.is_some()
    }

    pub fn stop(self: Arc<Self>) {
        self.rt.block_on(self.session.stop());
        // The runtime shuts down in the background when the last Arc drops.
    }

    /// Turn the user's input into something librqbit accepts. In proxy mode
    /// UDP trackers are stripped from magnets, and .torrent files become
    /// magnets carrying only their HTTP trackers, since UDP announces would
    /// bypass the proxy.
    fn to_add(&self, source: &str) -> Result<AddTorrent<'static>, String> {
        let normalized = normalize_source(source);
        let source = normalized.as_str();
        if source.starts_with("magnet:") {
            let m = if self.protected() { strip_udp_trackers(source) } else { source.to_string() };
            return Ok(AddTorrent::from_url(m));
        }
        let bytes = std::fs::read(source).map_err(|e| format!("cannot read torrent file: {e}"))?;
        if !self.protected() {
            return Ok(AddTorrent::from_bytes(bytes));
        }
        let t = torrent_from_bytes(&bytes).map_err(|e| format!("not a valid .torrent file: {e}"))?;
        let name = t.info.data.name.as_ref().map(|n| String::from_utf8_lossy(n.as_ref()).to_string()).unwrap_or_default();
        let mut magnet = format!("magnet:?xt=urn:btih:{}&dn={}", t.info_hash.as_string(), urlencoding::encode(&name));
        for tr in t.iter_announce() {
            let tr = String::from_utf8_lossy(tr.as_ref());
            if tr.starts_with("http://") || tr.starts_with("https://") {
                magnet.push_str("&tr=");
                magnet.push_str(&urlencoding::encode(&tr));
            }
        }
        Ok(AddTorrent::from_url(magnet))
    }

    /// Resolve the file list without starting a download. For magnets this
    /// fetches the metadata from peers, which can take a while.
    pub fn inspect(&self, source: &str) -> Result<Preview, String> {
        let add = self.to_add(source)?;
        let opts = AddTorrentOptions { list_only: true, ..Default::default() };
        let resp = self.rt.block_on(self.session.add_torrent(add, Some(opts))).map_err(anyhow_str)?;
        match resp {
            AddTorrentResponse::ListOnly(l) => {
                let files: Vec<PreviewFile> = l
                    .info
                    .iter_file_details()
                    .enumerate()
                    .map(|(index, f)| {
                        let path = f.filename.to_vec().join("/");
                        let video = parser::is_video(Path::new(&path));
                        PreviewFile { index, path, size: f.len, video }
                    })
                    .collect();
                Ok(Preview {
                    name: l.info.name().map(|n| n.to_string()).unwrap_or_else(|| "Untitled".into()),
                    info_hash: l.info_hash.as_string(),
                    total_bytes: files.iter().map(|f| f.size).sum(),
                    files,
                })
            }
            AddTorrentResponse::AlreadyManaged(..) => Err("This torrent is already in your downloads.".into()),
            AddTorrentResponse::Added(..) => Err("unexpected: torrent started".into()),
        }
    }

    fn remember_dest(&self, info_hash: &str, dest: Option<Dest>) {
        if let Ok(mut map) = self.dests.lock() {
            match dest {
                Some(d) => {
                    map.insert(info_hash.to_string(), d);
                }
                None => {
                    map.remove(info_hash);
                }
            }
            if let Ok(json) = serde_json::to_vec(&*map) {
                let _ = std::fs::write(&self.dests_file, json);
            }
        }
    }

    fn dest_for(&self, info_hash: &str) -> Option<Dest> {
        self.dests.lock().ok().and_then(|m| m.get(info_hash).cloned())
    }

    pub fn add(
        self: &Arc<Self>,
        app: &AppHandle,
        source: &str,
        only_files: Vec<usize>,
        needed_bytes: u64,
        dest: Dest,
    ) -> Result<usize, String> {
        if only_files.is_empty() {
            return Err("Pick at least one file.".into());
        }
        let save_in = PathBuf::from(dest.save_in.trim());
        if dest.save_in.trim().is_empty() || !save_in.is_dir() {
            return Err(format!("Save-in folder is not available: {}", save_in.display()));
        }
        if let Some(free) = free_space(&save_in) {
            if needed_bytes > free {
                return Err(format!(
                    "Not enough free space: {} needed, {} free on that drive.",
                    human(needed_bytes),
                    human(free)
                ));
            }
        }
        let subfolder = dest.subfolder.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(safe_folder_name);
        let dest = Dest { save_in: save_in.to_string_lossy().to_string(), subfolder, ..dest };
        // Pieces are written under `<save in>\.incomplete` so the scanner
        // never sees a half-finished file, whichever drive the user picked.
        let staging = make_staging(&save_in)?;

        let add = self.to_add(source)?;
        let opts = AddTorrentOptions {
            only_files: Some(only_files),
            overwrite: true,
            output_folder: Some(staging.to_string_lossy().to_string()),
            ..Default::default()
        };
        let resp = self.rt.block_on(self.session.add_torrent(add, Some(opts))).map_err(anyhow_str)?;
        match resp {
            AddTorrentResponse::Added(id, handle) => {
                self.remember_dest(&handle.info_hash().as_string(), Some(dest));
                if let Ok(mut a) = self.added.lock() {
                    a.insert(id, std::time::Instant::now());
                }
                self.watch(app.clone(), handle);
                Ok(id)
            }
            AddTorrentResponse::AlreadyManaged(..) => Err("This torrent is already in your downloads.".into()),
            AddTorrentResponse::ListOnly(_) => Err("unexpected: list only".into()),
        }
    }

    pub fn list(&self) -> Vec<TorrentRow> {
        let handles: Vec<Arc<ManagedTorrent>> = self.session.with_torrents(|it| it.map(|(_, t)| t.clone()).collect());
        let mut rows: Vec<TorrentRow> = handles.iter().map(|t| self.row(t)).collect();
        rows.sort_by_key(|r| r.id);
        rows
    }

    fn row(&self, t: &Arc<ManagedTorrent>) -> TorrentRow {
        let stats = t.stats();
        let only = t.only_files();
        let files: Vec<TorrentFile> = t
            .with_metadata(|m| {
                m.file_infos
                    .iter()
                    .enumerate()
                    .map(|(index, f)| TorrentFile {
                        index,
                        path: f.relative_filename.to_string_lossy().replace('\\', "/"),
                        size: f.len,
                        done: stats.file_progress.get(index).copied().unwrap_or(0),
                        included: only.as_ref().map(|o| o.contains(&index)).unwrap_or(true),
                        video: parser::is_video(&f.relative_filename),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let state = match stats.state {
            TorrentStatsState::Initializing { .. } => "checking",
            TorrentStatsState::Paused => "paused",
            TorrentStatsState::Error => "error",
            TorrentStatsState::Live if stats.finished => "seeding",
            TorrentStatsState::Live => "downloading",
        };
        let live = stats.live.as_ref();
        TorrentRow {
            id: t.id(),
            name: t.name().unwrap_or_else(|| t.info_hash().as_string()),
            info_hash: t.info_hash().as_string(),
            state: state.into(),
            error: stats.error.clone(),
            done_bytes: stats.progress_bytes,
            total_bytes: stats.total_bytes,
            uploaded_bytes: stats.uploaded_bytes,
            down_mbps: live.map(|l| l.download_speed.mbps).unwrap_or(0.0),
            up_mbps: live.map(|l| l.upload_speed.mbps).unwrap_or(0.0),
            peers: live.map(|l| l.snapshot.peer_stats.live as usize).unwrap_or(0),
            eta: live.and_then(|l| l.time_remaining.as_ref().map(|t| t.to_string())),
            finished: stats.finished,
            ephemeral: self.dest_for(&t.info_hash().as_string()).map(|d| d.ephemeral).unwrap_or(false),
            files,
        }
    }

    pub fn pause(&self, id: usize) -> Result<(), String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        self.rt.block_on(self.session.pause(&t)).map_err(anyhow_str)
    }

    pub fn resume(&self, id: usize) -> Result<(), String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        self.rt.block_on(self.session.unpause(&t)).map_err(anyhow_str)
    }

    pub fn remove(&self, id: usize, delete_files: bool) -> Result<(), String> {
        self.rt.block_on(self.session.delete(TorrentIdOrHash::Id(id), delete_files)).map_err(anyhow_str)
    }

    pub fn detail(&self, id: usize) -> Result<TorrentDetail, String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        let stats = t.stats();
        let live = stats.live.as_ref();
        let hash = t.info_hash().as_string();
        let (piece_count, piece_length, created_on, created_by, comment) = t
            .with_metadata(|m| {
                let info = m.info.info();
                let pieces = (info.pieces.as_ref().len() / 20) as u32;
                let meta = torrent_from_bytes(&m.torrent_bytes).ok();
                let s = |b: &Option<librqbit::ByteBuf<'_>>| b.as_ref().map(|v| String::from_utf8_lossy(v.as_ref()).to_string());
                (
                    pieces,
                    info.piece_length,
                    meta.as_ref().and_then(|t| t.creation_date).map(|d| d as i64),
                    meta.as_ref().and_then(|t| s(&t.created_by)),
                    meta.as_ref().and_then(|t| s(&t.comment)),
                )
            })
            .unwrap_or((0, 0, None, None, None));
        let elapsed_secs = self.added.lock().ok().and_then(|a| a.get(&id).map(|i| i.elapsed().as_secs())).unwrap_or(0);
        let wasted = live
            .map(|l| l.snapshot.fetched_bytes.saturating_sub(l.snapshot.downloaded_and_checked_bytes))
            .unwrap_or(0);
        let peer_counts = live.map(|l| serde_json::to_value(&l.snapshot.peer_stats).unwrap_or_default()).unwrap_or_default();
        let save_as = self
            .dest_for(&hash)
            .map(|d| match d.subfolder {
                Some(s) => format!("{}\\{}", d.save_in.trim_end_matches(['\\', '/']), s),
                None => d.save_in,
            })
            .unwrap_or_else(|| self.dir.to_string_lossy().to_string());
        let status = match stats.state {
            TorrentStatsState::Initializing { .. } => "Checking",
            TorrentStatsState::Paused => "Paused",
            TorrentStatsState::Error => "Error",
            TorrentStatsState::Live if stats.finished => "Seeding",
            TorrentStatsState::Live => "Downloading",
        };
        let peers = self
            .api
            .api_peer_stats(TorrentIdOrHash::Id(id), Default::default())
            .map(|snap| {
                let mut v: Vec<PeerRow> = snap
                    .peers
                    .into_iter()
                    .map(|(addr, p)| PeerRow {
                        addr,
                        client: p.client_name,
                        state: p.state.to_string(),
                        kind: p.conn_kind.map(|k| format!("{k:?}").to_lowercase()),
                        downloaded: p.counters.fetched_bytes,
                        uploaded: p.counters.uploaded_bytes,
                    })
                    .collect();
                v.sort_by(|a, b| b.downloaded.cmp(&a.downloaded).then(a.addr.cmp(&b.addr)));
                v
            })
            .unwrap_or_default();
        let mut trackers: Vec<TrackerRow> = t
            .shared()
            .trackers
            .iter()
            .map(|u| {
                let protocol = match u.scheme() {
                    "http" | "https" | "udp" => u.scheme().to_string(),
                    _ => "other".into(),
                };
                let active = protocol != "udp" || !self.protected();
                TrackerRow { url: u.to_string(), protocol, active }
            })
            .collect();
        trackers.sort_by(|a, b| a.url.cmp(&b.url));
        Ok(TorrentDetail {
            id,
            elapsed_secs,
            downloaded: stats.progress_bytes,
            remaining: stats.total_bytes.saturating_sub(stats.progress_bytes),
            wasted,
            uploaded: stats.uploaded_bytes,
            down_mbps: live.map(|l| l.download_speed.mbps).unwrap_or(0.0),
            up_mbps: live.map(|l| l.upload_speed.mbps).unwrap_or(0.0),
            down_limit_kbps: self.limits.0,
            up_limit_kbps: self.limits.1,
            share_ratio: if stats.progress_bytes > 0 { stats.uploaded_bytes as f64 / stats.progress_bytes as f64 } else { 0.0 },
            status: status.into(),
            error: stats.error.clone(),
            peer_counts,
            save_as,
            total_size: stats.total_bytes,
            piece_count,
            piece_length,
            created_on,
            created_by,
            comment,
            info_hash: hash,
            peers,
            trackers,
        })
    }

    pub fn session_status(&self) -> SessionStatus {
        let snap = self.session.stats_snapshot();
        let v = serde_json::to_value(&snap).unwrap_or_default();
        let num = |path: &[&str]| -> u64 {
            let mut cur = &v;
            for p in path {
                cur = cur.get(p).unwrap_or(&serde_json::Value::Null);
            }
            cur.as_u64().unwrap_or(0)
        };
        let dht = match self.session.get_dht() {
            None if self.protected() => Some("Off (proxy mode)".to_string()),
            None => Some("Off".to_string()),
            Some(d) => {
                let s = d.stats();
                Some(format!("{} nodes", s.routing_table_size + s.routing_table_size_v6))
            }
        };
        SessionStatus {
            dht,
            down_mbps: snap.download_speed.mbps,
            up_mbps: snap.upload_speed.mbps,
            downloaded_total: num(&["counters", "fetched_bytes"]),
            uploaded_total: num(&["counters", "uploaded_bytes"]),
            peers_live: num(&["peers", "live"]),
            uptime_secs: snap.uptime_seconds,
            protected: self.protected(),
        }
    }

    /// "Stream": resolve the source, take its largest video file, start it
    /// as an ephemeral torrent in the default folder's staging area, wait
    /// for a head buffer, then open the player. Resumes where the same
    /// stream last stopped. Blocks for up to `STREAM_HEAD_TIMEOUT`.
    pub fn stream(self: &Arc<Self>, app: &AppHandle, source: &str) -> Result<StreamStarted, String> {
        let source = source.trim();
        // Finished and moved into the library earlier: play that, no swarm needed.
        if let Some(done) = source_hash(source).and_then(|h| self.dest_for(&h)).and_then(|d| d.completed_path) {
            if let Some(video) = largest_video_under(Path::new(&done)) {
                let name = video.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                let episode_id = {
                    let state = app.state::<AppState>();
                    let conn = state.db.lock().map_err(|e| e.to_string())?;
                    conn.query_row("SELECT id FROM episodes WHERE path = ?1", [video.to_string_lossy().as_ref()], |r| r.get::<_, i64>(0)).ok()
                };
                let tracked = match episode_id {
                    Some(ep) => crate::player::play(app.clone(), ep)?,
                    None => crate::player::play_url(app, &video.to_string_lossy())?,
                };
                return Ok(StreamStarted { id: 0, name, file: 0, start_secs: 0, tracked, from_library: true });
            }
        }
        // Already added (streamed before, or downloading): just play it.
        let existing = self.rt.block_on(async {
            let add = self.to_add(source).ok()?;
            match self.session.add_torrent(add, Some(AddTorrentOptions { list_only: true, ..Default::default() })).await {
                Ok(AddTorrentResponse::AlreadyManaged(id, _)) => Some(id),
                _ => None,
            }
        });
        let id = match existing {
            Some(id) => id,
            None => {
                let preview = self.inspect(source)?;
                let file = preview
                    .files
                    .iter()
                    .filter(|f| f.video)
                    .max_by_key(|f| f.size)
                    .ok_or("This torrent has no video file to stream.")?;
                let dest = Dest {
                    save_in: self.dir.to_string_lossy().to_string(),
                    subfolder: Some(preview.name.clone()),
                    ephemeral: true,
                    ..Default::default()
                };
                self.add(app, source, vec![file.index], file.size, dest)?
            }
        };
        self.stream_existing(app, id)
    }

    /// Play the largest selected video of a torrent already in the session,
    /// buffering first. Used by "Stream" and by the row's Play button.
    pub fn stream_existing(self: &Arc<Self>, app: &AppHandle, id: usize) -> Result<StreamStarted, String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        if t.is_paused() {
            self.rt.block_on(self.session.unpause(&t)).map_err(anyhow_str)?;
        }
        self.rt.block_on(t.wait_until_initialized()).map_err(anyhow_str)?;
        let only = t.only_files();
        let file = t
            .with_metadata(|m| {
                m.file_infos
                    .iter()
                    .enumerate()
                    .filter(|(i, f)| parser::is_video(&f.relative_filename) && only.as_ref().map(|o| o.contains(i)).unwrap_or(true))
                    .max_by_key(|(_, f)| f.len)
                    .map(|(i, f)| (i, f.len))
            })
            .map_err(anyhow_str)?
            .ok_or("This torrent has no selected video file.")?;
        self.stream_file(app, id, file.0, file.1)
    }

    fn stream_file(self: &Arc<Self>, app: &AppHandle, id: usize, file: usize, size: u64) -> Result<StreamStarted, String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        let hash = t.info_hash().as_string();
        let name = t.name().unwrap_or_default();
        let url = self.stream_url(id, file)?;

        // Head buffer so the player does not open onto an empty pipe.
        // librqbit fetches each file's first and last pieces first, so this
        // also covers MP4s whose index sits at the end.
        let want = (size as f64 * STREAM_HEAD_FRACTION) as u64;
        let want = want.min(STREAM_HEAD_BYTES).max(1);
        let started = std::time::Instant::now();
        loop {
            let done = t.stats().file_progress.get(file).copied().unwrap_or(0);
            if done >= want || t.stats().finished {
                break;
            }
            if started.elapsed() > STREAM_HEAD_TIMEOUT {
                let peers = t.stats().live.as_ref().map(|l| l.snapshot.peer_stats.live).unwrap_or(0);
                return Err(if peers == 0 {
                    "No peers found yet. Check the proxy setting, or try a torrent with more seeds.".into()
                } else {
                    "Still buffering after 90 seconds; the swarm is too slow to stream right now.".into()
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        let dest = self.dest_for(&hash);
        let start_secs = dest.as_ref().map(|d| d.position_secs).unwrap_or(0);
        let ephemeral = dest.as_ref().map(|d| d.ephemeral).unwrap_or(false);
        let engine = self.clone();
        let app2 = app.clone();
        let name2 = name.clone();
        let on_end: Box<dyn FnOnce(crate::player::StreamEnd) + Send> = Box::new(move |end| {
            let handle = engine.session.get(TorrentIdOrHash::Id(id));
            let finished = handle.as_ref().map(|t| t.stats().finished).unwrap_or(false);
            // A stream nobody has kept stops fetching when the player closes;
            // Keep or Watch starts it again. Downloads carry on regardless.
            if ephemeral && !finished {
                if let Some(t) = &handle {
                    let _ = engine.rt.block_on(engine.session.pause(t));
                }
            }
            // Resume point, unless the player ran to (near) the end.
            let near_end = end.duration_secs.map(|d| end.position_secs >= (d as f64 * 0.9) as i64).unwrap_or(false);
            if let Some(mut d) = engine.dest_for(&hash) {
                d.position_secs = if near_end { 0 } else { end.position_secs };
                d.duration_secs = end.duration_secs.or(d.duration_secs);
                engine.remember_dest(&hash, Some(d));
            }
            let _ = app2.emit(
                "stream-ended",
                StreamEnded { id, name: name2, position_secs: end.position_secs, duration_secs: end.duration_secs, finished, ephemeral, exact: end.exact },
            );
        });
        let tracked = crate::player::play_url_tracked(app, &url, start_secs, Some(on_end))?;
        Ok(StreamStarted { id, name, file, start_secs, tracked, from_library: false })
    }

    /// Keep a stream: it becomes an ordinary download and moves into the
    /// library when complete (immediately, if it already is).
    pub fn keep(self: &Arc<Self>, app: &AppHandle, id: usize) -> Result<(), String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        let hash = t.info_hash().as_string();
        if let Some(mut d) = self.dest_for(&hash) {
            d.ephemeral = false;
            self.remember_dest(&hash, Some(d));
        }
        if t.is_paused() {
            self.rt.block_on(self.session.unpause(&t)).map_err(anyhow_str)?;
        }
        self.watch(app.clone(), t);
        Ok(())
    }

    /// Discard a stream: forget it and delete what was downloaded.
    pub fn discard(&self, id: usize) -> Result<(), String> {
        if let Some(t) = self.session.get(TorrentIdOrHash::Id(id)) {
            self.remember_dest(&t.info_hash().as_string(), None);
        }
        self.remove(id, true)
    }

    /// Local URL a player can open. Only video files are ever handed out.
    pub fn stream_url(&self, id: usize, file: usize) -> Result<String, String> {
        let t = self.session.get(TorrentIdOrHash::Id(id)).ok_or("torrent not found")?;
        let name = t
            .with_metadata(|m| m.file_infos.get(file).map(|f| f.relative_filename.clone()))
            .map_err(anyhow_str)?
            .ok_or("file not found in torrent")?;
        if !parser::is_video(&name) {
            return Err("Only video files can be played.".into());
        }
        if let Some(only) = t.only_files() {
            if !only.contains(&file) {
                return Err("This file is not selected for download.".into());
            }
        }
        if t.is_paused() {
            self.rt.block_on(self.session.unpause(&t)).map_err(anyhow_str)?;
        }
        let base = name.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Ok(format!("http://127.0.0.1:{}/torrents/{id}/stream/{file}/{}", self.stream_port, urlencoding::encode(&base)))
    }

    /// When the torrent finishes: forget it, move the files up into the
    /// library folder and let the scanner take it from there.
    fn watch(self: &Arc<Self>, app: AppHandle, t: Arc<ManagedTorrent>) {
        let engine = self.clone();
        self.rt.spawn(async move {
            if t.wait_until_completed().await.is_err() {
                return; // removed before it finished
            }
            let id = t.id();
            if engine.session.get(TorrentIdOrHash::Id(id)).is_none() {
                return;
            }
            let name = t.name().unwrap_or_default();
            // A stream stays put until the user keeps it; `keep` re-arms this watcher.
            if engine.dest_for(&t.info_hash().as_string()).map(|d| d.ephemeral).unwrap_or(false) {
                return;
            }
            let only = t.only_files();
            let tops: Vec<PathBuf> = t
                .with_metadata(|m| {
                    let mut v: Vec<PathBuf> = Vec::new();
                    for (i, f) in m.file_infos.iter().enumerate() {
                        if only.as_ref().map(|o| !o.contains(&i)).unwrap_or(false) {
                            continue;
                        }
                        if let Some(first) = f.relative_filename.components().next() {
                            let p = PathBuf::from(first.as_os_str());
                            if !v.contains(&p) {
                                v.push(p);
                            }
                        }
                    }
                    v
                })
                .unwrap_or_default();
            let from = t.output_folder().to_path_buf();
            let hash = t.info_hash().as_string();
            // Torrents added before per-torrent destinations existed go to the library folder.
            let dest = engine.dest_for(&hash).unwrap_or_else(|| Dest {
                save_in: engine.dir.to_string_lossy().to_string(),
                subfolder: Some(safe_folder_name(&name)),
                ..Default::default()
            });
            // Forget (keep files) so nothing holds the files open while they move.
            let _ = engine.session.delete(TorrentIdOrHash::Id(id), false).await;
            let (moved, root) = move_finished(&from, &tops, &dest);
            tidy_staging(&from);
            // Remember where it went, so streaming the same link later plays
            // the finished file instead of fetching it again.
            engine.remember_dest(
                &hash,
                root.map(|r| Dest { completed_path: Some(r.to_string_lossy().to_string()), ephemeral: false, ..dest }),
            );
            let _ = app.emit("torrent-done", TorrentDone { name, moved });
            jobs::refresh_async(app, "torrent");
        });
    }
}

/// A bare info hash (40 hex or 32 base32 characters) becomes a magnet link;
/// anything else is returned trimmed.
pub fn normalize_source(source: &str) -> String {
    let s = source.trim();
    let is_hex = s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit());
    let is_b32 = s.len() == 32 && s.chars().all(|c| c.is_ascii_alphanumeric());
    if is_hex || is_b32 {
        format!("magnet:?xt=urn:btih:{s}")
    } else {
        s.to_string()
    }
}

/// Info hash of a magnet link or .torrent file, as lowercase hex.
fn source_hash(source: &str) -> Option<String> {
    let normalized = normalize_source(source);
    let source = normalized.as_str();
    if source.starts_with("magnet:") {
        return librqbit::Magnet::parse(source).ok()?.as_id20().map(|h| h.as_string());
    }
    let bytes = std::fs::read(source).ok()?;
    torrent_from_bytes(&bytes).ok().map(|t| t.info_hash.as_string())
}

/// The biggest video file at or under a path.
fn largest_video_under(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        return parser::is_video(p).then(|| p.to_path_buf());
    }
    walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && parser::is_video(e.path()))
        .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .map(|e| e.into_path())
}

/// Move a finished torrent's top-level entries out of staging. With a
/// subfolder, everything goes into `<save in>\<subfolder>\`; a torrent that
/// already wraps its files in one top-level folder has that folder renamed
/// rather than nested. Without one, entries land in `<save in>` directly.
/// Returns the names of what was moved and the path it all lives under
/// (the subfolder, or the single entry when there is no subfolder).
fn move_finished(from: &Path, tops: &[PathBuf], dest: &Dest) -> (Vec<String>, Option<PathBuf>) {
    let save_in = PathBuf::from(&dest.save_in);
    let Some(folder_name) = dest.subfolder.as_deref() else {
        let moved = move_entries(from, tops, &save_in);
        let root = match moved.as_slice() {
            [one] => Some(save_in.join(one)),
            [] => None,
            _ => Some(save_in),
        };
        return (moved, root);
    };
    let dest_dir = unique_dest(&save_in.join(folder_name));
    let single_dir = tops.len() == 1 && from.join(&tops[0]).is_dir();
    if single_dir {
        let src = from.join(&tops[0]);
        return match std::fs::rename(&src, &dest_dir) {
            Ok(()) => (vec![dest_dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()], Some(dest_dir)),
            Err(_) => (Vec::new(), None),
        };
    }
    if std::fs::create_dir_all(&dest_dir).is_err() {
        return (Vec::new(), None);
    }
    let moved = move_entries(from, tops, &dest_dir);
    if moved.is_empty() {
        let _ = std::fs::remove_dir(&dest_dir);
        return (moved, None);
    }
    (moved, Some(dest_dir))
}

fn move_entries(from: &Path, tops: &[PathBuf], dest_dir: &Path) -> Vec<String> {
    let mut moved = Vec::new();
    for top in tops {
        let src = from.join(top);
        if !src.exists() {
            continue;
        }
        let dest = unique_dest(&dest_dir.join(top));
        if std::fs::rename(&src, &dest).is_ok() {
            moved.push(dest.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
        }
    }
    moved
}

/// Torrent names can carry characters Windows will not accept in a folder.
fn safe_folder_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 32 { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').trim().to_string();
    if trimmed.is_empty() { "Download".to_string() } else { trimmed }
}

fn unique_dest(p: &Path) -> PathBuf {
    if !p.exists() {
        return p.to_path_buf();
    }
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for n in 2..100 {
        let c = p.with_file_name(format!("{stem} ({n}){ext}"));
        if !c.exists() {
            return c;
        }
    }
    p.to_path_buf()
}

fn strip_udp_trackers(magnet: &str) -> String {
    let Some((head, query)) = magnet.split_once('?') else { return magnet.to_string() };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|part| {
            let Some(v) = part.strip_prefix("tr=") else { return true };
            !urlencoding::decode(v).map(|d| d.starts_with("udp:")).unwrap_or(false)
        })
        .collect();
    format!("{head}?{}", kept.join("&"))
}

fn free_space(dir: &Path) -> Option<u64> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

fn human(b: u64) -> String {
    let gb = b as f64 / 1_073_741_824.0;
    if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", b as f64 / 1_048_576.0)
    }
}

/// The running engine, started on first use. `None` until something needs it.
pub fn current(app: &AppHandle) -> Option<Arc<Engine>> {
    app.state::<AppState>().torrent.lock().ok().and_then(|g| g.clone())
}

pub fn ensure(app: &AppHandle) -> Result<Arc<Engine>, String> {
    let state = app.state::<AppState>();
    let mut guard = state.torrent.lock().map_err(|e| e.to_string())?;
    if let Some(e) = guard.as_ref() {
        return Ok(e.clone());
    }
    let engine = Engine::start(app, &read_config(app))?;
    *guard = Some(engine.clone());
    Ok(engine)
}

/// Stop and start again with the current settings (proxy, folder, limits).
pub fn restart(app: &AppHandle) -> Result<Status, String> {
    let state = app.state::<AppState>();
    let old = state.torrent.lock().map_err(|e| e.to_string())?.take();
    if let Some(e) = old {
        e.stop();
    }
    let result = ensure(app);
    Ok(status(app, result.err()))
}

pub fn status(app: &AppHandle, error: Option<String>) -> Status {
    let config = read_config(app);
    let engine = current(app);
    Status {
        running: engine.is_some(),
        protected: engine.as_ref().map(|e| e.protected()).unwrap_or(!config.proxy.is_empty()),
        config,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_udp_trackers() {
        let m = "magnet:?xt=urn:btih:abc&dn=x&tr=udp%3A%2F%2Ft.example%3A80&tr=http%3A%2F%2Fh.example%2Fa&tr=https%3A%2F%2Fs.example";
        let out = strip_udp_trackers(m);
        assert!(!out.contains("udp"));
        assert!(out.contains("tr=http%3A%2F%2Fh.example%2Fa"));
        assert!(out.contains("tr=https%3A%2F%2Fs.example"));
        assert!(out.starts_with("magnet:?xt=urn:btih:abc&dn=x"));
    }

    #[test]
    fn bare_hashes_become_magnets() {
        let hex = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(normalize_source(&format!("  {hex} ")), format!("magnet:?xt=urn:btih:{hex}"));
        assert_eq!(normalize_source("magnet:?xt=urn:btih:abc"), "magnet:?xt=urn:btih:abc");
        assert_eq!(normalize_source("F:\\Torrents\\x.torrent"), "F:\\Torrents\\x.torrent");
    }

    #[test]
    fn staging_is_removed_only_when_empty() {
        let root = std::env::temp_dir().join(format!("vortex-st-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let staging = make_staging(&root).unwrap();
        assert!(staging.is_dir());

        std::fs::write(staging.join("part.mkv"), b"x").unwrap();
        tidy_staging(&staging);
        assert!(staging.is_dir(), "a torrent is still using it");

        std::fs::remove_file(staging.join("part.mkv")).unwrap();
        tidy_staging(&staging);
        assert!(!staging.exists(), "empty staging should be gone");

        // Never touches anything that is not the staging folder.
        tidy_staging(&root);
        assert!(root.is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn safe_folder_names() {
        assert_eq!(safe_folder_name("Show: S01 [1080p]?"), "Show  S01 [1080p]");
        assert_eq!(safe_folder_name("Trailing dots..."), "Trailing dots");
        assert_eq!(safe_folder_name("  "), "Download");
    }

    #[test]
    fn finished_files_land_in_a_named_folder() {
        let root = std::env::temp_dir().join(format!("vortex-mv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let lib = root.join("lib");
        let staging = lib.join(STAGING_DIR);
        std::fs::create_dir_all(&staging).unwrap();

        let with = |sub: &str| Dest { save_in: lib.to_string_lossy().to_string(), subfolder: Some(sub.to_string()), ..Default::default() };
        let flat = Dest { save_in: lib.to_string_lossy().to_string(), subfolder: None, ..Default::default() };

        // Single-file torrent with "Create subfolder": the bare file gets its own folder.
        std::fs::write(staging.join("Movie.mkv"), b"x").unwrap();
        let (moved, at) = move_finished(&staging, &[PathBuf::from("Movie.mkv")], &with("Movie (2026) [1080p]"));
        assert_eq!(moved, vec!["Movie.mkv"]);
        assert_eq!(at, Some(lib.join("Movie (2026) [1080p]")));
        assert!(lib.join("Movie (2026) [1080p]").join("Movie.mkv").is_file());

        // Multi-file torrent already wrapped in one folder: renamed, not nested.
        std::fs::create_dir_all(staging.join("Pack").join("Subs")).unwrap();
        std::fs::write(staging.join("Pack").join("E01.mkv"), b"x").unwrap();
        let (moved, at) = move_finished(&staging, &[PathBuf::from("Pack")], &with("Show S01"));
        assert_eq!(moved, vec!["Show S01"]);
        assert_eq!(at, Some(lib.join("Show S01")));
        assert!(lib.join("Show S01").join("E01.mkv").is_file());
        assert!(!lib.join("Show S01").join("Pack").exists());

        // Same subfolder name twice: a counter keeps them apart.
        std::fs::write(staging.join("Movie.mkv"), b"y").unwrap();
        let (moved, at) = move_finished(&staging, &[PathBuf::from("Movie.mkv")], &with("Movie (2026) [1080p]"));
        assert_eq!(moved, vec!["Movie.mkv"]);
        assert_eq!(at, Some(lib.join("Movie (2026) [1080p] (2)")));
        assert!(lib.join("Movie (2026) [1080p] (2)").join("Movie.mkv").is_file());

        // "Create subfolder" off: entries land directly in Save in.
        std::fs::write(staging.join("Loose.mkv"), b"z").unwrap();
        let (moved, at) = move_finished(&staging, &[PathBuf::from("Loose.mkv")], &flat);
        assert_eq!(moved, vec!["Loose.mkv"]);
        assert_eq!(at, Some(lib.join("Loose.mkv")));
        assert!(lib.join("Loose.mkv").is_file());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unique_dest_appends_a_counter() {
        let dir = std::env::temp_dir().join(format!("vortex-ud-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("Movie.mkv");
        assert_eq!(unique_dest(&p), p);
        std::fs::write(&p, b"x").unwrap();
        assert_eq!(unique_dest(&p), dir.join("Movie (2).mkv"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A session starts in both modes and the stream endpoint binds. This is
    /// the librqbit integration that matters; it needs no network.
    #[test]
    fn session_starts_in_both_modes() {
        for proxy in [None, Some("socks5://127.0.0.1:1080".to_string())] {
            let root = std::env::temp_dir().join(format!("vortex-tt-{}-{}", std::process::id(), proxy.is_some()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("persist")).unwrap();
            let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
            let opts = session_options(proxy.clone(), root.join("persist"), 512, 0);
            let session = rt.block_on(Session::new_with_opts(root.join("dl"), opts)).expect("session starts");
            if proxy.is_some() {
                assert!(session.get_dht().is_none(), "DHT must be off behind a proxy");
                assert!(session.listen_addr().is_none(), "no listener behind a proxy");
            } else {
                assert!(session.get_dht().is_some());
            }
            let listener = {
                let _g = rt.enter();
                DualstackTcpListener::bind_tcp("127.0.0.1:0".parse().unwrap(), BindOpts { request_dualstack: false, ..Default::default() }).unwrap()
            };
            assert!(listener.bind_addr().port() > 0);
            let http = HttpApi::new(Api::new(session.clone(), None, None), Some(HttpApiOptions { read_only: true, ..Default::default() }));
            let port = listener.bind_addr().port();
            rt.spawn(async move { let _ = http.make_http_api_and_run(listener, None).await; });
            // The read-only API answers on the loopback port.
            let body = rt.block_on(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                reqwest::get(format!("http://127.0.0.1:{port}/torrents")).await.unwrap().text().await.unwrap()
            });
            assert!(body.contains("torrents"), "unexpected body: {body}");
            rt.block_on(session.stop());
            drop(rt);
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}
