# 011. Rename files to TMDb names

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `dd4006d`; safeguards `8b647f9`) — written from the code on 2026-10-08 |
| **Area** | Library · TMDb · Settings |
| **Roadmap** | Done: Rename files to TMDb names |
| **Design doc** | `docs/FEATURES.md` §8 Rename |

## Problem

Release names (`3some.2009.SPANISH.1080p.WEBRip.x265-VXT.mp4`, `[Group] Shingeki no Kyojin - 01 [1080p].mkv`) are unreadable in Explorer and in every other app, and anime numbered straight through ("- 27") does not match the seasons TMDb uses. Once a title is matched, Vortex knows the proper name; the user wants the files and folders to carry it, without a single file being renamed wrongly or losing its progress, and with a way back.

## Goals

- Rename a matched movie's or series' files and folders to `Title (Year)` / `Show (Year) - SxxEyy - Episode` names, subtitles alongside.
- Show every change before anything happens; nothing changes until the user presses Rename.
- The library follows the rename: progress, posters, tags and collections stay attached; the next scan sees no change.
- Undo the last rename.

## Non-goals

- Renaming unmatched titles, extras folders, or files on offline drives.
- Moving files between folders or creating new folders.
- Custom naming templates.

## Where it lives

| Place | What |
| --- | --- |
| Title page → Rename files (matched titles) | Preview dialog for one title |
| Settings → TMDB → Review renames… | Preview for the whole library; Undo last rename |
| Background | Runs with scans held off |

## Requirements

- **FR-1.** The preview must list, per title, every file and folder that would change (old → new), notes for files left alone, and titles that are skipped with the reason; the user ticks which titles to rename.
- **FR-2.** Movies must become `Title (Year).ext`; episodes `Show (Year) - SxxEyy - Episode Title.ext` (double episodes `S01E01-E02` without a title; titles cut to 80 characters); characters Windows refuses must be replaced and reserved names avoided.
- **FR-3.** Several main files of one movie in one folder must be told apart by part number, by quality when every file has a distinct one, else by a counter.
- **FR-4.** Subtitles and other sidecar files named after the video must be renamed with it; one that also belongs to a longer-named video in the folder must not be taken.
- **FR-5.** A movie file whose own name carries a year more than one off TMDb's must be left alone with a note, and then no folder renamed for that title.
- **FR-6.** A series must be renamed whole or not at all: every episode must place on TMDb's seasons (anime numbered straight through is counted across seasons; a spelled-out `S01E27` is kept as it is; inside a season folder the count must land in that season), and two files for one episode that only a counter could separate must stop the series.
- **FR-7.** Season folders holding one season must become `Season NN` (`Specials` for 0) only inside the show's own folder; the title's own folder must be renamed only when its name reads as the title, it is not a library or holding one, holds no other title's files, and the new name is free.
- **FR-8.** A rename must be refused when the new name would read back as another kind, another year or an empty title, or would file the title together with another already in the library.
- **FR-9.** Applying must re-plan against the disk, rename each title all-or-nothing (putting earlier steps back on a failure), then update the library's paths, subtitles and title key; if the library cannot be updated the files must be put back.
- **FR-10.** Every applied batch must be recorded; Undo must reverse the last one and refuse if an original path exists again.
- **FR-11.** No scan may run while a rename or undo is in progress.

## Rules and edge cases

- Path limit 250 characters; existing targets and duplicate targets skip the title.
- Unavailable files skip the title ("Not available: … Is the drive connected?").
- TMDb's spelling becomes the library title only when it keys the same as the new file name; the year stays as it was.
- 20 batches kept for undo.

## Acceptance criteria

- **AC-1** (FR-1, FR-2, FR-4). `Inception.2010.1080p\Inception.2010.1080p.mkv` with `….en.srt`: preview shows the three moves; after Rename the folder is `Inception (2010)` with `Inception (2010).mkv` and `.en.srt`; the title keeps its poster, tags and paused position; a rescan reports no changes.
- **AC-2** (FR-6). `[Group] Shingeki no Kyojin - 27.mkv` under an Attack on Titan match becomes `Attack on Titan (2013) - S02E02 - …` in `Season 02`.
- **AC-3** (FR-5). A folder with `Dune 1984.mkv` and `Dune 2021.mkv` matched to Dune (2021): the 1984 file is noted and left; no folder rename.
- **AC-4** (FR-8). "Gojira" matched to Godzilla (1954) beside an existing "Godzilla (2014)": skipped with the "filed together with" reason.
- **AC-5** (FR-9). Rename while a file is open in a player: that title fails with the OS reason and its earlier steps are put back; other titles succeed.
- **AC-6** (FR-10). Undo last rename restores the names; the library follows.

## Principles

- Respects all principles. P3: previewed, all-or-nothing, undoable.

## Open questions

Answered 2026-10-08: rule for several copies (part / quality / counter; size plays no part); episodes and anime included.

## Changelog

- 2026-10-08: Built (`dd4006d`), spec written alongside the build but after it. Safeguards in `8b647f9`: reads-back check, sidecar ownership, anime `S2 - 05`, files put back when the library update fails, season folders only inside the show's folder. D-014.
- 2026-10-08: Spec recorded. Done.
