# 003. Home, library pages, title page and search: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

One page-level component per view, all mounted by `App.vue` which owns navigation (`page`, `openId`, `search`), exposes `reload()` on each view and refreshes them all through a debounced `refreshAll`. Data comes from a few wide queries (`list_media` with counts, `list_episodes`, `continue_watching`) so views filter and sort client-side; details come from the TMDb cache (004).

## Data

No tables of its own. Reads `media_items`, `episodes`, `watch_progress`, tags; writes progress, category, tags and collections through existing commands. localStorage `vortex-sort`.

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `db.rs` | `list_media` (aggregated counts, tags, collections), `get_media_item`, `list_episodes`, `continue_watching`, `search`, `list_categories`, `set_item_category`, `hide_from_home`, `set_item_watched`, `clear_progress`. |
| `commands.rs` | The browsing commands; `list_episodes`, `continue_watching`, `search` use `read_then_check` so availability is filled off the lock; `reveal_path` on a blocking thread. |

## Events and commands

- Consumes `scan-done`, `library-changed`, `posters-done`, `durations-done`, `playback-ended`, `library-restored` via `refreshAll`.

## UI (`src`)

- `App.vue` (shell, nav, search, keyboard, Up next, toasts), `HomeView.vue` (hero, rows), `LibraryView.vue` (grid, filters, sort, smart-list save), `SeriesView.vue` (title page, dialogs), `SearchView.vue`, `QuickSearch.vue`, `MediaCard.vue`, `MediaRow.vue`, `EmptyState.vue`, `composables/useGridKeys.ts`, `lib/format.ts`.

## Cross-feature effects

- Collections (005) and smart lists (005) render through `LibraryView`/`MediaCard`. The tray (008) mirrors Continue watching.

## Risks

- `list_media` is one aggregated query over every episode; fine at hundreds of titles, worth an index review at tens of thousands.
- The title page keeps its skeleton when the item id is gone (Known gaps; spec 014 addresses it).

## Design doc updates

- `docs/FEATURES.md` §10–§13, §23.
