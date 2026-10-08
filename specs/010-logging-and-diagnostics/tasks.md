# 010. Logging and diagnostics: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-23 (1.1.1–1.1.3); improved 2026-10-08.

## Build

- [x] 1. File logging with rotation, local time, env filter (FR-1, FR-7)
- [x] 2. Startup summary and clean-exit line (FR-2)
- [x] 3. Panic hook, unwind profile, line tables (FR-3)
- [x] 4. Renderer diagnostics and `log_frontend` (FR-4)
- [x] 5. Blocking helpers; availability off the lock; player launch, reveal, drives, backups on blocking threads (FR-5)
- [x] 6. Show log button (FR-6)

## Verify

- [x] `cargo test -j 2` (no dedicated test)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner's log, read during the 1.1.8 round (settings line, libraries) | Pass |
| AC-2 | Product owner's log showed `get_details` at 8–10 s during a poster fetch (1.1.7 round) | Pass |
| AC-3 | 1.1.9 review round: History, Continue watching, Duplicates and search no longer froze on a sleeping drive | Pass |
| AC-4 | Product owner, installed builds | Pass |

## Close

- [x] `docs/FEATURES.md` §26, §28
- [x] README
- [x] Roadmap: Done
- [x] Decision log: D-010, D-015
- [x] Spec status Done
- [x] Spec index updated
