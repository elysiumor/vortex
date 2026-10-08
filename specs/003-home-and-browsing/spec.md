# 003. Home, library pages, title page and search

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; redesigned 2026-10-08, `8b647f9`, spec 013) — written from the code on 2026-10-08 |
| **Area** | Shell · Library |
| **Roadmap** | Done: Home, library pages, title page, search |
| **Design doc** | `docs/FEATURES.md` §10 Home, §11 Movies/Series pages, §12 Title page, §13 Search, §23 UI shell |

## Problem

A library of several hundred titles needs a front page that answers "what was I watching?" and "what's new?", list pages that can be filtered and sorted, a page per title that shows everything about it and lets the user act on each file, and a way to find anything by typing. All of it must work without TMDb (posters are optional) and must not hide files the user owns.

## Goals

- Home shows what to continue, what is new, and a few rows worth browsing, with one title featured large.
- Movies and Series pages list every title with filters, sorting and a title filter box.
- A title page shows its files, extras, progress and TMDb details, and lets the user play, mark, fix the match, tag, collect, rename and reveal.
- Search finds titles, episode names and file names from anywhere; Ctrl+K jumps to titles, pages and actions.
- Keyboard works everywhere cards are shown.

## Non-goals

- A built-in player; per-user profiles; a server or remote access.
- Searching cast or crew (roadmap).

## Where it lives

| Place | What |
| --- | --- |
| Top bar | Logo, Home / Movies / Series / Collections / Downloads, More (History, Statistics, Duplicates, smart lists), search box, Ctrl K, theme, Settings |
| Home | Hero + rows |
| Movies, Series, smart-list pages | Grid with filters and sort |
| Title page | Hero, actions, details, seasons and files, extras |
| Search page | Replaces the page while the box has text |
| Ctrl+K | Command palette |

## Requirements

### Home

- **FR-1.** Home must show a featured title large: the first Continue watching item with its resume bar and minutes left, else a random unwatched matched title. A Shuffle button must pick another random unwatched one.
- **FR-2.** Home must show rows: Continue watching; Recently added (24); Top rated (rating ≥ 7.5, only when at least 3); "More <genre>" for the genre the user has finished most, when at least 3 unwatched titles are in it; Movies (30, unwatched first) and Series (30) with See all; Collections in progress (12).
- **FR-3.** An empty library must show a welcome with "Add a folder" leading to Settings.
- **FR-4.** Each Continue watching card must be dismissable; dismissing keeps the position, and playing the title again brings it back.

### Library pages

- **FR-5.** Movies and Series pages must list every title of that kind with a title filter, Watched/Unwatched/All, Category (when more than one), Genre, Tag, and sort by A→Z, newest year, recently added, recently watched, highest rated, largest. The sort must be remembered.
- **FR-6.** Each card must show the poster (or an initial), the title, a sort-appropriate subtitle, a rating badge, a watched tick, a progress bar for series, and Play on hover.
- **FR-7.** Play on a card must resume the partially watched episode, else start the first unwatched, else the first.

### Title page

- **FR-8.** The page must show the backdrop or poster, kind and category, TMDb title (library title when unmatched), original title, rating and votes, year, certification, runtime, seasons and episodes, status, language, genres, tagline and overview.
- **FR-9.** Actions: Back; category (editable in place); Fix match; Refresh and Rename files (when matched); Mark all watched/unwatched; a primary Play whose label says Resume at / Play SxxEyy / Play again / Watch again / Play / No file; Trailer; IMDb; TMDb; open the folder.
- **FR-10.** The page must show creators/director/writers/network or studio/release dates, tags (add with suggestions, remove), collections (add from a list or create one, remove), and cast with photos (8, then all).
- **FR-11.** Episodes must be grouped by season with TMDb season names, "x of y on disk" and the season overview; each row shows still, code, title, rating, air date, overview, duration, size, subtitles, watched/paused state, availability; actions: details, set paused time, mark watched/unwatched, Play; double-click plays.
- **FR-12.** The details panel must show file, folder, full path, exact size, modified, duration, subtitles, status, and offer Show in Explorer, Copy path, Copy folder, Reset progress.
- **FR-13.** Extras must be listed separately by their folder label, never counted as episodes, each playable.

### Search and keyboard

- **FR-14.** Typing in the search box must show matching titles (by name) and episodes (by TMDb title or file name) after a short pause; Esc clears; `/` focuses the box.
- **FR-15.** Ctrl+K must open a palette listing every title, every page, and actions: search everything, switch theme, rescan.
- **FR-16.** On any card grid or row: arrows move (row-aware), Home/End jump, Enter opens, P plays; Esc leaves a title page.

## Rules and edge cases

- Search debounce 150 ms; 30 titles, 40 episodes at most.
- "Paused time" accepts `23:14`, `1:05:00` or bare minutes.
- The hero keeps the same title across refreshes unless it finished, was dismissed or left the library.
- Cards without a poster show a gradient with the initial and the title. Unavailable files show "Offline" instead of Play.
- The title page shows a loading skeleton until the first load; a title that disappears is followed to its successor or the page goes back (spec 014, 1.1.11).

## Acceptance criteria

- **AC-1** (FR-1, FR-2). With a paused episode, Home's hero shows "Continue watching" with its bar and resumes on Play; rows appear per the counts above.
- **AC-2** (FR-5). Movies, Genre = Drama, sort Highest rated: only Drama films, ordered by rating, subtitles showing "★ x.x".
- **AC-3** (FR-9, FR-11). A series with one episode paused: the primary button says "Resume S01E03 at 12:30"; the season header reads "8 of 10 on disk".
- **AC-4** (FR-12). Details → Copy path puts the full path on the clipboard; Reset progress clears the paused state.
- **AC-5** (FR-14). Typing "pilot" lists episodes titled Pilot; typing a file-name fragment lists that file.
- **AC-6** (FR-15, FR-16). Ctrl+K, type a title, Enter opens it; on a grid, arrows and P work.
- **AC-7** (FR-3). A fresh install shows the welcome panel; "Add a folder" opens Settings.

## Principles

- Respects all principles.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08 (`dd4006d`): "Mark watched" no longer empties Continue watching; History and the title page no longer freeze on a sleeping drive.
- 2026-10-08 (`8b647f9`): redesign (spec 013): hero, carousel rows, Ctrl+K, light and dark.
- 2026-10-08: Spec written from the code. Done.
- 2026-10-08 (`5ba2545`, 1.1.11): Sync button and status pill in the top bar; TMDb page in the More menu; the title page follows a replaced title (spec 014).
