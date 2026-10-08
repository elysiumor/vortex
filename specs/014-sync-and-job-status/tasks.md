# 014. Sync button, status pill, title page successor: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Built 2026-10-08 in a parallel session (`5ba2545`), shipped as 1.1.11 (`209e659`).

## Build

- [x] 1. `jobs::report` / `finished` with per-job thinning; `Job::release` (FR-1)
- [x] 2. Scan progress: walk and write phases, per-library slice (FR-4)
- [x] 3. Poster fetch progress, cancel flag, Stop command (FR-1, FR-5)
- [x] 4. Duration, rename and memory-restore progress (FR-1)
- [x] 5. Status pill in the top bar with order, percent, "+N", bar, Stop (FR-2)
- [x] 6. Sync button and Ctrl+K wiring, refusal while scanning (FR-3)
- [x] 7. Title page: load sequencing, `item_for_paths`, successor switch or back (FR-6)
- [x] 8. TMDb page percent and Stop (FR-7)

## Verify

- [x] `cd src-tauri && cargo test -j 1` (44 tests; `-j 2` crashed rustc when memory was short — the build note now says `-j 1`)
- [x] `pnpm build`
- [ ] Acceptance criteria in the installed 1.1.11 (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Dev build during the build session (pill phases and Synced toast) | Pass (dev) |
| AC-2 | Dev build: Stop ends the fetch with "stopped by you" | Pass (dev) |
| AC-3 | Code path (`jobs.value.scan?.running` guard) | Not re-verified |
| AC-4 | Dev build: successor switch after an external rename | Pass (dev); delete case not re-verified |
| AC-5 | Dev build: rename and duration labels in the pill | Pass (dev) |

Installed-build confirmation by the product owner is pending; 1.1.11 installers are at `src-tauri\target\release\bundle\{msi,nsis}`.

## Close

- [x] `docs/FEATURES.md` §3, §6–§8, §12, §23–§25 and Known gaps (blank page removed; pill width added)
- [x] README unchanged
- [x] Roadmap: moved to Done
- [x] Decision log: nothing new
- [x] Spec status Done with commit
- [x] Spec index updated
