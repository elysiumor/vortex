# 002. Playback with exact resume: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); VLC one-instance 2026-10-08 (`a7367fd`).

## Build

- [x] 1. Schema: watch_progress, history; player settings (FR-4, FR-5)
- [x] 2. Detection and first-run selection (FR-1)
- [x] 3. Launch arguments per player, subtitle choice, start position, default-app fallback, unavailable-file error (FR-2, FR-3, FR-11)
- [x] 4. Tracker: VLC HTTP, mpv pipe, PotPlayer messages, clock fallback; 15 s saves; end rules; 90 % completion; 15 s minimum (FR-4–FR-6, FR-10)
- [x] 5. Autoplay next in the open player, else `playback-ended` and the Up next card (FR-7)
- [x] 6. Session takeover for shared windows (FR-8)
- [x] 7. VLC one-instance adoption and `Elsewhere` detection (FR-9)
- [x] 8. Settings Video player card

## Verify

- [x] `cargo test -j 2`: `player::tests::the_playing_file_is_recognised`; `scanner` test covers continue-watching after progress
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner's PC, first launch of 1.0.0 | Pass |
| AC-2 | Product owner, installed builds since 1.0.0 (VLC and PotPlayer) | Pass |
| AC-3 | Product owner, installed builds (autoplay in VLC; Up next card) | Pass |
| AC-4 | Code path (`!exact && elapsed < 15`) | Not re-verified |
| AC-5 | Handed to the product owner with 1.1.10 (2026-10-08) with the test steps; result not yet reported | Open |
| AC-6 | Product owner, 1.0.0 | Pass |
| AC-7 | Product owner, 1.1.3 review (offline drive) | Pass |

## Close

- [x] `docs/FEATURES.md` §9, §10
- [x] README player table
- [x] Roadmap: Done
- [x] Decision log: D-002
- [x] Spec status Done
- [x] Spec index updated
