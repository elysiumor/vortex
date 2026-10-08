# 016. Remakes as separate titles: plan

Spec: [spec.md](spec.md) · Status: Draft (written before approval so the scope of the change is visible; to be revised with the answers to the open questions)

## Approach

Extend the grouping key rather than the uniqueness rule: `sort_key` becomes `<name key>` for files without a year and `<name key>:<year>` for files with one, with a lookup step in the scanner that joins a no-year file to an existing same-name title (choosing by TMDb year, else earliest). The `UNIQUE(kind, sort_key)` constraint stays. A one-time upgrade in `db::open` walks titles whose files carry more than one year (parsed from `file_name` / parent folder) and splits them with `move_progress`-style copies.

Considered and set aside: making `year` part of the unique key directly (a no-year file could never join a title); grouping by TMDb id (unmatched titles have none).

## Data

| Table / key / file | Change |
| --- | --- |
| `media_items.sort_key` | now carries `:<year>` when known; a `schema_version` setting records the upgrade |
| `tmdb_memory.sort_key` | upgraded in step with the titles |
| settings | `schema_version` (new) |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `parser.rs` | `group_key(title, year) -> String`. |
| `scanner.rs` | `write` closure: for movies, look up same-name titles before `upsert_media_item`; log the choice for no-year files. |
| `db.rs` | `split_merged_titles(conn)` upgrade (titles with files of differing years → new rows, progress/history/episode columns moved, tags and collections copied); `find_duplicates` unchanged (grouping by title id already separates them after the split). |
| `rename.rs` | keep `movie_moves` year partition as a safety net; `resync_title` uses `group_key`. |
| `tmdb.rs` | `apply_remembered` and `remember_*` key by `group_key`; the ±1 rule stays. |

## Events and commands

- None new. The upgrade logs "N titles split into M".

## UI (`src`)

- None required. Movies page shows two cards.

## Cross-feature effects

- Smart lists, collections, tags: copied to both halves on split. Backups made before the upgrade are upgraded when restored (the upgrade runs in `db::open`).

## Risks

- A wrong split moves history to the wrong half: the upgrade must decide per file by its own year and log every move; tested on a copy of the product owner's database before release.
- Series grouping (open question 1) changes the extras attachment rules if applied.

## Design doc updates

- `docs/FEATURES.md` §4 grouping rule, §15, §8, §7, §27 data model, Known gaps.
