# 015. Confirm before removing a library folder: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Built 2026-10-08 (`5ba2545`), shipped as 1.1.11.

## Build

- [x] 1. `Library.file_count` from `list_libraries_rows` (FR-1, FR-2)
- [x] 2. Outlined Remove button and file count on each row (FR-1)
- [x] 3. Confirm dialog with the folder, count and disk note; Cancel/confirm paths; toast (FR-2–FR-4)

## Verify

- [x] `cd src-tauri && cargo test -j 1`
- [x] `pnpm build`
- [ ] Every acceptance criterion checked in the installed build (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Dev build in the build session; installed 1.1.11 not yet reported by the product owner | Pass (dev) |
| AC-2 | Dev build | Pass (dev) |
| AC-3 | Code path (count from the database) | Not re-verified |

## Close

- [x] `docs/FEATURES.md` §2 and Known gaps
- [x] README unchanged
- [x] Roadmap: moved to Done
- [x] Decision log: nothing new
- [x] Spec status Done with commit
- [x] Spec index updated
