# 004. TMDb posters, details and episode titles

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; matching fixes 2026-10-08, `dd4006d`, `8b647f9`) — written from the code on 2026-10-08 |
| **Area** | TMDb · Settings |
| **Roadmap** | Done: TMDb posters, details and episode titles |
| **Design doc** | `docs/FEATURES.md` §7 TMDb integration |

## Problem

File names tell the user "Inception.2010.1080p"; they do not show a poster, a cast, a rating or episode names. Without them the library is a list of file names, and series episodes are codes. The user has a free TMDb key and wants the library to look and read like a catalogue, without being asked for the key again, without hammering the API, and without a wrong match silently replacing a right one.

## Goals

- With a key entered once, every new title gets a poster, overview, rating and genres on its own.
- A title page shows full details: cast with photos, crew, trailer, certification, seasons, collection.
- Series episodes show their TMDb names, stills, air dates and ratings.
- A wrong automatic match can be fixed by hand and stays fixed.
- Everything fetched is cached locally; the app keeps working offline and never sends the key to the UI.

## Non-goals

- Other metadata sources (TVDB, OMDb, local `.nfo`).
- Writing metadata back into files.
- Keeping data across folder removal (spec 012 added that).

## Where it lives

| Place | What |
| --- | --- |
| Settings → Posters & details | Key entry and masked status, Remove, Fetch missing posters, Retry unmatched, progress, store size |
| Background | Poster fetch after scans that changed something |
| Title page | Details, Fix match, Refresh |
| Everywhere | Posters on cards, genres and ratings in filters and sorts |

## Requirements

- **FR-1.** The user must be able to enter a v3 key or a v4 read token. It must be checked against TMDb before it is saved, shown afterwards only as its last four characters, and never be sent to the UI.
- **FR-2.** After a scan that added titles, with TMDb connected, every title without a poster that has not been checked before must be searched (title with year, then without) and matched to the first result: poster, overview, rating, genres, and for a movie its collection.
- **FR-3.** A title TMDb has nothing for must be marked checked and not searched again automatically. "Retry unmatched" must search those again. A title that is matched must never be re-searched automatically.
- **FR-4.** A matched title whose poster download failed must keep its match and have only the image retried later.
- **FR-5.** The user must be able to fix a match from the title page by searching TMDb and picking a result. A hand-picked match must never be replaced by an automatic one.
- **FR-6.** The title page must show full details fetched once and cached for 90 days; Refresh must fetch again (details, backdrop, episode titles). With no key, cached details must still be shown.
- **FR-7.** For a matched series, episode names, overviews, stills, air dates and ratings must be filled from TMDb per season present in the library, cached, and refetched only when an episode is missing from the cache or on Refresh.
- **FR-8.** Settings must show fetch progress (done / total, matched, current title) and the outcome, and must say how much TMDb data is stored locally.
- **FR-9.** Requests must be paced (a short pause between titles) and an authentication or network failure must stop the run with a message rather than retry every title.
- **FR-10.** Posters and backdrops must be stored by TMDb id so titles matched to the same film share one file and a reused title id can never show another film's picture.

## Rules and edge cases

- Language pinned to en-US; adult results excluded. Image sizes: poster w342, backdrop w1280, still w300, cast photo w185 (stills and photos are remote URLs, not cached files).
- `best_match` takes the first result; no scoring. The year is sent as `year` (movies) or `first_air_date_year` (series).
- 120 ms between titles in a fetch; 100 ms between season fetches.
- A details fetch for a title runs once at match time (one extra API call per title) and writes rating, genres and collection onto the title.
- Removing the key keeps posters and cached data; new titles stop getting them.

## Acceptance criteria

- **AC-1** (FR-1). Paste a valid key: "TMDB connected", the field shows ••••xxxx; a wrong key shows TMDb's rejection and nothing is saved.
- **AC-2** (FR-2). Add a folder with "Inception.2010.1080p.mkv": within the fetch, the card shows the Inception poster, the title page its overview, rating, genres and the "Inception" collection is created when applicable.
- **AC-3** (FR-3, FR-8). A home video gets no match, is shown once in the progress as unmatched, and is not searched on the next scan; "Retry unmatched" searches it again.
- **AC-4** (FR-5). Fix match on a wrongly matched film to the right one: poster and details change; a later "Fetch missing posters" does not change it back.
- **AC-5** (FR-7). A matched series shows "S01E01 Pilot" with a still and air date; an episode added later gets its name on the next fetch.
- **AC-6** (FR-6). Disconnect from the network and open a title page seen before: details still show, with the fetched date.
- **AC-7** (FR-9). With a revoked key, "Fetch missing posters" stops at the first title with "Poster fetch stopped: TMDB rejected the API key".

## Principles

- Respects all principles. P8: the key stays in the backend.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-09-23 (`4595596`): a poster-fetch loop between Settings and the scan event fixed.
- 2026-10-08 (`dd4006d`, `8b647f9`): a failed poster download no longer discards the match; a remembered match is not applied to a different film of the same title; changing a match drops the old poster; images stored by TMDb id (D-012).
- 2026-10-08: Spec written from the code. Done.
- 2026-10-08 (`5ba2545`, 1.1.11): the Settings card became a TMDb page with matching rules and API options; Fix match includes adult titles; a Stop button for the fetch. See spec 017 and spec 014.
