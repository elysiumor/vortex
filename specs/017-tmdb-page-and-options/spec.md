# 017. A TMDb page: matching rules, API options, adult titles, unmatched list

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `5ba2545`, shipped in 1.1.11) |
| **Area** | TMDb · Settings |
| **Roadmap** | Done: TMDb page and API options |
| **Design doc** | `docs/FEATURES.md` §7 TMDb (Matching and API options, Unmatched titles), §22 Settings |

## Problem

TMDb lived in one crowded Settings card. Some titles never matched because TMDb hides films it flags as adult unless asked ("Monella" never matched "Frivolous Lola"). The language, region, year rule, certification country, image sizes, pacing and cache age were fixed in code, and there was no list of what was still unmatched.

## Goals

- Everything TMDb on one page: connection, matching rules, API options, fetch tools with Stop, what is stored, unmatched titles, file renaming.
- Adult titles can be matched: always by hand, and automatically when switched on.
- Every parameter TMDb's API accepts on the calls Vortex makes is a setting with the previous behaviour as its default.

## Non-goals

- Re-downloading existing images when a size changes.
- Requesting keywords, reviews, recommendations or watch providers (nothing shows them).

## Where it lives

| Place | What |
| --- | --- |
| More → TMDb, Ctrl+K "TMDb", Settings → "Open TMDb settings" | The page |
| Title page → Fix match | Always includes adult titles |

## Requirements

- **FR-1.** The TMDb page must hold: Connection (key, masked status, Remove key, link to get a key); Matching (auto-match on/off, include adult titles, retry without the year, language, refresh-details-after days); API options (search region, year filter any/original release, certification country, poster/backdrop/still/cast photo sizes, pause between calls, call timeout); Posters & details (Fetch missing posters, Stop while fetching, Retry unmatched, progress with %, store size); Unmatched titles (every title without a match, first 150, each opening its page); File names (Review renames…, Undo last rename).
- **FR-2.** Settings' TMDb card must become a link to the page.
- **FR-3.** Fix match must always search with adult titles included; automatic matching only with the setting on.
- **FR-4.** Every option must take effect on the next call without a restart; changing the language must clear the cached genre names; already fetched text stays until Refresh.
- **FR-5.** Defaults must equal the previous fixed behaviour: auto-match on, adult off, year fallback on, en-US, no region, year = any release, US certification, w342/w1280/w300/w185, 120 ms, 20 s, 90 days (0 = keep for ever).
- **FR-6.** The pause must be capped at 10 s and the timeout kept between 5 and 120 s.

## Rules and edge cases

- A blank or unparsable setting keeps the default. The region is stored upper-case; "TMDb default" stores an empty value.
- Image sizes apply to images fetched from then on (Known gaps).
- With auto-match off, only Fix match and the page's buttons ask TMDb.

## Acceptance criteria

- **AC-1** (FR-1, FR-2). More → TMDb shows the six cards; Settings' TMDb card has only "Open TMDb settings".
- **AC-2** (FR-3). Fix match on a title named "Monella" lists the adult-flagged film; with "Include adult titles" off, "Fetch missing posters" leaves it unmatched; on, it matches.
- **AC-3** (FR-4). Set the language to Deutsch and press Refresh on a matched film: the overview and genres are German; set it back and Refresh: English.
- **AC-4** (FR-1). With unmatched titles, the Unmatched list shows them with year · kind · category and opens the title page on click.
- **AC-5** (FR-5, FR-6). A fresh install behaves exactly as 1.1.10 did; entering 99999 ms for the pause stores 10000.

## Principles

- Respects all principles. P8: the key still never reaches the UI.

## Open questions

None open.

## Changelog

- 2026-10-08: Built in the parallel session (`5ba2545`), shipped as 1.1.11; spec recorded from the code. Done.
