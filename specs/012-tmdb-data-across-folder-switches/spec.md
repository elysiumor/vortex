# 012. TMDb data kept across folder switches

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `8b647f9`) — written from the code on 2026-10-08 |
| **Area** | TMDb · Library |
| **Roadmap** | Done: TMDb data kept across folder switches |
| **Design doc** | `docs/FEATURES.md` §7 (Memory across folder switches), §1 (posters folder) |

## Problem

The user switches library folders often. Each time a folder came back, its titles were new to the database: every one was searched on TMDb again, posters were downloaded again, hand-picked matches were lost, and the API quota and minutes went with them.

## Goals

- A folder removed and added back finds its titles' matches, posters, details and episode names on disk, with no TMDb call.
- Hand-picked matches survive.
- Posters and details are stored in a way that outlives the library title they were fetched for.
- Settings shows what is stored.

## Non-goals

- Pruning the stored data; exporting it.

## Where it lives

| Place | What |
| --- | --- |
| Background | After every scan that changed something, before the poster fetch |
| Settings → TMDB | "Saved on this PC: N matched titles, N images (N MB)" |

## Requirements

- **FR-1.** What TMDb said about a title (match, poster, overview, rating, genres, year, whether it was picked by hand, or that nothing was found) must be remembered under the title's grouping key and kept when the title is removed.
- **FR-2.** Full details and season episode lists must be stored per TMDb id, not per library title, and shared by every title matched to the same id.
- **FR-3.** Poster and backdrop images must be stored by TMDb id; images stored under the old title-id names must be renamed once.
- **FR-4.** After a scan, a new title whose key is remembered must get its match, poster, details, genres, rating, collection and episode names from disk, unless the remembered year differs from the library's by more than one. A remembered "nothing found" must mark the title checked without a search.
- **FR-5.** A hand-picked match must never be overwritten by an automatic one, in the memory or on the title.
- **FR-6.** A rename (spec 011) must carry the memory to the new key.
- **FR-7.** Settings must report the number of remembered titles, cached details, images and their size.

## Rules and edge cases

- A poster image missing from disk is downloaded again from the remembered path; details come from the cache without a key.
- Titles remembered as not found are retried only by "Retry unmatched".

## Acceptance criteria

- **AC-1** (FR-1, FR-4). Remove a library whose titles are matched, add it back: titles show their posters and details within the scan, the log says "TMDb data reused from earlier, no API calls".
- **AC-2** (FR-5). Fix match a title by hand, remove and re-add the folder: the hand-picked match is back.
- **AC-3** (FR-4). "The Lion King" (2019) added next to a remembered 1994 match: it is searched on its own, not given the 1994 data.
- **AC-4** (FR-7). Settings shows the store line with non-zero counts after a fetch.

## Principles

- Respects all principles.

## Open questions

Answered 2026-10-08: store posters permanently and reuse them when switching folders; save API usage. (The product owner's request.)

## Changelog

- 2026-10-08: Built (`8b647f9`). D-012, D-013.
- 2026-10-08: Spec recorded. Done.
