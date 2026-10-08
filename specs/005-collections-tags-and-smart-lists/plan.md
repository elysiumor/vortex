# 005. Collections, tags and smart lists: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

Tags and collections share `tags` + `item_tags` (a kind column and a position for ordering). TMDb collections are attached from cached details (`belongs_to_collection`) at match time and backfilled at startup. Smart lists are UI-only: a JSON array in one setting, interpreted by `LibraryView` as extra filters.

## Data

| Table / key / file | Change |
| --- | --- |
| `tags` | `id, name, kind ('tag'/'collection'), tmdb_collection_id, poster_url, overview; UNIQUE(kind, name NOCASE)` |
| `item_tags` | `tag_id, media_item_id, position` |
| settings | `smart_lists` (JSON) |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `db.rs` | `list_tags` (counts, watched counts, first poster), `get_tag`, `ensure_tag`, `rename_tag`, `delete_tag`, `add_item_tag`, `remove_item_tag`, `set_item_tags` (prunes unused), `set_collection_order`, `collection_items`, `attach_tmdb_collection` (re-sorted by year), `backfill_tmdb_collections`. |
| `tmdb.rs` | `details_to_item` attaches the collection. |
| `commands.rs` | `list_tags`, `get_tag`, `create_tag`, `rename_tag`, `delete_tag`, `set_item_tags`, `add_to_collection`, `remove_from_collection`, `set_collection_order`, `collection_items`. |

## Events and commands

- No events. UI: the `api.*Tag*`/`*Collection*` calls; smart lists via `api.getSettings`/`api.setSetting("smart_lists")`.

## UI (`src`)

- `CollectionsView.vue`, `SeriesView.vue` (tags, collections), `LibraryView.vue` (tag filter, smart-list dialog and chips), `App.vue` (More menu, `addSmartList`, `removeSmartList`), `collectionPoster` in `api.ts`.

## Cross-feature effects

- Home's "Collections in progress" row; `list_media` carries tag and collection names for filters.

## Risks

- `maxYear` exists in the smart-list type with no input (roadmap).

## Design doc updates

- `docs/FEATURES.md` §14, §11.
