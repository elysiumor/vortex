# Roadmap

The backlog, in three lists. Each item links its spec once it has one. How items become specs: [spec-driven development](spec-driven-development.md).

## Now

| Item | Spec | Notes |
| --- | --- | --- |
| Sync button, automatic refresh after renames, and a 0–100 % status for background work (scan, posters, rename, durations) | [014](../specs/014-sync-and-job-status/spec.md) | Being built in a separate session; combine when it finishes. Also answers "the title page goes blank after a rename" and "it keeps fetching after a bulk rename". |
| Confirm before removing a library folder | [015](../specs/015-library-remove-confirm/spec.md) | Draft. Also check why the Remove button isn't seen in the installed build. |
| Remakes as separate titles (Dune 1984 / 2021) | [016](../specs/016-remakes-as-separate-titles/spec.md) | Draft. A session proposal exists. |

## Next

| Item | Notes |
| --- | --- |
| Retry files whose duration could not be read | `duration_checked` is never cleared; "Read missing" skips them. |
| Smart list "to year" input | The field exists in the saved list, the dialog has no input for it. |
| Collection descriptions | `tags.overview` is never filled; TMDb collections have one. |
| Search cast and crew | The search box promises "people"; only titles, episode titles and file names are searched. |
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
