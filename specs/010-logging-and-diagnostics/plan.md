# 010. Logging and diagnostics: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`tracing-subscriber` with a `RollingFile` writer and a `LocalTime` formatter; a panic hook that logs with `Backtrace::force_capture`; release profile `panic = "unwind"` with line tables so backtraces name lines. In the renderer, `diag.ts` wraps every `invoke` in a timer and installs error, long-task and frame-rate observers that post to the `log_frontend` command. Blocking work moved to `tauri::async_runtime::spawn_blocking` (`blocking()` and `read_then_check()` helpers in `commands.rs`), and availability filled after the DB lock is released.

## Data

No tables. Files: `vortex.log`, `vortex.log.1`, `vortex.log.0`.

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `logging.rs` | `init`, `RollingFile`, `LocalTime`, `startup_summary`, `closing`, `DEFAULT_FILTER`. |
| `lib.rs` | `log_panics`; `closing` on window close / quit. |
| `commands.rs` | `blocking`, `read_then_check`, `log_frontend`, `reveal_log`. |
| `db.rs` | `fill_available`, `fill_availability` (called outside the lock). |
| `Cargo.toml` | release profile: thin LTO, `panic = "unwind"`, `debug = "line-tables-only"`, `strip = false`. |

## Events and commands

- `api.revealLog`; `log_frontend(level, message)` from `diag.ts`.

## UI (`src`)

- `lib/diag.ts` (`timedInvoke`, `installDiagnostics`), `main.ts`; `SettingsView.vue` Show log.

## Cross-feature effects

- Every command goes through `timedInvoke`; librqbit's tracing shares the subscriber.

## Risks

- Older WebView2 builds lack the `longtask` entry type (caught).

## Design doc updates

- `docs/FEATURES.md` §26, §28.
