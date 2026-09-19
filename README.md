# Vortex

A local media library for Windows that remembers where you paused in every movie and episode.

Vortex catalogues the films and series scattered across your PC and external drives, launches them in the player you already use (PotPlayer, VLC, mpv, MPC-HC), and tracks the exact second you stopped. No streaming account, no server, no cloud. Everything stays on your disk.

## Features

- **Exact resume.** Talks to the running player: PotPlayer via window messages, VLC via its HTTP interface, mpv via IPC. Position saved every 15 s; episode marked watched at 90%; next episode starts automatically or via a 10-second "Up next" card.
- **Understands messy folders.** Groups every season under one title, keeps featurettes and trailers as extras, carries progress across renames, skips system folders so a whole drive can be added.
- **TMDB details, optional.** Posters, backdrops, ratings, genres, cast, trailer, episode titles and stills, cached locally after one fetch. Film series become collections in release order.
- **Library tools.** Collections, tags, smart lists, duplicate finder with Recycle Bin delete, history log, statistics, global search, keyboard navigation.
- **Runs quietly.** System tray, folder watcher with Windows notifications, rescan on startup, one-file backup and restore.

## Stack

Tauri 2 + Rust (scanning, playback tracking, file watching, SQLite) · Vue 3 + TypeScript · Tailwind v4 + shadcn-vue · SQLite via rusqlite.

## Development

Requirements: Node 20+, Rust stable, and the Tauri prerequisites for Windows (WebView2, MSVC build tools).

```bash
npm install
npm run tauri dev
```

The first Rust compile takes a few minutes. The dev server uses port 3420 (Windows reserves the Tauri default of 1420 on many machines).

Backend tests:

```bash
cd src-tauri && cargo test
```

Build an installer:

```bash
npm run tauri build
```

## Setup after first launch

1. Settings → **Add folder** or **Add whole drive**. Scanning starts immediately.
2. The player installed on your PC is selected automatically; change it under **Video player**.
3. Optionally paste a free [TMDB API key](https://www.themoviedb.org/settings/api) under **Posters & details**.

## Player support

| Player | Resume | Position tracking | Auto-play next |
|---|---|---|---|
| PotPlayer | Yes | Exact | Same window |
| VLC | Yes | Exact | Same window |
| mpv | Yes | Exact | Countdown |
| MPC-HC | Yes | Estimated | Countdown |
| Other | No | Estimated | Countdown |

## Keyboard

`/` or `Ctrl+K` search · arrows move between cards · `Enter` open · `P` play · `Esc` back · `Home` / `End` first and last card.

## Data

Database and cached images live in `%APPDATA%\com.admin.vortex\`. Backups are a single zip made from Settings. The TMDB key is stored in the database and never sent to the UI.

This product uses the TMDB API but is not endorsed or certified by TMDB.
