# 004. TMDb posters, details and episode titles: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); matching fixes 2026-10-08.

## Build

- [x] 1. Schema: tmdb columns on titles and episodes, tmdb_cache, tmdb_seasons, settings keys (FR-2, FR-6, FR-7)
- [x] 2. Key handling: verify, store, mask, never to the UI (FR-1)
- [x] 3. Poster fetch job: candidates, best match, apply, not-found marking, retry rules, pacing, abort on auth failure, events (FR-2–FR-4, FR-8, FR-9)
- [x] 4. Details: fetch with appended credits/videos/ids/ratings, normalise, cache 90 days, offline fallback, backdrop file, Refresh (FR-6)
- [x] 5. Episode titles per season, cached, refetch rules (FR-7)
- [x] 6. Fix match dialog and manual flag (FR-5)
- [x] 7. Image store by TMDb id and migration of old names (FR-10)
- [x] 8. Settings card: progress, outcome toasts, store size (FR-8)

## Verify

- [x] `cargo test -j 2`: `db::tests::tmdb_matches_are_remembered_across_folder_switches` (manual match wins, not-found remembered)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, installed builds (key connected; key removal keeps posters) | Pass |
| AC-2 | Product owner, installed builds since 1.0.0 | Pass |
| AC-3 | Product owner, 1.1.8 round ("Retry unmatched") | Pass |
| AC-4 | 1.1.9 review round: a hand-picked match no longer replaced by a failed download or the next fetch | Pass |
| AC-5 | Product owner, installed builds | Pass |
| AC-6 | Code path (stale cache served without a key); not separately exercised | Not re-verified |
| AC-7 | Code path (`Err` branch in `fetch_pass`) | Not re-verified |

## Close

- [x] `docs/FEATURES.md` §7
- [x] README (TMDb attribution)
- [x] Roadmap: Done
- [x] Decision log: D-012
- [x] Spec status Done
- [x] Spec index updated
