# 009. Downloads and streaming, magnet links from the browser

| | |
| --- | --- |
| **Status** | Done (2026-09-20, `7b74654`; 1.1.0–1.1.6 fixes 2026-09-23/25; review fixes 2026-10-08) — written from the code on 2026-10-08 |
| **Area** | Downloads · Settings |
| **Roadmap** | Done: Downloads and streaming |
| **Design doc** | `docs/FEATURES.md` §18 Downloads, §19 Deep links |

## Problem

The user already has links (magnets, .torrent files). Getting the file into the library means a separate client, a download folder the library does not watch, and a wait before anything can be watched. They want to paste a link and watch, decide afterwards whether to keep it, and have finished downloads appear in the library like any other file — while their VPN's SOCKS5 endpoint carries every connection so nothing leaks.

## Goals

- Paste a magnet or open a .torrent, pick files and where they go, and download into the library.
- Stream: one button from link to the player, resuming where the user stopped, with Keep / Discard afterwards.
- A finished download moves into the chosen library folder and is scanned.
- With a proxy, nothing bypasses it; without one, the user is told plainly, once.
- Magnet links clicked in a browser open Vortex and stream.

## Non-goals

- Any search, index or catalogue of content (D-004).
- Seeding after completion, seed-ratio limits, scheduling (roadmap).
- Adapter binding on Windows (parked).

## Where it lives

| Place | What |
| --- | --- |
| Downloads page | Link box, Stream, Download…, Import Torrent File, rows with tabs, session bar, badges |
| Settings → Downloads | Save-in folder, proxy, magnet handler, limits, Apply |
| Browser | `magnet:` links when the handler is on; `vortex://stream?magnet=` always |
| Background | Engine, completion watchers, stream trackers |

## Requirements

### Engine and privacy

- **FR-1.** The engine must start on first use when a download folder is set, resume the previous session's torrents, and restart with new settings on Apply.
- **FR-2.** With a SOCKS5 proxy set, every connection must go through it: DHT, incoming connections, uTP, local discovery and UDP trackers must be off, and .torrent files must be added with their HTTP trackers only. Without a proxy the page must say the user's IP is visible to peers.
- **FR-3.** Before the first unprotected stream or download, a dialog must explain the risk and ask once.
- **FR-4.** Download and upload limits in KB/s must apply.

### Adding and downloading

- **FR-5.** The user must be able to paste a magnet link or a bare info hash, or open a .torrent file, see the file list with sizes, choose files (videos ticked by default), choose Save in and whether to create a subfolder (named after the torrent by default), and start.
- **FR-6.** A download must be refused when the drive lacks free space, when the torrent is already in the list, or when no file is chosen.
- **FR-7.** Pieces must be written to a hidden staging folder the scanner and watcher ignore; when the torrent finishes, its files must move into the chosen folder, the staging folder must be cleaned, the torrent must leave the engine, and a library scan must run. The user must be told what moved.
- **FR-8.** Each row must show name, state, progress, speeds, peers, ETA and errors, and offer pause, resume and remove (keeping or deleting the files).
- **FR-9.** An expanded row must show Files (with per-file progress and Play for videos), Info, Peers, Trackers (UDP marked skipped behind a proxy) and a speed graph; the page must show a session bar with DHT state, totals and uptime.

### Streaming

- **FR-10.** Stream must resolve the link, take its largest video, buffer the start, and open the player at the position where the same stream last stopped. A link that already finished and moved must play the library file instead.
- **FR-11.** When the player closes, a stream that was never kept must pause and ask: Keep in library, Discard (delete what was fetched), or Decide later (stays paused with a Watch button).
- **FR-12.** Keep must turn it into a normal download that moves into the library when complete. Discard must remove it and its files.
- **FR-13.** Only video files may be played; the file served must be one that was selected.

### Magnet links

- **FR-14.** With "Open magnet links with Vortex" on, clicking a magnet link anywhere must bring Vortex up and stream it; switching it off must give the association back. `vortex://stream?magnet=…` must always work.

## Rules and edge cases

- Metadata resolution times out after 60 s with a proxy-aware message. Buffering needs min(2 % of the file, 8 MB) within 90 s.
- A stream is paused when its player closes only if it is still ephemeral and unfinished; its position resets when ≥ 90 % was reached. Only the newest stream of a torrent decides.
- Each torrent stages in `<save in>\.incomplete\<info hash>`; destination names that collide get " (2)", " (3)"…; a torrent wrapped in one folder is renamed, not nested.
- Links that arrive before the Downloads page listens are queued and delivered when it mounts.
- Polling is one tick at a time every 2 s; details are fetched only for expanded rows.

## Acceptance criteria

- **AC-1** (FR-1, FR-2). Set a `socks5://` proxy and Apply: the page says "Protected via proxy", the session bar says "DHT: Off (proxy mode)", a torrent's UDP trackers show "Skipped behind proxy".
- **AC-2** (FR-3, FR-5–FR-7). Without a proxy, Download… a magnet: the warning appears once; the picker ticks the videos; after it finishes a "Download finished" toast names the folder, the files are in `<save in>\<name>`, the row is gone and the film appears in the library.
- **AC-3** (FR-10, FR-11). Stream a magnet: the player opens; close it at 5:00: the Keep dialog shows "You stopped at 5m 0s"; Decide later leaves a paused row with Watch; Watch resumes at 5:00.
- **AC-4** (FR-12). Keep: the row loses its Stream badge and later moves into the library. Discard on another: the row and its files are gone.
- **AC-5** (FR-14). Switch the handler on, click a magnet link in a browser: Vortex comes to the front on Downloads and starts streaming.
- **AC-6** (FR-8). Pause and resume a download; it still moves into the library when it finishes.

## Principles

- Respects all principles. P2 and P6 are the whole shape of this feature; P7 for streams.

## Open questions

All answered on 2026-09-20: privacy warn-not-block; SOCKS5 as the kill switch (no adapter binding on Windows); finished files land in the library folder and seeding stops; external player only, with position tracking during streams added later; never a search.

## Changelog

- 2026-09-20: Built (`7b74654`), no spec. Decisions D-004 to D-009.
- 2026-09-23/25 (1.1.0–1.1.6): staging folder hidden and removed when empty; remove button fixed; polling of removed torrents stopped; Browse for the download folder; import button renamed.
- 2026-10-08 (`dd4006d`, `8b647f9`): per-torrent staging folders; paused-and-resumed downloads move into the library; Keep during a stream is not undone when the player closes; a second stream of the same torrent is not paused by an older player closing.
- 2026-10-08: Spec written from the code. Done.
