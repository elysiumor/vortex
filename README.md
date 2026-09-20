# Vortex

A local media library for Windows that remembers where you paused in every movie and episode.

Vortex catalogues the films and series scattered across your PC and external drives, launches them in the player you already use (PotPlayer, VLC, mpv, MPC-HC), and tracks the exact second you stopped. No streaming account, no server, no cloud. Everything stays on your disk.

## Features

- **Exact resume.** Talks to the running player: PotPlayer via window messages, VLC via its HTTP interface, mpv via IPC. Position saved every 15 s; episode marked watched at 90%; next episode starts automatically or via a 10-second "Up next" card.
- **Understands messy folders.** Groups every season under one title, keeps featurettes and trailers as extras, carries progress across renames, skips system folders so a whole drive can be added.
- **TMDB details, optional.** Posters, backdrops, ratings, genres, cast, trailer, episode titles and stills, cached locally after one fetch. Film series become collections in release order.
- **Library tools.** Collections, tags, smart lists, duplicate finder with Recycle Bin delete, history log, statistics, global search, keyboard navigation.
- **Runs quietly.** System tray, folder watcher with Windows notifications, rescan on startup, one-file backup and restore.
- **Downloads, bring your own links.** Paste a magnet link or open a .torrent file, pick which files you want, stream a video in your player while it downloads, and find it in your library when it finishes. There is no search and no index: Vortex is a transport, not a source. Optional SOCKS5 proxy support routes every connection through your VPN provider's endpoint and switches off everything that cannot be proxied.

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

## Downloads

Vortex bundles a BitTorrent engine ([librqbit](https://github.com/ikatson/rqbit)) but deliberately no way to find content. You bring a magnet link or a .torrent file; Vortex downloads it into the library folder you choose, then the normal scan, parse and TMDB match take over.

- **Choose files.** Video files are ticked by default. Nothing in a torrent is ever run, and only video files can be opened in the player.
- **Stream in one step.** Paste a link and press *Stream*: Vortex resolves it, takes the largest video, buffers the first few megabytes (head and tail first, so MP4 files with a trailing index start promptly) and opens your player. Each file is served on a private `127.0.0.1` port with range requests; PotPlayer, VLC, mpv and MPC-HC open it like any URL and pieces near the playback position are fetched first. Streams remember where you stopped and resume there.
- **Magnet links from your browser.** Switch on *Open magnet links with Vortex* in Settings → Downloads and clicking a magnet link anywhere brings Vortex up and streams it, the same way qBittorrent registers itself. A bare info hash pasted into the box works too. Switch it off to hand `magnet:` back to whatever client had it.
- **Keep or discard.** A stream is not a download until you say so. When the player closes, Vortex asks: *Keep* turns it into a normal download that lands in your library, *Discard* deletes what was fetched, *Decide later* leaves it paused with a Watch button. Downloads started the ordinary way are never asked.
- **Privacy.** Without a proxy the engine behaves like any torrent client: every peer sees your IP address, and the Downloads page says so. With a SOCKS5 proxy (`socks5://user:pass@host:port`, most VPN providers offer one that only answers inside the tunnel) every connection goes through it, and DHT, uTP, local discovery, incoming connections and UDP trackers are switched off because they cannot be proxied. Torrents that only carry UDP trackers will not find peers in that mode.
- **Where files go.** Each download picks a *Save in* folder (default: the one in Settings) and whether to create a subfolder, named after the torrent by default. In progress: `<save in>\.incomplete` (ignored by the scanner and watcher). Finished: moved into place, at which point seeding stops.
- **Limits.** Download and upload caps in KB/s, and a free-space check before a download starts.
