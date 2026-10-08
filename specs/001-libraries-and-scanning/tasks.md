# 001. Libraries and scanning: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`), hardened 2026-10-08 (`dd4006d`).

## Build

- [x] 1. Schema: libraries, media_items (unique kind + sort_key), episodes; settings keys (FR-1, FR-9, FR-19)
- [x] 2. `parser.rs`: patterns, year rules, title cleaning, extras detection, with unit tests (FR-8, FR-10)
- [x] 3. `scanner.rs`: two-pass scan, grouping, category, extras attachment, rename matching, unreadable and offline protection, stats (FR-7, FR-9–FR-14)
- [x] 4. `jobs.rs`: one job per kind, "again" pass, exclusive, follow-ups and toast after a scan (FR-15, FR-16)
- [x] 5. `watcher.rs`: debounced watcher, availability loop, drive return (FR-17, FR-18)
- [x] 6. Settings Libraries card: add folder/drive, overlap rules, remove, rescan, skip list, system-drive confirm (FR-1–FR-6, FR-19)
- [x] 7. Startup scan flag; tray Rescan (FR-19)

## Verify

- [x] `cargo test -j 2`: `parser::tests` (14 cases incl. `titles_that_look_like_release_tags_survive`, `release_year_not_the_year_in_the_title`, `generic_extras_names_are_categories_at_the_top`), `scanner::tests::scan_groups_series_and_movies_and_tracks_progress`, `scanner::tests::extras_attach_to_their_series_and_movies`, `db::tests::nested_libraries_are_refused_or_folded_in`
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | `scan_groups_series_and_movies_and_tracks_progress` builds exactly this tree and asserts the counts, titles and categories | Pass |
| AC-2 | `extras_attach_to_their_series_and_movies` (Euphoria tree, label "Season 1 › Featurettes › Behind The Scenes", 2 episodes) | Pass |
| AC-3 | Same test: file renamed on disk, rescan → (0 added, 0 removed, 1 renamed), position 600 kept | Pass |
| AC-4 | Same test: file deleted, rescan → removed 1, episode count 2 | Pass |
| AC-5 | `nested_libraries_are_refused_or_folded_in` | Pass |
| AC-6 | Product owner's installed build on 2026-10-08 (1.1.8 review round: a drive unplugged mid-scan no longer deleted episodes) | Pass |
| AC-7 | Product owner's installed build (watcher arrivals with the Windows toast, since 1.0.0) | Pass |
| AC-8 | Code path (`scan_now` → `Job::own`); not separately exercised | Not re-verified |

## Close

- [x] `docs/FEATURES.md` §2–§5
- [x] README feature list
- [x] Roadmap: Done
- [x] Decision log: D-003
- [x] Spec status Done
- [x] Spec index updated
