# 009. Downloads and streaming: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`torrent.rs` wraps a librqbit `Session` on its own tokio runtime (`Engine`), with a read-only librqbit HTTP API on a loopback port so players can open `…/torrents/<id>/stream/<file>/<name>` with range requests. Per-torrent destinations live in `dests.json` beside librqbit's session persistence. A completion watcher per torrent polls stats, forgets the torrent (keeping files) and moves entries into place. Streams are ephemeral torrents with a head-buffer wait and the same player tracker as library playback (002), with a callback that decides pause/keep/discard.

Set aside: adapter binding (Linux/macOS only in librqbit), a built-in player, seeding after completion.

## Data

| Table / key / file | Change |
| --- | --- |
| settings | `torrent_dir`, `torrent_proxy`, `torrent_down_kbps`, `torrent_up_kbps`, `torrent_unprotected_ack`, `magnet_handler` |
| `torrents\` | librqbit session JSON, `dht.json`, `dests.json` (`Dest {save_in, subfolder, ephemeral, position_secs, duration_secs, completed_path}`) |
| `<save in>\.incomplete\<hash>\` | staging, hidden |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `torrent.rs` | `Engine::start/stop`, `session_options` (proxy mode), `to_add` (UDP stripping, .torrent → HTTP-only magnet), `inspect`, `add` (free space, staging), `list`/`row`, `pause`/`resume`/`remove`, `detail`, `session_status`, `stream`/`stream_existing`/`stream_file`, `keep`/`discard`, `stream_url`, `watch` (completion), `move_finished`, `unique_dest`, `normalize_source`, `source_hash`, `ensure`/`restart`/`status`. |
| `player.rs` | `play_url`, `play_url_tracked` with `StreamEnd` callback. |
| `deeplink.rs` | `magnet_from`, `setup` (register `vortex`, `magnet` when on), `deliver`, `take_pending`, `set_magnet_handler`. |
| `lib.rs` | single-instance + deep-link plugins; saved session resumed at start. |
| `commands.rs` | the `torrent_*` commands, `set_magnet_handler`, `pending_open_urls`. |

## Events and commands

- `torrent-done {name, moved}`, `stream-ended {id, name, position_secs, duration_secs, finished, ephemeral, exact}`, `open-url`. UI: `api.torrent*`, `api.setMagnetHandler`, `api.pendingOpenUrls`, `api.onOpenUrl`, `api.onTorrentDone`, `api.onStreamEnded`.

## UI (`src`)

- `DownloadsView.vue` (add flow, picker dialog, stream flow, rows, tabs, graphs, session bar, dialogs); `SettingsView.vue` Downloads card; `App.vue` routes `open-url` to Downloads.

## Cross-feature effects

- Finished downloads trigger a `torrent` scan (001) and the arrival notification (008). The watcher ignores `.incomplete`.

## Risks

- Torrents with only UDP trackers find no peers in proxy mode (by design).
- The engine restart on Apply pauses about a second.

## Design doc updates

- `docs/FEATURES.md` §18, §19.
