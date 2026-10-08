# 008. Tray, notifications, backup and restore: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`tray.rs` builds a Tauri tray menu from `continue_watching(3)` and rebuilds it on demand. The single-instance plugin fronts the window. Notifications go through `tauri-plugin-notification` from `jobs::scan_once`. `backup.rs` zips a `VACUUM INTO` snapshot plus `posters\`; restore validates, swaps the connection under the lock inside `jobs::exclusive`, and rolls back on any error.

## Data

| Table / key / file | Change |
| --- | --- |
| settings | `close_to_tray`, `notify_new`, `last_backup`, `last_backup_path` |
| data folder | `backup-snapshot.db` (temp), `restore-incoming.db` (temp), `vortex.db.before-restore`, `.before-restore.1` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `tray.rs` | `build`, `menu`, `rebuild`, `show_window`. |
| `lib.rs` | single-instance plugin; `on_window_event` close-to-tray; `quit_app`. |
| `jobs.rs` | arrival notification in `scan_once`. |
| `backup.rs` | `create`, `restore`. |
| `db.rs` | `reset_watch_data`, `hide_all_from_home`. |
| `commands.rs` | `create_backup`, `restore_backup`, `reset_watch_data`, `quit_app`. |

## Events and commands

- `library-restored`. UI: `api.createBackup`, `api.restoreBackup`, `api.resetWatchData`, `api.quitApp`, `api.onLibraryRestored`.

## UI (`src`)

- `SettingsView.vue`: flags, Quit, Backup card with confirm dialogs; `App.vue` closes any open title on `library-restored`.

## Cross-feature effects

- The tray mirrors Continue watching (003); restore triggers image-name migration (004).

## Risks

- A background job holding its own connection during a restore would keep the file open on Windows; the restore refuses while fetch/probe run and holds the scan job.

## Design doc updates

- `docs/FEATURES.md` §20, §21.
