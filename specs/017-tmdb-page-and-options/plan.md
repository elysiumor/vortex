# 017. TMDb page and API options: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

A `tmdb::Prefs` struct held in a static `Mutex`, loaded from the `tmdb_*` settings at startup (`lib.rs`) and reloaded by `set_setting` whenever a `tmdb_*` key changes. Every call site reads `prefs()` for language, region, year parameter, certification country, image sizes (`image_url(size, path)`), delay and timeout. `search` gains `include_adult`; `best_match` gains `adult` and `year_fallback`; `fetch_pass` reads `tmdb_adult` and `tmdb_year_fallback`. `TmdbView.vue` is a new page; the Settings card links to it.

## Data

| Table / key / file | Change |
| --- | --- |
| settings | `tmdb_auto_match`, `tmdb_adult`, `tmdb_year_fallback`, `tmdb_language`, `tmdb_region`, `tmdb_year_mode`, `tmdb_cert_country`, `tmdb_poster_size`, `tmdb_backdrop_size`, `tmdb_still_size`, `tmdb_profile_size`, `tmdb_delay_ms`, `tmdb_timeout_secs`, `tmdb_cache_days` |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `tmdb.rs` | `Prefs`, `prefs()`, `load_prefs`, `lang()`, `image_url`; `search(.., include_adult)`; `best_match(.., adult, year_fallback)`; `get_details` reads `tmdb_cache_days`; `certification` uses `cert_country`; delays from prefs. |
| `jobs.rs` | `fetch_missing` gated by `tmdb_auto_match`. |
| `commands.rs` | `search_tmdb` passes `true`; `set_setting` reloads prefs; `cancel_posters` (spec 014). |
| `lib.rs` | `load_prefs` at startup. |

## Events and commands

- `api.cancelPosters` (014). Existing settings commands.

## UI (`src`)

- `TmdbView.vue` (new): six cards, selects for language/region/country/sizes, numeric inputs, flags, fetch tools, unmatched list, rename row; `SettingsView.vue` link card; `App.vue` and `QuickSearch.vue` page entry.

## Cross-feature effects

- Poster fetch (004), details, episode titles and collections use the prefs; rename moved from Settings to the TMDb page (011).

## Risks

- Changing the poster size mid-library mixes sizes on disk (Known gaps).

## Design doc updates

- `docs/FEATURES.md` §7, §22 (done in `5ba2545`).
