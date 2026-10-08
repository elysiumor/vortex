# Roadmap

The backlog, in three lists. Each item links its spec once it has one. How items become specs: [spec-driven development](spec-driven-development.md).

## Now

| Item | Spec | Notes |
| --- | --- | --- |
| Remakes as separate titles (Dune 1984 / 2021) | [016](../specs/016-remakes-as-separate-titles/spec.md) | Draft; two open questions for the product owner. A session proposal exists. |
| Installed-build check of 1.1.11 | [014](../specs/014-sync-and-job-status/tasks.md), [015](../specs/015-library-remove-confirm/tasks.md), [017](../specs/017-tmdb-page-and-options/tasks.md) | Acceptance criteria were checked in the dev build; the product owner's confirmation on the installed build is pending. |

## Next

| Item | Notes |
| --- | --- |
| Status pill in narrow windows | Hidden below the `lg` breakpoint; only the Sync icon spins there. |
| Match scoring | Automatic matching takes the first search result; a title-similarity or "prefer exact title" rule would cut Fix match work. |
| Re-download images when a size changes | Sizes apply only to images fetched afterwards. |
| Retry files whose duration could not be read | `duration_checked` is never cleared; "Read missing" skips them. |
| Smart list "to year" input | The field exists in the saved list, the dialog has no input for it. |
| Collection descriptions | `tags.overview` is never filled; TMDb collections have one. |
| Search cast and crew | The search box promises "people"; only titles, episode titles and file names are searched. |
| Forget TMDb data when removing a library | Open question from spec 015. |
| Torrent polish: seed-ratio limit, scheduling | Phase 4 of the downloads plan; bandwidth caps are done. Must stay within the proxy-mode rules (D-006). |
| Faster "Apply" for download settings | The engine restart pauses about a second (librqbit). |

## Parked

| Item | Why |
| --- | --- |
| A built-in player | D-002: external players only. |
| Any kind of content search or index | D-004: never. |
| Adapter binding for the torrent engine on Windows | librqbit supports it on Linux/macOS only; the proxy is the kill switch (D-006). |

## Done

| Shipped | Item | Spec |
| --- | --- | --- |
| 1.0.0 (2026-09-20) | Libraries and scanning | [001](../specs/001-libraries-and-scanning/spec.md) |
| 1.0.0 | Playback with exact resume | [002](../specs/002-playback-and-resume/spec.md) |
| 1.0.0 | Home, library pages, title page, search | [003](../specs/003-home-and-browsing/spec.md) |
| 1.0.0 | TMDb posters, details and episode titles | [004](../specs/004-tmdb-posters-and-details/spec.md) |
| 1.0.0 | Collections, tags and smart lists | [005](../specs/005-collections-tags-and-smart-lists/spec.md) |
| 1.0.0 | Duplicates, history and statistics | [006](../specs/006-duplicates-history-and-statistics/spec.md) |
| 1.0.0 | File durations | [007](../specs/007-file-durations/spec.md) |
| 1.0.0 | Tray, notifications, backup and restore | [008](../specs/008-tray-notifications-backup/spec.md) |
| 1.1.0 (2026-09-23) | Downloads and streaming, magnet links from the browser | [009](../specs/009-downloads-and-streaming/spec.md) |
| 1.1.1–1.1.3 (2026-09-23) | Crashes survivable, logging and renderer diagnostics, nothing on the main thread | [010](../specs/010-logging-and-diagnostics/spec.md) |
| 1.1.8 (2026-10-08) | Rename files to TMDb names | [011](../specs/011-rename-to-tmdb-names/spec.md) |
| 1.1.9 (2026-10-08) | TMDb data kept across folder switches | [012](../specs/012-tmdb-data-across-folder-switches/spec.md) |
| 1.1.9 (2026-10-08) | UI redesign: light and dark, carousel rows, Ctrl+K | [013](../specs/013-ui-redesign/spec.md) |
| 1.1.10 (2026-10-08) | VLC single-instance tracking | [002](../specs/002-playback-and-resume/spec.md) (amendment) |
| 1.1.11 (2026-10-08) | Sync button, status pill with 0–100 %, title page follows its files, Stop for the poster fetch | [014](../specs/014-sync-and-job-status/spec.md) |
| 1.1.11 | Confirm before removing a library folder, file count per library | [015](../specs/015-library-remove-confirm/spec.md) |
| 1.1.11 | TMDb page: matching rules, API options, adult titles, unmatched list | [017](../specs/017-tmdb-page-and-options/spec.md) |
| 1.1.11 | Content Security Policy | [010](../specs/010-logging-and-diagnostics/spec.md) (amendment, D-017) |
