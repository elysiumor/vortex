# 001. Libraries and scanning: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

Two layers: a pure parser (`parser.rs`) that turns a path into kind/title/year/season/episode/extra using only the file name and the folders between it and the library root, and a scanner (`scanner.rs`) that walks each library, upserts rows, and reconciles what it saw against what the database had. Grouping is a unique key per kind, so every file that parses to the same title lands on one row without any lookup logic.

Scans are serialised by `jobs::Job` (scan, poster, probe each have one). A pass walks the disk first with no transaction open, then writes under an `IMMEDIATE` transaction, so the player's 15-second progress writes never make the scan fail.

Set aside: hashing files to detect renames (too slow on external drives; size + mtime is enough), parsing `.nfo` files, a per-library scan button.

## Data

| Table / key / file | Change |
| --- | --- |
| `libraries` | `id, path UNIQUE, name, added_at` |
| `media_items` | `id, kind, title, year, sort_key, category; UNIQUE(kind, sort_key)`; upsert keeps the first non-null year/category |
| `episodes` | `id, media_item_id, library_id, path UNIQUE, file_name, season, episode, size, modified, added_at, extra, subtitles` (+ later columns from specs 004 and 007) |
| settings | `rescan_on_startup`, `watch_folders`, `notify_new`, `ignore_dirs` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `parser.rs` | `parse(path, root) → Parsed`; `episode_code`, `year_in`, `folder_title_of`, `season_from_dir`, `is_season_dir`, `is_extras_dir`, `in_extras_dir`, `clean_title`, `sort_key`. Regex tables `SEASON_EP`, `X_EP`, `LONG_EP`, `EP_ONLY`, `DASH_EP`, `SEASON_WORD`, `SEASON_CODE`, `YEAR`, `JUNK`, `TECH`. |
| `scanner.rs` | `scan_all(conn)`, `scan_library` (two passes, fingerprints, rename matching, unreadable protection), `category_for`, extras attachment (`series_under`, `sole_series_under`, `movie_in`, `extra_label`), `DEFAULT_IGNORED_DIRS`, `ignored_dirs(conn)`, `ScanStats`. |
| `db.rs` | `add_library`, `add_library_from_ui` (overlap rules), `remove_library`, `prune_empty_items`, `upsert_media_item`, `upsert_episode`, `episode_fingerprints_for_library`, `move_progress`, `delete_episodes`, `fill_availability`. |
| `jobs.rs` | `Job` (begin / own / another_pass / release_on_panic), `refresh`, `refresh_async`, `scan_now`, `exclusive`, `run_scans`, `scan_once` (event, toast, follow-ups, tray). |
| `watcher.rs` | Debounced `notify` watcher over library roots; availability loop; `is_relevant`. |
| `commands.rs` | `list_libraries` (availability off the main thread), `add_library`, `remove_library`, `scan_libraries`, `list_drives`, `default_ignored_dirs`. |
| `lib.rs` | Startup scan or probe-only; `watcher::start`. |

## Events and commands

- `scan-done {reason, stats}`, `scan-error`. UI: `api.onScanDone`, `api.scanLibraries`, `api.listLibraries`, `api.addLibrary`, `api.removeLibrary`, `api.listDrives`, `api.defaultIgnoredDirs`.

## UI (`src`)

- `SettingsView.vue` Libraries card: list with dots, Add folder (dialog plugin), Add whole drive dialog (usage bars, system-drive alert), Remove, Rescan all with spinner and summary, Folders to skip textarea.
- `App.vue`: `onScanDone` toast wording by reason; `refreshAll`.

## Cross-feature effects

- Every scan that changed something starts the duration probe (007) and, when TMDb is connected, the poster fetch (004) and the memory re-apply (012).
- A finished download triggers a `torrent` scan (009). The tray is rebuilt after each scan (008).
- Renames (011) run under `jobs::exclusive` so a scan never sees half a rename.

## Risks

- Grouping by name merges remakes (spec 016).
- A folder watcher over a whole drive sees constant churn; the ignore list and `.incomplete` keep it quiet, and a burst is one scan.
- A library on a network share that is slow rather than offline makes the scan long; nothing else is blocked meanwhile (own connection).

## Design doc updates

- `docs/FEATURES.md` §2–§5 describe this as built.
