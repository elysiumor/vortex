# 010. Crashes survivable, logging and diagnostics, nothing on the main thread

| | |
| --- | --- |
| **Status** | Done (2026-09-23, `66e32fa`, `ea0bc53`, `886839e`; log improvements 2026-10-08, `dd4006d`) — written from the code on 2026-10-08 |
| **Area** | Shell |
| **Roadmap** | Done: Crashes survivable, logging and renderer diagnostics |
| **Design doc** | `docs/FEATURES.md` §26 Logging and diagnostics, §28 Build |

## Problem

Releases 1.1.0 and 1.1.1 crashed or froze on the product owner's PC with nothing to go on: no log, no backtrace, and a window that stopped painting whenever an external drive was asleep. Bug reports arrived as "it froze" with no numbers.

## Goals

- Every run writes a log that a bug report can be read from after the fact, with local timestamps and a startup summary.
- A panic in a background task does not take the app down and is recorded with a backtrace.
- The renderer reports its own trouble: slow commands, long tasks, frame stalls, uncaught errors.
- Nothing that can block runs on the main thread or under the database lock.

## Non-goals

- Sending logs anywhere; a crash reporter UI.

## Where it lives

| Place | What |
| --- | --- |
| `%APPDATA%\com.admin.vortex\vortex.log` (+ `.1`, `.0`) | The log |
| Settings → Appearance & behaviour → Show log | Reveals the file |
| Everywhere | Timed IPC calls, diagnostics |

## Requirements

- **FR-1.** The app must log to a file from start to exit, keep the previous run's log, and roll a run that grows past 20 MB.
- **FR-2.** The log must start with the version, the settings that change behaviour (secrets only as present/absent) and every library with its availability, and must note a clean exit with its reason.
- **FR-3.** A panic must be logged with its thread and a backtrace, and a panic in a background task must not end the process.
- **FR-4.** The UI must report into the log: commands slower than 250 ms, failed commands, tasks that block the renderer for 200 ms or more, five-second windows under 30 fps with a frame gap over 100 ms, uncaught errors and rejections, and its size and DPR when ready.
- **FR-5.** File existence checks, process launches, drive enumeration, network calls and long scans must run off the main thread and outside the database lock, so a sleeping drive never freezes the window.
- **FR-6.** The user must be able to open the log's folder from Settings.
- **FR-7.** The log level must be adjustable with the `VORTEX_LOG` environment variable; the torrent engine's own tracing must land in the same file.

## Rules and edge cases

- Default filter: info overall, debug for the UI and Vortex's own modules, info for librqbit, warn for its DHT.
- Timestamps are local wall-clock.
- Messages raised before the IPC bridge is up are queued and retried.

## Acceptance criteria

- **AC-1** (FR-1, FR-2). After a launch, `vortex.log` begins with "Vortex starting version=…", a "settings" line with `tmdb_key=true/false` (never the key) and one line per library; the previous run is in `vortex.log.1`.
- **AC-2** (FR-4). Open a title whose details take long to load: a "slow command get_details took …ms" line appears in the log.
- **AC-3** (FR-5). With an external drive asleep, open History and Continue watching: the window keeps repainting while the rows wait for availability.
- **AC-4** (FR-6). Settings → Show log opens Explorer at the file.

## Principles

- Respects all principles. P4, P8, P9.

## Open questions

None open.

## Changelog

- 2026-09-23: Built across `66e32fa` (panics logged, unwind, line tables), `ea0bc53` (renderer diagnostics), `886839e` (blocking work off the main thread). Decision D-010, D-015.
- 2026-10-08 (`dd4006d`): local timestamps, startup summary, scan reasons and job outcomes logged, log rotation per run.
- 2026-10-08: Spec written from the code. Done.
- 2026-10-08 (`5ba2545`, 1.1.11): a Content Security Policy is set (scripts and styles from the app, images from the app, the asset protocol and `image.tmdb.org`, connections to the IPC bridge only, plus the Vite server in dev); violations are forwarded to the log as errors (D-017).
