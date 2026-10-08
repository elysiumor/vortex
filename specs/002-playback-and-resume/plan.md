# 002. Playback with exact resume: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`player::play` spawns the player with per-kind arguments and a `Link` describing how to read it (VLC HTTP, mpv pipe, PotPlayer window, none), then a thread polls every 2 s until `still_tracking` says the session is over, and finally writes progress + history and decides the next episode. A global `SESSION` counter lets a newer playback end older trackers that share a window. `LAST_VLC` remembers the last VLC window that answered, for the one-instance hand-off.

Set aside: Windows message hooks for MPC-HC (it has a web interface but off by default), launching players through their own playlists.

## Data

| Table / key / file | Change |
| --- | --- |
| `watch_progress` | `episode_id PK, position_secs, completed, last_watched, hidden` |
| `history` | `id, episode_id, at, position_secs, completed, exact, source` |
| settings | `player_kind`, `player_path`, `autoplay_next` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `player.rs` | `detect`, `build_launch`, `Link`, `Reading`, `read_vlc` / `read_mpv` / `pot::read`, `still_tracking`, `adopt_running_vlc`, `remember_vlc`, `same_file`, `play` (tracker thread, end logic, autoplay), `play_url` / `play_url_tracked` (streams, spec 009), `label_for`. |
| `db.rs` | `set_progress`, `touch_progress`, `add_history`, `next_episode`, `get_episode`, `set_duration`. |
| `commands.rs` | `play_episode` (blocking thread), `detect_players`. |
| `lib.rs` | First-run player selection. |

## Events and commands

- `playback-ended {episode_id, position_secs, completed, exact, next_episode_id, next_label, auto_started}`. UI: `api.playEpisode` (resolves false when untracked), `api.detectPlayers`, `api.onPlaybackEnded`.

## UI (`src`)

- `App.vue`: toasts by outcome, Up next card with countdown, `refreshAll`.
- `SettingsView.vue` Video player card. Every view's Play buttons call `api.playEpisode` and warn when it returns false.

## Cross-feature effects

- Continue watching, Home hero and the tray follow `watch_progress`. History (006) and Statistics read `history`. Streams (009) reuse the tracker.

## Risks

- Players that neither answer nor exit cleanly leave a tracker polling until their process ends; bounded by process-name checks.
- A VLC opened by the user (not Vortex) cannot be talked to: hand-offs to it fall back to the clock estimate.

## Design doc updates

- `docs/FEATURES.md` §9, §10.
