# 003. Home, library pages, title page and search: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); redesigned 2026-10-08 (`8b647f9`, spec 013).

## Build

- [x] 1. Queries: list_media with counts, continue_watching rules, search, categories (FR-2, FR-5, FR-14)
- [x] 2. App shell: navigation, More menu, search box, keyboard, refreshAll (FR-14, FR-16)
- [x] 3. HomeView: hero rules, rows, empty welcome, dismiss (FR-1–FR-4)
- [x] 4. LibraryView: filters, sorts, card subtitles, Play on card (FR-5–FR-7)
- [x] 5. SeriesView: hero, actions, people, tags, collections, cast, seasons, episode rows, details panel, extras (FR-8–FR-13)
- [x] 6. SearchView and QuickSearch (FR-14, FR-15)
- [x] 7. MediaCard, MediaRow, useGridKeys (FR-6, FR-16)

## Verify

- [x] `cargo test -j 2`: `db::tests::continue_watching_survives_a_series_marked_watched`, `scanner` continue-watching assertions
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, installed 1.1.9/1.1.10; hero and rows reviewed in the redesign round | Pass |
| AC-2 | Product owner, installed builds | Pass |
| AC-3 | Product owner, installed builds | Pass |
| AC-4 | Code path; not separately exercised | Not re-verified |
| AC-5 | Product owner, installed builds | Pass |
| AC-6 | Redesign round 2026-10-08 (arrow keys in rows fixed then) | Pass |
| AC-7 | First-run screen, 1.1.9 | Pass |

## Close

- [x] `docs/FEATURES.md` §10–§13, §23
- [x] README keyboard section
- [x] Roadmap: Done
- [x] Decision log: nothing new
- [x] Spec status Done
- [x] Spec index updated
