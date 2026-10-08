# 009. Downloads and streaming: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-09-20 (`7b74654`); fixes through 2026-10-08.

## Build

- [x] 1. Engine lifecycle, settings, persistence, proxy-mode options (FR-1, FR-2, FR-4)
- [x] 2. Add flow: inspect, picker, free-space check, staging per hash, destinations (FR-5, FR-6)
- [x] 3. Completion watcher, move into place, staging tidy, `torrent-done`, scan (FR-7)
- [x] 4. Rows, actions, expanded tabs, session bar, polling (FR-8, FR-9)
- [x] 5. Stream: resolve, largest video, head buffer, resume position, tracker callback, Keep/Discard/Decide later (FR-10–FR-13)
- [x] 6. Unprotected warning and badge (FR-3)
- [x] 7. Deep links: `vortex://`, `magnet:` handler switch, pending queue (FR-14)
- [x] 8. Settings Downloads card

## Verify

- [x] `cargo test -j 2`: `torrent::tests::strips_only_udp_trackers`, `bare_hashes_become_magnets`, `staging_is_removed_only_when_empty`, `safe_folder_names`, `finished_files_land_in_a_named_folder`, `unique_dest_appends_a_counter`, `session_starts_in_both_modes` (DHT and listener off behind a proxy, stream endpoint answers); `deeplink::tests::extracts_magnets`; `player::tests::the_playing_file_is_recognised`
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | `session_starts_in_both_modes` + product owner's VPN proxy in installed builds | Pass |
| AC-2 | Product owner, installed builds 1.1.x; `finished_files_land_in_a_named_folder` | Pass |
| AC-3 | Product owner, installed builds | Pass |
| AC-4 | 1.1.8/1.1.9 review rounds (Keep during a stream, paused-then-resumed download) | Pass |
| AC-5 | Product owner, 1.1.0 | Pass |
| AC-6 | 1.1.8 review round | Pass |

## Close

- [x] `docs/FEATURES.md` §18, §19
- [x] README Downloads section
- [x] Roadmap: Done; polish items under Next
- [x] Decision log: D-004 to D-009
- [x] Spec status Done
- [x] Spec index updated
