# 012. TMDb data kept across folder switches: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

Three tables keyed independently of `media_items`: `tmdb_memory` by `(kind, sort_key)` (what the scanner will key a returning title by), `tmdb_cache` and `tmdb_seasons` by TMDb id. `apply_remembered` runs in `scan_once` before the poster fetch. Images are named `{kind}-{tmdb_id}.jpg`. `db::open` backfills all three from the legacy per-title tables once.

## Data

| Table / key / file | Change |
| --- | --- |
| `tmdb_memory(kind, sort_key, tmdb_id, manual, poster, overview, rating, genres, year, updated_at)` | new |
| `tmdb_cache(kind, tmdb_id, json, fetched_at)` | new, replaces `tmdb_details` reads |
| `tmdb_seasons(tmdb_id, season, json, fetched_at)` | new |
| `posters\` | files renamed to TMDb-id names by `migrate_image_names` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `db.rs` | `remembered`, `remember_match`, `remember_not_found`, `rekey_memory`, `tmdb_store_counts`, details/season json getters and setters; backfills in `open`. |
| `tmdb.rs` | `apply_remembered`, `poster_path_for`, `backdrop_path_for`, `migrate_image_names`, `store_stats`, `retry_poster` reading the memory. |
| `jobs.rs` | `apply_remembered` after a changed scan; `library-changed` event. |
| `commands.rs` | `tmdb_store`. |

## Events and commands

- `library-changed`. UI: `api.tmdbStore`, `api.onLibraryChanged`.

## UI (`src`)

- `SettingsView.vue` store line; `App.vue` refreshes on `library-changed`.

## Cross-feature effects

- Rename (011) re-keys the memory; backup (008) includes the tables and images.

## Risks

- Keyed by name: a remake pair shares one memory row (year ±1 rule limits the damage; spec 016).

## Design doc updates

- `docs/FEATURES.md` §7 memory subsection, §27 data model.
