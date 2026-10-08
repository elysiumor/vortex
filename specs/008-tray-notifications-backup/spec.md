# 008. Tray, notifications, backup and restore

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; restore hardened 2026-10-08, `dd4006d`, `8b647f9`) — written from the code on 2026-10-08 |
| **Area** | Shell · Settings |
| **Roadmap** | Done: Tray, notifications, backup and restore |
| **Design doc** | `docs/FEATURES.md` §20 Tray and window, §21 Backup and restore, §3 (arrival toasts) |

## Problem

A library app is useful only if it is running when files arrive and when the user wants to continue something; it should not be a window to manage. And the database holds months of positions and history that a reinstall or a new PC must not lose.

## Goals

- Vortex keeps running in the tray; closing the window hides it; the tray menu continues the last three things directly.
- New arrivals found by the watcher raise a Windows notification.
- One file backs up everything (database and posters); restoring it is safe even when it fails half way.

## Non-goals

- Cloud sync; scheduled backups; restoring onto a different library layout automatically.

## Where it lives

| Place | What |
| --- | --- |
| System tray | Icon, menu |
| Window close | Hide to tray (switchable) |
| Settings → Appearance & behaviour | Keep running in tray, notifications, Quit |
| Settings → Backup, restore & reset | Back up now, Restore…, last backup, Reset all watch data |

## Requirements

- **FR-1.** A tray icon must show the window on left click and offer a menu: up to three Continue watching entries (title, episode code, paused time; disabled when offline) that play on click, Open Vortex, Rescan libraries, Quit. The entries must stay current.
- **FR-2.** Closing the window must hide it to the tray when the setting is on (default), otherwise quit. Quit must be available from the tray and from Settings.
- **FR-3.** A second launch of the app must bring the running window to the front instead of opening another.
- **FR-4.** When the watcher, a returning drive or a finished download adds titles, a Windows notification must list them (first six, "… and N more"), switchable in Settings.
- **FR-5.** Back up now must write one zip chosen by the user holding a consistent copy of the database and every cached image, and remember the date and path.
- **FR-6.** Restore must take such a zip, check it is a Vortex backup, replace the live database and images, keep the previous database aside, and refuse while a poster fetch or duration check runs. Any failure must leave the current library in place and working.
- **FR-7.** After a restore, posters must show even when the backup was made under another Windows user or drive.
- **FR-8.** Reset all watch data must, after a confirm, clear every watched mark, position and history entry and nothing else.

## Rules and edge cases

- The tray menu is rebuilt after scans, playback, dismissals, resets and restores.
- The restore keeps `vortex.db.before-restore` and one older copy; the live connection is swapped under the lock with scans held off.
- The backup's database copy is a `VACUUM INTO` snapshot taken on its own connection so the app does not freeze meanwhile.
- A clean quit is logged; a log that just stops is a crash.

## Acceptance criteria

- **AC-1** (FR-1). With two paused episodes, the tray menu lists them with times; clicking one starts the player.
- **AC-2** (FR-2, FR-3). Close the window: the app stays in the tray; launch it again: the same window comes to the front.
- **AC-3** (FR-4). Copy a film into a watched folder: a Windows notification "New in your library" names it.
- **AC-4** (FR-5–FR-7). Back up, change a position, restore: the old position is back, posters show, `vortex.db.before-restore` exists in the data folder.
- **AC-5** (FR-6). Restore a zip without `vortex.db`: an error, and the library is unchanged and usable without restart.

## Principles

- Respects all principles. P5: restore never leaves an empty library behind.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08 (`dd4006d`, `8b647f9`): a failed restore no longer leaves an empty library; backups no longer freeze the app; restore repoints poster paths and waits for scans.
- 2026-10-08: Spec written from the code. Done.
