use crate::{db, jobs, player, AppState};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main";

pub fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    let items = {
        let state = app.state::<AppState>();
        let guard = state.db.lock();
        match guard {
            Ok(conn) => db::continue_watching(&conn, 3).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    };
    if items.is_empty() {
        let none = MenuItem::with_id(app, "none", "Nothing in progress", false, None::<&str>)?;
        menu.append(&none)?;
    } else {
        for it in items {
            let mut label = it.title.clone();
            if it.kind == "series" {
                if let (Some(s), Some(e)) = (it.episode.season, it.episode.episode) {
                    label.push_str(&format!("  S{s:02}E{e:02}"));
                }
            }
            if it.episode.position_secs > 0 {
                let p = it.episode.position_secs;
                label.push_str(&format!("  ({:02}:{:02})", p / 60, p % 60));
            }
            let id = format!("play:{}", it.episode.id);
            let item = MenuItem::with_id(app, id, &label, it.episode.available, None::<&str>)?;
            menu.append(&item)?;
        }
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "open", "Open Vortex", true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "rescan", "Rescan libraries", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?)?;
    Ok(menu)
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Vortex")
        .menu(&menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "open" => show_window(app),
                "rescan" => jobs::refresh_async(app.clone(), "tray"),
                "quit" => app.exit(0),
                _ => {
                    if let Some(ep) = id.strip_prefix("play:").and_then(|s| s.parse::<i64>().ok()) {
                        let _ = player::play(app.clone(), ep);
                    }
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Rebuild the menu so the "continue watching" entries stay current.
pub fn rebuild(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(m) = menu(app) {
            let _ = tray.set_menu(Some(m));
        }
    }
}
