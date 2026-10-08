# 006. Duplicates, history and statistics: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

Three read-mostly pages over existing tables. Duplicates is a grouped query over episodes; History is a windowed query (latest row per episode) joined back to the episode; Statistics is a handful of aggregates in `db::stats`. File availability is filled after the lock is released (`read_then_check`), and the Recycle Bin call runs on a blocking thread.

## Data

No new tables. `history` (002) and `episodes`.

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `db.rs` | `find_duplicates`, `delete_episode` (+ prune), `list_history` (ROW_NUMBER per episode), `history_stats`, `delete_history`, `clear_history`, `stats`. |
| `commands.rs` | `find_duplicates`, `trash_episode` (`trash::delete` off the lock), `list_history`, `history_stats`, `delete_history`, `clear_history`, `stats`. |

## Events and commands

- No events. UI: `api.findDuplicates`, `api.trashEpisode`, `api.listHistory`, `api.historyStats`, `api.deleteHistory`, `api.clearHistory`, `api.stats`.

## UI (`src`)

- `DuplicatesView.vue` (confirm dialog holds its target outside the ref), `HistoryView.vue`, `StatsView.vue`; `App.vue` badge via `refreshDupCount`.

## Cross-feature effects

- Deleting a duplicate prunes an emptied title; `refreshAll` updates the badge.

## Risks

- Duplicates by title only: a remake pair shows as copies (spec 016).

## Design doc updates

- `docs/FEATURES.md` §15–§17.
