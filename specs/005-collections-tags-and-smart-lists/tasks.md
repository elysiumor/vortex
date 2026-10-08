# 005. Collections, tags and smart lists: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`).

## Build

- [x] 1. Schema: tags, item_tags (FR-1, FR-2, FR-5)
- [x] 2. TMDb collection attachment and backfill, ordered by year (FR-1)
- [x] 3. Collections page: cards, create, rename, delete, reorder, remove, Play next (FR-2–FR-4)
- [x] 4. Tags on the title page and the Tag filter; unused tags pruned (FR-5)
- [x] 5. Smart lists: save dialog, setting, More menu, chips, delete (FR-6, FR-7)

## Verify

- [x] `cargo test -j 2` (no dedicated test; `list_media` tag/collection columns exercised by other tests)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, installed builds (TMDb collections appear in release order) | Pass |
| AC-2 | Product owner, installed builds | Pass |
| AC-3 | Product owner, installed builds | Pass |
| AC-4 | Product owner, installed builds | Pass |

## Close

- [x] `docs/FEATURES.md` §14, §11
- [x] README feature list
- [x] Roadmap: Done
- [x] Decision log: nothing new
- [x] Spec status Done
- [x] Spec index updated
