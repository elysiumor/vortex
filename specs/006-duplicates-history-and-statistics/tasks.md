# 006. Duplicates, history and statistics: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); fixes 2026-09-23 and 2026-10-08.

## Build

- [x] 1. Duplicate grouping query, recoverable size, Recycle Bin delete, More badge (FR-1–FR-5)
- [x] 2. History list (one row per file), tiles, remove, clear (FR-6–FR-8)
- [x] 3. Statistics aggregates and charts (FR-9, FR-10)

## Verify

- [x] `cargo test -j 2`: `db::tests::history_lists_each_file_once_at_its_latest_position`, `db::tests::stats_runs_with_and_without_watch_history`, `scanner::tests::extras_attach_to_their_series_and_movies` (no duplicate group for a featurette)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, 1.1.8 (delete button fixed and confirmed working) | Pass |
| AC-2 | `history_lists_each_file_once_at_its_latest_position` | Pass |
| AC-3 | Code path; not separately exercised | Not re-verified |
| AC-4 | `stats_runs_with_and_without_watch_history` (empty library and a 200-day-old session) | Pass |

## Close

- [x] `docs/FEATURES.md` §15–§17
- [x] README feature list
- [x] Roadmap: Done
- [x] Decision log: nothing new
- [x] Spec status Done
- [x] Spec index updated
