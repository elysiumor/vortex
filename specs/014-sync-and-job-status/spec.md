# 014. Sync button, status pill, and a title page that follows its files

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `5ba2545`, shipped in 1.1.11) |
| **Area** | Shell · Library · TMDb |
| **Roadmap** | Now → Done: Sync button, automatic refresh and job status |
| **Design doc** | `docs/FEATURES.md` §3 (progress), §6, §7, §8 (progress), §12 (successor), §23 (status pill, Sync), §24 (`job-progress`) |

## Problem

After a bulk rename of 233 files, the product owner saw the app "keep fetching" with no way to tell what was running — a scan, the poster fetch, the TMDb memory being re-applied — or how far along it was. The poster fetch showed progress only on the Settings page; scans, renames and duration reads showed nothing anywhere. There was no way to stop a long fetch.

Separately, a title page went blank after a rename: a rescan had filed the renamed files under a new title and the page sat on its loading skeleton for ever, because the id it was showing no longer existed. And there was no one-click way to rescan and refresh from wherever the user was.

## Goals

- Everything running in the background is visible in one place, with a name, what it is on, and 0–100 % where the extent is known.
- A long poster fetch can be stopped.
- One Sync button rescans every library and refreshes the screen, from any page.
- A title page never goes blank: when its title is replaced it follows the files to the new one; when they are gone it says so and goes back.

## Non-goals

- Per-library progress bars in Settings; a job history.
- Cancelling scans, renames or duration reads mid-way (a rename is all-or-nothing per title; a scan finishes its library).

## Where it lives

| Place | What |
| --- | --- |
| Top bar | Status pill (wide windows), Sync button |
| Ctrl+K | "Rescan libraries" runs the same Sync |
| TMDb page | Fetch progress with %, Stop button |
| Title page | Successor handling |
| Background | Every job reports on one channel |

## Requirements

- **FR-1.** Every background job — scan, poster fetch, duration read, rename, TMDb data restore — must report its name, what it is working on, and done/total (total 0 when unknown), and must say when it has finished.
- **FR-2.** The top bar must show a pill while any job runs: spinner, label and detail, percent when known, "+N" for further running jobs (all listed in a tooltip), a thin progress bar (pulsing while indeterminate), and an X to stop a poster fetch. Jobs are shown in the order rename, scan, memory restore, posters, durations.
- **FR-3.** A Sync button must rescan every library, refresh every open view, toast "Synced: N files, N added, N renamed, N removed, N offline", spin while syncing or while any job runs, and refuse with a toast when a scan is already running.
- **FR-4.** A scan must report which library it is walking and how many files it has seen, then how many rows it is writing of how many; each library is an equal share of the percentage.
- **FR-5.** A poster fetch must be stoppable: the running pass ends at the next title, the outcome says "stopped by you", no queued pass runs, and the titles not reached remain unchecked for the next fetch.
- **FR-6.** A title page whose title no longer exists must look up which title now holds any of the files it was showing, switch to it when there is one, and otherwise toast "This title is no longer in the library" and go back. An older load must never overwrite a newer one.
- **FR-7.** The TMDb page must show the fetch percentage next to its counts.

## Rules and edge cases

- Progress events are thinned to one per 150 ms per job, except the first and the last.
- The memory-restore job reports only when it has candidates.
- The pill is shown only at the `lg` breakpoint and wider; the Sync icon still spins in narrower windows (Known gaps).
- A scan asked for while one runs is still served by one more pass (001); only the Sync button refuses up front, because its result would not be its own.

## Acceptance criteria

- **AC-1** (FR-1, FR-2, FR-4). Press Sync on a two-library setup: the pill reads "Scanning library · <name> · N files", then "Updating library" with a percentage; it disappears when the scan ends and the Synced toast appears.
- **AC-2** (FR-2, FR-5). Start "Fetch missing posters" with many unmatched titles: the pill shows "Fetching posters · <title> · 37 %"; press its X: the fetch stops, the TMDb page says "Poster fetch stopped: stopped by you", and the next fetch continues with the titles not reached.
- **AC-3** (FR-3). Press Sync while the watcher's scan is running: "Already syncing; the library refreshes when it finishes."
- **AC-4** (FR-6). Open a film's page, rename its file in Explorer to a different title, wait for the watcher's scan: the page switches to the new title instead of a skeleton. Delete the file instead: the toast appears and the page goes back.
- **AC-5** (FR-1). During a bulk rename the pill reads "Renaming files · <title> · 42 %"; during a duration read, "Reading durations · <file>".

## Principles

- Respects all principles. P4 and P9.

## Open questions

None open. The product owner asked for "a status 0 to 100 % so that I can know what is happening behind the scenes", a Sync button, and auto refresh without a blank page; all three are in.

## Changelog

- 2026-10-08: Requested by the product owner (sync button, auto refresh after renames without a blank page, 0–100 % status for posters and renames, "it keeps fetching after a bulk rename").
- 2026-10-08: Built in a parallel session (`5ba2545`) with the TMDb page (spec 017) and the library-remove confirmation (spec 015); shipped as 1.1.11 (`209e659`). Spec written alongside from the code. Done.
