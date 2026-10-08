# 015. Confirm before removing a library folder: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`Library` gains a `file_count` filled by a subquery in `list_libraries_rows`, so the Settings list and the dialog have it without another command. The Remove button becomes `variant="outline"` and opens an `AlertDialog` whose target is held in a plain variable (closing-race pattern); confirming calls `remove_library` and emits `scanned`.

## Data

| Table / key / file | Change |
| --- | --- |
| none | `Library.file_count` is computed, not stored |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `db.rs` | `Library { file_count }`; `list_libraries_rows` subquery; `add_library` returns 0. |

## Events and commands

- No new commands; `api.listLibraries` carries `file_count`.

## UI (`src`)

- `SettingsView.vue`: file count span, outlined Remove, `removePending`/`removeTarget`, `askRemove`, `confirmRemove`, the dialog text.

## Cross-feature effects

- `emit("scanned")` refreshes views (unchanged).

## Risks

- None material.

## Design doc updates

- `docs/FEATURES.md` §2 (done in `5ba2545`).
