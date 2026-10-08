# 004. TMDb posters, details and episode titles: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

A blocking `reqwest` client in `tmdb.rs`, always called from a blocking thread. Matching is a background job (`fetch_job`) that walks the titles missing a poster; details and seasons are fetched on demand and cached as raw JSON per TMDb id, normalised at read time so the schema of what is shown can change without refetching. Images live on disk under the TMDb id; the WebView loads them through the asset protocol with a cache-busting query.

## Data

| Table / key / file | Change |
| --- | --- |
| `media_items` | `tmdb_id, poster_path, overview, rating, genres, poster_checked` |
| `episodes` | `title, overview, air_date, still_path, rating` |
| `tmdb_cache(kind, tmdb_id, json, fetched_at)` | details per TMDb id (replaces per-title `tmdb_details`) |
| `tmdb_seasons(tmdb_id, season, json, fetched_at)` | season episode lists |
| settings | `tmdb_key`, `tmdb_connected` |
| `posters\{kind}-{tmdb_id}.jpg`, `-backdrop.jpg` | image store |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `tmdb.rs` | `search`, `best_match`, `fetch_missing` / `fetch_pass`, `apply_match`, `retry_poster`, `get_details` (cache, 90-day age, offline fallback), `normalize`, `certification`, `backdrop_file`, `fetch_episode_titles` / `season_episodes` / `season_names`, `genre_names`, `details_to_item`, `apply_cached_details`, `posters_dir`, `download_poster`, `migrate_image_names`, `store_stats`. |
| `db.rs` | `items_missing_poster`, `mark_poster_checked`, `clear_poster_checked`, `set_poster`, `set_tmdb`, `set_item_rating`, `set_item_genres`, `set_episode_meta`, `seasons_for_item`, `get/set_details_json`, `get/set_season_json`, `attach_tmdb_collection`, `backfill_tmdb_collections`. |
| `commands.rs` | `connect_tmdb`, `disconnect_tmdb`, `get_tmdb_status`, `test_tmdb_key`, `fetch_posters`, `search_tmdb`, `apply_tmdb_match`, `get_details`, `fetch_episode_titles`, `tmdb_store`; `get_settings` masks the key. |

## Events and commands

- `posters-progress`, `posters-done {done,total,matched,current}`. UI: `api.fetchPosters`, `api.searchTmdb`, `api.applyTmdbMatch`, `api.getDetails`, `api.fetchEpisodeTitles`, `api.tmdbStatus`, `api.connectTmdb`, `api.disconnectTmdb`, `api.tmdbStore`.

## UI (`src`)

- `SettingsView.vue` TMDB card; `SeriesView.vue` details, Fix match dialog, Refresh; `posterSrc`/`backdropSrc` in `api.ts`.

## Cross-feature effects

- Genres and ratings feed the library filters/sorts and Home rows; collections (005) are created from details; the memory (012) is written on every match; the scan (001) starts the fetch.

## Risks

- First-result matching mis-matches short or common titles; Fix match is the remedy and is protected from being undone.
- One details call per matched title adds API traffic on first scan of a large library.

## Design doc updates

- `docs/FEATURES.md` §7.
