# 008. Tray, notifications, backup and restore: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); restore hardened 2026-10-08.

## Build

- [x] 1. Tray icon and menu, rebuild hooks (FR-1)
- [x] 2. Close to tray, Quit, single instance (FR-2, FR-3)
- [x] 3. Arrival notifications with the `notify_new` flag (FR-4)
- [x] 4. Backup: snapshot + posters zip, last-backup settings (FR-5)
- [x] 5. Restore: validation, refusal while jobs run, swap under lock and scan exclusion, rollback, poster path repointing (FR-6, FR-7)
- [x] 6. Reset all watch data with confirm (FR-8)

## Verify

- [x] `cargo test -j 2` (no dedicated test)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, installed builds | Pass |
| AC-2 | Product owner, installed builds | Pass |
| AC-3 | Product owner, installed builds | Pass |
| AC-4 | Product owner, 1.1.8/1.1.9 rounds | Pass |
| AC-5 | 1.1.8 review: a failed restore left the app usable | Pass |

## Close

- [x] `docs/FEATURES.md` §20, §21
- [x] README
- [x] Roadmap: Done
- [x] Decision log: nothing new
- [x] Spec status Done
- [x] Spec index updated
