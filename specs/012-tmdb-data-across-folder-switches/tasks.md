# 012. TMDb data kept across folder switches: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-10-08 (`8b647f9`).

## Build

- [x] 1. Tables and backfills in `db::open` (FR-1, FR-2)
- [x] 2. Memory writes on match / not found; manual rule (FR-1, FR-5)
- [x] 3. Images by TMDb id and one-time migration (FR-3)
- [x] 4. `apply_remembered` after scans with the year rule (FR-4)
- [x] 5. Rekey on rename (FR-6)
- [x] 6. Store stats in Settings (FR-7)

## Verify

- [x] `cargo test -j 2`: `db::tests::tmdb_matches_are_remembered_across_folder_switches`
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, 1.1.9 (folder switch reused posters) | Pass |
| AC-2 | `tmdb_matches_are_remembered_across_folder_switches` (hand-picked wins) | Pass |
| AC-3 | 1.1.9 review round (remembered match not applied to a different film of the same title) | Pass |
| AC-4 | Product owner, 1.1.9 | Pass |

## Close

- [x] `docs/FEATURES.md` §7, §27
- [x] README (data section)
- [x] Roadmap: Done
- [x] Decision log: D-012, D-013
- [x] Spec status Done
- [x] Spec index updated
