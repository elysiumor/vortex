//! `magnet:` and `vortex://` links. Clicking one in a browser opens (or
//! fronts) Vortex and starts the Stream flow on the Downloads page. Vortex
//! only ever registers `magnet:` when the user switches it on in Settings,
//! since that takes the association away from whatever client had it.

use crate::{db, tray, AppState};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

/// URLs that arrived before the page was listening (app launch by link).
pub struct Pending(pub Mutex<Vec<String>>);

/// A magnet link out of whatever URL form arrived.
pub fn magnet_from(url: &str) -> Option<String> {
    let url = url.trim();
    if url.starts_with("magnet:") {
        return Some(url.to_string());
    }
    // vortex://stream?magnet=<encoded> or vortex://stream?url=<encoded>
    let (_, query) = url.split_once('?')?;
    for part in query.split('&') {
        let (k, v) = part.split_once('=')?;
        if k == "magnet" || k == "url" {
            let v = urlencoding::decode(v).ok()?.to_string();
            if v.starts_with("magnet:") {
                return Some(v);
            }
        }
    }
    None
}

fn deliver(app: &AppHandle, urls: Vec<String>) {
    let magnets: Vec<String> = urls.iter().filter_map(|u| magnet_from(u)).collect();
    if magnets.is_empty() {
        return;
    }
    tray::show_window(app);
    if let Some(p) = app.try_state::<Pending>() {
        if let Ok(mut v) = p.0.lock() {
            v.extend(magnets.iter().cloned());
        }
    }
    for m in magnets {
        let _ = app.emit("open-url", m);
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    app.manage(Pending(Mutex::new(Vec::new())));
    // Our own scheme is always ours; in dev builds nothing registered it at install.
    let _ = app.deep_link().register("vortex");
    let wants_magnet = {
        let state = app.state::<AppState>();
        let guard = state.db.lock();
        matches!(guard.ok().and_then(|c| db::get_setting(&c, "magnet_handler").ok().flatten()).as_deref(), Some("1"))
    };
    if wants_magnet {
        let _ = app.deep_link().register("magnet");
    }
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        deliver(&handle, event.urls().iter().map(|u| u.to_string()).collect());
    });
    // Launched by a link: the plugin already parsed it from argv.
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        deliver(app, urls.iter().map(|u| u.to_string()).collect());
    }
    Ok(())
}

pub fn set_magnet_handler(app: &AppHandle, on: bool) -> Result<(), String> {
    let r = if on { app.deep_link().register("magnet") } else { app.deep_link().unregister("magnet") };
    r.map_err(|e| e.to_string())
}

/// Hand over and clear anything that arrived before the page listened.
pub fn take_pending(app: &AppHandle) -> Vec<String> {
    app.try_state::<Pending>().and_then(|p| p.0.lock().ok().map(|mut v| std::mem::take(&mut *v))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::magnet_from;

    #[test]
    fn extracts_magnets() {
        assert_eq!(magnet_from("magnet:?xt=urn:btih:abc").as_deref(), Some("magnet:?xt=urn:btih:abc"));
        assert_eq!(magnet_from("vortex://stream?magnet=magnet%3A%3Fxt%3Durn%3Abtih%3Aabc").as_deref(), Some("magnet:?xt=urn:btih:abc"));
        assert_eq!(magnet_from("vortex://stream?url=https%3A%2F%2Fx"), None);
        assert_eq!(magnet_from("vortex://stream"), None);
    }
}
