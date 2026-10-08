# 011. Rename files to TMDb names: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

`rename.rs` builds a `Plan` per title from the cached TMDb details (`TmdbInfo`: title, year, season sizes) and the title's episodes: `movie_moves` / `series_moves` produce ordered `Move`s (files, then season folders, then the show folder), `reads_back` re-parses the final path with `parser::parse` to make sure the next scan groups it the same way. `apply` runs under `jobs::exclusive`, re-plans, `rename_on_disk` (rollback on failure), `update_library` (paths, subtitle paths, `resync_title` + `rekey_memory`), appends the batch to `rename-history.json`.

## Data

| Table / key / file | Change |
| --- | --- |
| `episodes` | `path`, `file_name`, `subtitles` updated in place |
| `media_items` | `title`, `sort_key` re-synced to the new file name |
| `tmdb_memory` | re-keyed to the new key |
| `rename-history.json` | `{batches: [[Move]]}`, 20 kept |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `rename.rs` | `safe_name`, `base_name`, `stems_for`, `told_apart`, `sidecars`, `named_after`, `tmdb_info`, `plan_item`/`plan_into`, `movie_moves`, `series_moves`, `place`, `own_folder_move`, `named_after_title`, `folder_is_own`, `reads_back`, `final_path`, `preview`, `apply`, `undo`, `can_undo`, `carry_out`, `rename_on_disk`, `update_library`, `update_paths`, `resync_title`, history load/save. |
| `tmdb.rs` | `season_names`. |
| `db.rs` | `rekey_memory`. |
| `jobs.rs` | `exclusive`. |
| `commands.rs` | `rename_preview`, `rename_apply`, `rename_undo`, `rename_can_undo`. |

## Events and commands

- No events (the UI reloads itself after apply/undo). UI: `api.renamePreview`, `api.renameApply`, `api.renameUndo`, `api.renameCanUndo`.

## UI (`src`)

- `RenameDialog.vue` (preview, checkboxes, Show all, skipped list, Undo toast action); `SeriesView.vue` button; `SettingsView.vue` row.

## Cross-feature effects

- The watcher sees the renames and runs a `watch` scan afterwards, which must find nothing changed. The memory (012) follows the new key.

## Risks

- The watcher's scan after a large batch, plus the title page reloading on a possibly removed id, is the "blank page / keeps fetching" report (spec 014).

## Design doc updates

- `docs/FEATURES.md` §8.
