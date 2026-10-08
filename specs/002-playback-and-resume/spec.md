# 002. Playback with exact resume

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; VLC single instance 2026-10-08, `a7367fd`) — written from the code on 2026-10-08 |
| **Area** | Playback · Settings |
| **Roadmap** | Done: Playback with exact resume |
| **Design doc** | `docs/FEATURES.md` §9 Playback and tracking, §10 Watch progress |

## Problem

The user watches in PotPlayer or VLC and stops half way. Next time nothing remembers where: the player's own history is per file and easily lost, and across a library of hundreds of films and series nobody can tell what is half watched, what is finished and what comes next. Marking things watched by hand does not scale, and a second-based guess from how long the player was open is wrong whenever the user paused.

Vortex should launch the user's own player and know, to the second, where they stopped, mark things watched when they really finished, and offer the next episode.

## Goals

- Playing from Vortex starts the user's player at the saved position, with the right subtitle file.
- While playing, the real position is read from the player and saved often enough that a crash loses little.
- An episode counts as watched only when it really reached the end; a manual stop keeps the position.
- The next episode starts by itself, or is offered with a countdown.
- Works with the four players people have on Windows, and degrades honestly (estimated) for others.

## Non-goals

- A built-in player (D-002).
- Reading positions from players Vortex did not launch (except a VLC window that took a hand-off from one it did).
- Remote control of the player beyond reading position and state.

## Where it lives

| Place | What |
| --- | --- |
| Every Play button (Home, library cards, title page, history, search, duplicates, tray) | Launches the player and starts a tracker |
| Settings → Video player | Detected players, path, type |
| Background | One tracker thread per playback; the "Up next" card; `playback-ended` event |

## Requirements

- **FR-1.** Vortex must detect PotPlayer, VLC, MPC-HC and mpv at their usual install paths (and scoop shims) and select the first one found on first run. The user must be able to pick a detected player, browse to any executable, and set its type.
- **FR-2.** Play must start the file in the configured player from the saved position, except that a completed episode starts from the beginning. Sidecar subtitles must be passed to players that accept them, preferring an English one.
- **FR-3.** With no player configured, Play must open the file with Windows' default app and say that progress will not be tracked.
- **FR-4.** While the player runs, the position must be read every two seconds from PotPlayer, VLC and mpv and saved every 15 seconds. For MPC-HC and other players the position is estimated from the time elapsed.
- **FR-5.** When the player closes, the final position must be saved and a history entry written, marked exact or estimated. An episode must count as watched when the position reached 90 % of its duration, or when the player itself reported the file finished after reaching 90 %.
- **FR-6.** A session shorter than 15 seconds with no exact reading must leave no trace.
- **FR-7.** When an episode of a series is completed and the next one exists, with auto-play on: if the player is still open the next episode starts in it at once; otherwise a 10-second "Up next" card offers Play now or cancel.
- **FR-8.** Starting another playback from Vortex must end a tracker that shares a player window (PotPlayer, a VLC in one-instance mode, estimated players), so a position is never saved against the wrong episode.
- **FR-9.** With VLC's "Allow only one instance" on, a file handed to the already-open window must still be tracked exactly: Vortex follows that window once it confirms the file is playing there, and stops when the window moves on to another file.
- **FR-10.** The duration learned from the player must be stored when the file had none.
- **FR-11.** A file on a drive that is not connected must not launch; the error must say so.

## Rules and edge cases

- PotPlayer: `/seek=hh:mm:ss`, position via window messages; "stopped" status counts as the end only if 90 % was reached. VLC: `--extraintf=http` on a random loopback port with a per-launch password; `--start-time`. mpv: `--input-ipc-server` named pipe; `--start`. MPC-HC: `/startpos`, `/sub`; no position link.
- Tracking ends when: a per-launch link we heard from loses its process; a shared link's process name disappears; a newer playback starts (shared links, or per-launch links never heard from); VLC reports a different file.
- Final position = the furthest position seen if the player reported the end, else the last exact reading, else start + elapsed; clamped to the duration.
- Playing anything touches its `last_watched` and un-hides it from Continue watching.
- Next episode = the next non-extra episode in season/episode order whose file exists; movies have none.

## Acceptance criteria

- **AC-1** (FR-1). On a PC with VLC installed and nothing configured, Settings shows VLC selected after first launch.
- **AC-2** (FR-2, FR-4, FR-5). Play an episode in VLC, stop at 10:00 and close VLC: the episode shows "paused at 10:00", a history row "Stopped at 10:00" (exact), and Play now says "Resume at 10:00".
- **AC-3** (FR-5, FR-7). Let an episode play to the end with VLC open: it is marked watched and the next episode starts in the same VLC; with mpv, the Up next card appears and starts the next one after 10 s.
- **AC-4** (FR-6). Open and close the player within 15 s: no history row, no position.
- **AC-5** (FR-9). VLC with "one instance": play episode 1, then episode 2 while 1 plays. Episode 1 keeps its own position; episode 2 is tracked in the same window.
- **AC-6** (FR-3). Clear the player path and press Play: the file opens with the default app and the toast says progress is not tracked.
- **AC-7** (FR-11). Play a file on an unplugged drive: "File is not available. Is the drive connected?".

## Principles

- Respects all principles. P7 exactly.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08 (`dd4006d`): launching the player moved off the main thread; `touch_progress` on play.
- 2026-10-08 (`8b647f9`): double-clicking Play starts the player once; a newer playback takes over a shared window.
- 2026-10-08 (`a7367fd`): VLC single-instance hand-off adopted and takeover detected (FR-9).
- 2026-10-08: Spec written from the code. Done.
