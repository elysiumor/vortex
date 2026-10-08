# 006. Duplicates, history and statistics

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; fixes 2026-09-23 `4595596`, `310e8a9`) — written from the code on 2026-10-08 |
| **Area** | Library |
| **Roadmap** | Done: Duplicates, history and statistics |
| **Design doc** | `docs/FEATURES.md` §15 Duplicates, §16 History, §17 Statistics |

## Problem

Over years a collection gathers the same film twice in different qualities, and the same episode from two releases. Disk space goes, and the library shows two of everything. Separately, the user wants to look back: what did I watch last week, how far did I get, how many hours this year, which series have I abandoned.

## Goals

- Show every title and episode that exists more than once, with enough detail to choose which copy to drop, and drop it safely.
- Keep a log of every play session and manual mark, readable by day.
- Show how the library is used: hours, finished counts, genres, storage, series in progress and abandoned.

## Non-goals

- Automatic deletion or "keep the best quality" rules.
- Detecting duplicates by content.

## Where it lives

| Place | What |
| --- | --- |
| More → Duplicates (with a count badge) | Groups and files, Explorer, Delete |
| More → History | Tiles, day groups, entries |
| More → Statistics | Tiles and charts |
| Everywhere | History rows are written by playback (002) and manual marks |

## Requirements

### Duplicates

- **FR-1.** A movie with more than one main file, and a series episode present more than once (same season and episode), must be listed as a group with the title, year and episode code, and the number of copies.
- **FR-2.** Each file must show its quality tag, name, folder, size, duration, modified date, whether it carries watch progress, and whether it is available.
- **FR-3.** The page must show the total space recoverable (all but the largest file in each group).
- **FR-4.** Delete must ask first, then move the file to the Recycle Bin and drop it from the library; the other copies stay.
- **FR-5.** The More menu must show the number of duplicate groups.

### History

- **FR-6.** History must list each file once, at its most recent session, grouped by Today, Yesterday and date, newest first, with the time, title, episode, "Finished" or "Stopped at", and whether it was marked by hand or estimated.
- **FR-7.** Tiles must show sessions in the last 30 days, episodes finished this year, hours this year and sessions all time.
- **FR-8.** An entry can be removed; Clear history removes the whole log but keeps watched marks and positions. Each entry offers Play.

### Statistics

- **FR-9.** Tiles: hours watched, movies watched of total, episodes watched of total, series in the library, total size on disk.
- **FR-10.** Charts: hours per month (last 12 months with activity); genres finished most (top 8); storage by category (top 10); series in progress (10, most recent first) with progress bars; series untouched for 60 days or more ("Gathering dust").

## Rules and edge cases

- Hours count player sessions only: the full duration when the session completed and the duration is known, else the position reached.
- Extras never count as duplicates, episodes or watched episodes.
- An unavailable file cannot be deleted or revealed.
- Statistics must load with an empty library and with history older than 60 days.

## Acceptance criteria

- **AC-1** (FR-1–FR-4). Two files of one film in a library: Duplicates shows one group with both, the recoverable size equals the smaller file; Delete on the smaller confirms, the file lands in the Recycle Bin and the group disappears; the More badge drops to 0.
- **AC-2** (FR-6, FR-7). Watch one film three times, stopping further each time: History shows it once, at the last position; the "all time" tile counts three sessions.
- **AC-3** (FR-8). Clear history: the list empties, the film still shows "paused at".
- **AC-4** (FR-9, FR-10). Statistics loads on a fresh library and on one with a series last watched 200 days ago, which appears under Gathering dust.

## Principles

- Respects all principles. P3: deletion only to the Recycle Bin after a confirm.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-09-23 (`4595596`): Statistics failed on `strftime` typing; fixed with a test. (`310e8a9`): one history row per file.
- 2026-10-08 (`dd4006d`, `8b647f9`): Duplicates' delete button worked; pages no longer freeze on a sleeping drive.
- 2026-10-08: Spec written from the code. Done.
