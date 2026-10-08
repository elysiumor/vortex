# 007. File durations: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`probe.rs`: a `probe_job` pass reads `episodes_missing_duration`, tries the native parsers (`matroska`, `mp4` crates) by extension, then ffprobe; writes `duration_secs` or `duration_checked`. Detection runs once per process (`OnceLock`). Children spawn with `CREATE_NO_WINDOW`.

## Data

| Table / key / file | Change |
| --- | --- |
| `episodes` | `duration_secs`, `duration_checked` |
| settings | `ffprobe_path` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `probe.rs` | `detect_ffprobe`, `detect_ffprobe_cached`, `hidden()`, `duration_of`, `probe_missing`, `probe_pass`. |
| `db.rs` | `episodes_missing_duration`, `set_duration`, `mark_duration_checked`. |
| `commands.rs` | `probe_durations`, `detect_ffprobe` (spawn_blocking). |
| `jobs.rs` | `scan_once` starts the probe. |

## Events and commands

- `durations-progress`, `durations-done {done,total,found}`. UI: `api.probeDurations`, `api.detectFfprobe`.

## UI (`src`)

- `SettingsView.vue` File durations card; `App.vue` refreshes when `found > 0`.

## Cross-feature effects

- Playback (002) stores a player-reported length; Statistics (006) and progress bars read it.

## Risks

- `duration_checked` never cleared (roadmap item).

## Design doc updates

- `docs/FEATURES.md` §6.
