# 005. Collections, tags and smart lists

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`) — written from the code on 2026-10-08 |
| **Area** | Library |
| **Roadmap** | Done: Collections, tags and smart lists |
| **Design doc** | `docs/FEATURES.md` §14 Collections and tags, §11 (smart lists) |

## Problem

A flat list of titles is not how people think about a library. Film series (Alien, Harry Potter) belong in release order; the user's own groupings ("Sunday afternoon", "with the kids") cut across genres; and a filter the user applies every week ("unwatched 4K dramas rated over 7.5") should be one click, not five.

## Goals

- Film series known to TMDb become ordered collections on their own.
- The user can make and order their own collections and tag titles freely.
- A set of filters can be saved as a smart list that updates itself.

## Non-goals

- Sharing or exporting lists; nested collections; collection posters chosen by hand.

## Where it lives

| Place | What |
| --- | --- |
| Collections page | Cards, create; inside one: rename, delete, reorder, remove, Play next |
| Title page | Tags input; "+ collection" |
| Movies/Series pages | Tag filter; "Smart list" save button |
| More menu | Smart lists, with delete |
| Home | "Collections in progress" row |

## Requirements

- **FR-1.** A matched movie that TMDb lists in a collection must be added to that collection, created on first sight with TMDb's poster, and members must be kept in release-year order.
- **FR-2.** The user must be able to create a collection by name, rename it, delete it (the titles stay), add any title to it from the title page, remove a title, and move titles up or down.
- **FR-3.** A collection card must show the first member's poster (or TMDb's), "N titles", "from TMDB" when applicable, and watched/total with a progress bar and a tick when complete.
- **FR-4.** Inside a collection, Play next must play the first unwatched episode of the first member that is not fully watched.
- **FR-5.** The user must be able to add free-text tags to a title with suggestions from existing tags, remove them, and filter the library pages by tag. A tag nobody uses any more disappears.
- **FR-6.** From a library page the user must be able to save the current filters (watched state, category, genre, tag) plus optional minimum rating, "untouched for N days" and "from year", with a name, as a smart list that appears in the More menu and shows its rules as chips when opened.
- **FR-7.** A smart list must be deletable from the menu; deleting the open one returns to Home.

## Rules and edge cases

- Collections and tags are the same table with a kind; names are unique per kind, case-insensitive.
- "Untouched for N days" excludes titles played within N days; never-played titles are included.
- Home shows up to 12 non-empty, unfinished collections.
- A collection's description field is never filled (Known gaps).

## Acceptance criteria

- **AC-1** (FR-1). Match "Alien" and "Aliens": a collection "Alien Collection" appears with Alien first, from TMDB.
- **AC-2** (FR-2, FR-4). Create "Weekend", add two series from their pages, move the second up, press Play next: the first unwatched episode of the first member plays.
- **AC-3** (FR-5). Tag a film "kids"; the Movies page Tag filter lists "kids" and shows only it; remove the tag: the filter option disappears.
- **AC-4** (FR-6, FR-7). On Series with Unwatched + Genre Drama, save a smart list "Unwatched drama" with min rating 7: it appears under More, opens with the chips, lists only matching series; delete it from the menu.

## Principles

- Respects all principles.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08: Spec written from the code. Done.
