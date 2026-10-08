# 007. File durations: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-19 (`9ab6f90`); console fix 2026-10-08.

## Build

- [x] 1. Columns `duration_secs`, `duration_checked`; `ffprobe_path` (FR-1, FR-3)
- [x] 2. Native MKV/MP4 readers; ffprobe fallback; detection paths (FR-1, FR-2)
- [x] 3. Probe job under `probe_job`, progress events, started after scans (FR-5)
- [x] 4. Hidden child processes (FR-4)
- [x] 5. Settings card (FR-2, FR-5)
- [x] 6. Player-reported length stored (FR-6)

## Verify

- [x] `cargo test -j 2` (no dedicated test)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Product owner, installed builds since 1.0.0 | Pass |
| AC-2 | Product owner's PC (WinGet FFmpeg detected) | Pass |
| AC-3 | Product owner, 1.1.8 (the "application cmd spam" report, fixed and confirmed) | Pass |
| AC-4 | Product owner, installed builds | Pass |

## Close

- [x] `docs/FEATURES.md` §6
- [x] README (durations mentioned under setup)
- [x] Roadmap: Done; retry item under Next
- [x] Decision log: nothing new
- [x] Spec status Done
- [x] Spec index updated
