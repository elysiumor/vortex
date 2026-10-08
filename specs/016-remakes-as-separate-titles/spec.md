# 016. Remakes as separate titles

| | |
| --- | --- |
| **Status** | Draft |
| **Area** | Library · TMDb |
| **Roadmap** | Now: Remakes as separate titles |
| **Design doc** | `docs/FEATURES.md` §4 Parsing and grouping, §15 Duplicates, §8 Rename, §7 memory |

## Problem

Titles group by name alone (D-003). "Dune (1984)" and "Dune (2021)", "The Lion King" 1994 and 2019, "Godzilla" 1954 and 2014 become one title with two files. The Duplicates page then offers to delete one as a copy of the other — a real data-loss risk in a classics collection. The TMDb match belongs to one of them, so the other shows the wrong poster. The rename refuses to touch the odd-year file, which keeps files safe but leaves the library wrong.

## Goals

- Films with the same name and different years are separate titles with their own posters, details and history.
- Files without a year still join the right title.
- Existing libraries are upgraded in place with nothing lost.
- Duplicates never pairs two different films.

## Non-goals

- Telling apart same-name, same-year films; identifying films by content.
- Series: a show's seasons and reboots keep today's grouping unless a year is in both names and differs (to be decided, open question 1).

## Where it lives

| Place | What |
| --- | --- |
| Background | Scan grouping, one-time upgrade of an existing database |
| Movies page, title pages, Duplicates, Rename, TMDb memory | Follow the new grouping |

## Requirements

- **FR-1.** A movie file whose name (or own folder) carries a year must group with files of the same name whose year is within one of it, and not with files whose year differs by more.
- **FR-2.** A movie file with no year must join the one same-name title that exists; when several exist (a remake pair), it must join the one whose year the TMDb match agrees with, else the earliest, and be noted in the log.
- **FR-3.** An existing database must be upgraded once at startup: a merged title whose files carry different years is split, each file's progress, history, duration and episode details moving to its new title, and tags and collections kept on both.
- **FR-4.** Each split title must find its own TMDb match (the memory's year rule already refuses a remembered match more than a year off).
- **FR-5.** Duplicates must never list files of different years as copies; real copies of one film (same year or no year) must still show.
- **FR-6.** The rename must no longer need the remake guard for files that are now separate titles; the guard must stay as a safety net for a title that still mixes years.
- **FR-7.** Titles with a year in the name ("Blade Runner 2049 (2017)", "Wonder Woman 1984 (2020)") must keep working as today.

## Rules and edge cases

- "Within one year" because release and copyright years often differ by one between a file name and TMDb.
- The grouping key is extended with the year when there is one; the memory is keyed the same way and upgraded alongside.
- A series folder reboot ("Battlestar Galactica" 1978 and 2004) is the open question.

## Acceptance criteria

- **AC-1** (FR-1). `Dune.1984.mkv` and `Dune.2021.mkv` in one folder appear as two titles, each with its own poster and details.
- **AC-2** (FR-2). `Dune.mkv` with no year added later joins the title TMDb matched for Dune (2021) when that is the only match, else the earliest.
- **AC-3** (FR-3). A 1.1.10 database with a merged "Dune" and a paused position on the 1984 file upgrades to two titles with the position on the 1984 one.
- **AC-4** (FR-5). Duplicates lists nothing for the Dune pair, and still lists two 1080p copies of one film.
- **AC-5** (FR-7). "Blade Runner 2049 (2017)" is one title, not grouped with "Blade Runner (1982)".

## Principles

- Respects all principles. P5 for the upgrade.

## Open questions

1. Series: should two shows with the same name and different years ("Battlestar Galactica" 1978 / 2004) also split? (Recommended: yes, with the same ±1 rule, since the same data-loss risk exists for episodes.)
2. On upgrade, when a title's files have different years and the TMDb match was picked by hand, which split keeps the manual match? (Recommended: the one whose year is within one of the match's; the other is searched afresh.)

## Changelog

- 2026-10-08: Drafted from the Known gaps entry and the session proposal "Separate remakes in the Vortex library".
