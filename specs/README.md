# Specs

Every feature or change in behaviour in Vortex starts here, as a spec the product owner approves before code is written. How it works: [Spec-driven development](../docs/spec-driven-development.md).

## Starting a spec

1. Take the next free number in the index below.
2. Copy [`_template/`](_template/) to `specs/NNN-short-name/` (for example `specs/018-watch-later/`).
3. Fill in `spec.md` and add a row to the index with status **Draft**.
4. Once it's approved, write `plan.md` and `tasks.md`, then build.

With Claude: `/spec <idea>`, then `/spec-plan NNN`, then `/spec-build NNN`.

## Statuses

| Status | Means |
| --- | --- |
| Draft | Being written or waiting for approval. No code yet. |
| Approved | The product owner approved it; planning or ready to build. |
| In progress | Being built from its task list. |
| Done | Built, verified, and `docs/FEATURES.md` updated. |
| Dropped | Not doing it; the spec says why. |
| Superseded | Replaced by a later spec. |

## Index

| # | Spec | Status | Area | Updated |
| --- | --- | --- | --- | --- |
| 001 | [Libraries and scanning](001-libraries-and-scanning/spec.md) · [plan](001-libraries-and-scanning/plan.md) · [tasks](001-libraries-and-scanning/tasks.md) | Done (1.0.0; retroactive) | Library · Settings | 2026-10-08 |
| 002 | [Playback with exact resume](002-playback-and-resume/spec.md) · [plan](002-playback-and-resume/plan.md) · [tasks](002-playback-and-resume/tasks.md) | Done (1.0.0, VLC one-instance 1.1.10; retroactive) | Playback · Settings | 2026-10-08 |
| 003 | [Home, library pages, title page and search](003-home-and-browsing/spec.md) · [plan](003-home-and-browsing/plan.md) · [tasks](003-home-and-browsing/tasks.md) | Done (1.0.0; retroactive) | Shell · Library | 2026-10-08 |
| 004 | [TMDb posters, details and episode titles](004-tmdb-posters-and-details/spec.md) · [plan](004-tmdb-posters-and-details/plan.md) · [tasks](004-tmdb-posters-and-details/tasks.md) | Done (1.0.0; retroactive) | TMDb · Settings | 2026-10-08 |
| 005 | [Collections, tags and smart lists](005-collections-tags-and-smart-lists/spec.md) · [plan](005-collections-tags-and-smart-lists/plan.md) · [tasks](005-collections-tags-and-smart-lists/tasks.md) | Done (1.0.0; retroactive) | Library | 2026-10-08 |
| 006 | [Duplicates, history and statistics](006-duplicates-history-and-statistics/spec.md) · [plan](006-duplicates-history-and-statistics/plan.md) · [tasks](006-duplicates-history-and-statistics/tasks.md) | Done (1.0.0; retroactive) | Library | 2026-10-08 |
| 007 | [File durations](007-file-durations/spec.md) · [plan](007-file-durations/plan.md) · [tasks](007-file-durations/tasks.md) | Done (1.0.0; retroactive) | Library · Settings | 2026-10-08 |
| 008 | [Tray, notifications, backup and restore](008-tray-notifications-backup/spec.md) · [plan](008-tray-notifications-backup/plan.md) · [tasks](008-tray-notifications-backup/tasks.md) | Done (1.0.0; retroactive) | Shell · Settings | 2026-10-08 |
| 009 | [Downloads and streaming, magnet links from the browser](009-downloads-and-streaming/spec.md) · [plan](009-downloads-and-streaming/plan.md) · [tasks](009-downloads-and-streaming/tasks.md) | Done (1.1.0; retroactive) | Downloads · Settings | 2026-10-08 |
| 010 | [Crashes survivable, logging and diagnostics, nothing on the main thread](010-logging-and-diagnostics/spec.md) · [plan](010-logging-and-diagnostics/plan.md) · [tasks](010-logging-and-diagnostics/tasks.md) | Done (1.1.1–1.1.3, CSP 1.1.11; retroactive) | Shell | 2026-10-08 |
| 011 | [Rename files to TMDb names](011-rename-to-tmdb-names/spec.md) · [plan](011-rename-to-tmdb-names/plan.md) · [tasks](011-rename-to-tmdb-names/tasks.md) | Done (1.1.8) | Library · TMDb · Settings | 2026-10-08 |
| 012 | [TMDb data kept across folder switches](012-tmdb-data-across-folder-switches/spec.md) · [plan](012-tmdb-data-across-folder-switches/plan.md) · [tasks](012-tmdb-data-across-folder-switches/tasks.md) | Done (1.1.9) | TMDb · Library | 2026-10-08 |
| 013 | [UI redesign: light and dark, hero and rows, Ctrl+K](013-ui-redesign/spec.md) · [plan](013-ui-redesign/plan.md) · [tasks](013-ui-redesign/tasks.md) | Done (1.1.9) | Shell | 2026-10-08 |
| 014 | [Sync button, status pill, and a title page that follows its files](014-sync-and-job-status/spec.md) · [plan](014-sync-and-job-status/plan.md) · [tasks](014-sync-and-job-status/tasks.md) | Done (1.1.11); installed-build check pending | Shell · Library · TMDb | 2026-10-08 |
| 015 | [Confirm before removing a library folder](015-library-remove-confirm/spec.md) · [plan](015-library-remove-confirm/plan.md) · [tasks](015-library-remove-confirm/tasks.md) | Done (1.1.11); installed-build check pending | Settings · Library | 2026-10-08 |
| 016 | [Remakes as separate titles](016-remakes-as-separate-titles/spec.md) · [plan](016-remakes-as-separate-titles/plan.md) · [tasks](016-remakes-as-separate-titles/tasks.md) | Draft: two open questions | Library · TMDb | 2026-10-08 |
| 017 | [A TMDb page: matching rules, API options, adult titles, unmatched list](017-tmdb-page-and-options/spec.md) · [plan](017-tmdb-page-and-options/plan.md) · [tasks](017-tmdb-page-and-options/tasks.md) | Done (1.1.11); installed-build check pending | TMDb · Settings | 2026-10-08 |

Specs 001–013 were written on 2026-10-08 from the code of 1.1.10, after the fact; their task lists record the tests and review rounds that existed at the time rather than a fresh verification. From 014 on, specs come first.
