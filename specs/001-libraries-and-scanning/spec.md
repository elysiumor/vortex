# 001. Libraries and scanning

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; hardened 2026-10-08, `dd4006d`) — written from the code on 2026-10-08 |
| **Area** | Library · Settings |
| **Roadmap** | Done: Libraries and scanning |
| **Design doc** | `docs/FEATURES.md` §2 Libraries, §3 Scanning, §4 Parsing and grouping, §5 Folder watcher |

## Problem

The user's films and series are spread over internal and external drives in folders named by whoever released them: `The.Bear.S03.1080p.WEB.h264-GRP`, `Inception (2010) [1080p] [YTS.MX]`, `Anime\Oshi no Ko\[SubsPlease] Oshi no Ko S2 - 05.mkv`. Nothing on the PC knows that these are one show's seasons, one film and its featurettes, or an episode of a second season. Drives come and go; files get renamed; folders are added and removed.

Vortex needs to turn those folders into a library of titles and episodes it can show, play and track, keep it current without the user doing anything, and never lose the user's watch progress when a file is renamed or a drive is unplugged.

## Goals

- Add a folder or a whole drive and have every movie and episode in it appear, grouped under the right title, within one scan.
- Keep the library current automatically: files added, removed or renamed on disk are reflected without a manual rescan.
- Never lose progress: a renamed or moved file keeps its position and history; a drive that is unplugged is shown offline, not deleted.
- Keep junk out: samples, tiny clips, system folders, caches and half-finished downloads never become titles.

## Non-goals

- Identifying films by content (hashes, fingerprints). Grouping is by name only (D-003).
- Reading metadata from `.nfo` files or embedded tags.
- Telling remakes with the same title apart (spec 016).
- Watching network shares specially; they are plain folders.

## Where it lives

| Place | What |
| --- | --- |
| Settings → Libraries | Add folder, Add whole drive, Remove, Rescan all, Folders to skip, last-scan summary, online/offline dot per library |
| Background | Startup scan, folder watcher, drive-return scan, scans after a finished download; the scan job queue |
| Everywhere | `scan-done` refreshes every open view; a toast reports what changed |
| Tray | Rescan libraries |

## Requirements

### Libraries

- **FR-1.** The user must be able to add any folder as a library, and any drive as a whole. Adding starts a scan at once.
- **FR-2.** A folder inside an existing library must be refused with a message naming that library. A new folder that contains existing libraries must absorb them, keeping their files and progress.
- **FR-3.** A library is named after its folder and shown as online or offline according to whether the folder can be reached right now.
- **FR-4.** The user must be able to remove a library. Its titles, episodes, progress and history go with it; what TMDb said about those titles is kept for a later return.
- **FR-5.** The user must be able to name extra folder names to skip, in addition to the built-in list of system and cache folders.
- **FR-6.** Adding the Windows drive must be confirmed first, with the reason, and offer picking a folder instead.

### Scanning

- **FR-7.** A scan must find every video file (by extension) under each online library, skipping files under 20 MB, files with "sample" in the name, hidden folders, folders on the skip lists and symbolic links.
- **FR-8.** Each file must be parsed into a kind (movie or series), a title, a year where the name has one, and for series a season and episode number, using the file name and its parent folders.
- **FR-9.** Files that parse to the same title must group under one item per kind, whatever the release names look like; every season of a show is one title.
- **FR-10.** Bonus material (featurettes, trailers, behind the scenes, files in a season folder without an episode number) must attach to the title it belongs to as extras, labelled by its folders, and never count as an episode or become a title of its own.
- **FR-11.** A title's category must be the first folder under the library root ("Anime", "Hollywood"), unless that folder is the title's own or a season folder, in which case the library's name.
- **FR-12.** A file that disappears and reappears under another name with the same size and modification time must be treated as renamed: its progress, history, duration, added date and episode details move to the new row.
- **FR-13.** Files that are really gone are removed; titles left with no files are removed.
- **FR-14.** A folder or drive that cannot be read during a scan must not have its files treated as gone. A library that goes offline during the scan leaves the library untouched.
- **FR-15.** After every scan the user must see what changed (added, renamed, removed) unless they started the scan themselves from Settings, where the summary is shown in place.
- **FR-16.** Only one scan runs at a time. A scan asked for while one runs is served by one more pass, never dropped and never duplicated.

### Watching

- **FR-17.** With folder watching on, a file added, removed or renamed in any online library must be reflected in the library within a few seconds without user action; a burst of changes causes one scan.
- **FR-18.** A library that comes back online (an external drive plugged in) must be scanned automatically.
- **FR-19.** Scanning at startup and watching folders must each be switchable in Settings.

## Rules and edge cases

- Video extensions: mkv, mp4, avi, mov, wmv, flv, webm, m4v, ts, mpg, mpeg, m2ts, vob, 3gp, ogv.
- Episode patterns: `S01E02` (ranges `S01E01-E03`, `S01E01E02`), `1x02`, `Season 1 Episode 2`, `E05`/`Ep 05`/`Episode 5` with the season from the folder, and `Show - 05` inside the show's folder (season from an `S2` marker in the name, else 1). Dotted numbers after a code (`S01E01.1.23.45`) are not more episodes.
- Year: a bracketed year wins; otherwise the last year not at the start of the name ("Blade.Runner.2049.2017" → 2017, "Wonder Woman 1984 (2020)" → 2020); a name starting with a year keeps it as the title ("1917", "2012 (2009)"); the end of a range ("1999-2007") is not a year.
- Release tags cut the title only when something precedes them: "Dual (2022)", "Internal Affairs", "Season of the Witch" survive; before a year only unambiguous tags are cut, so "Charlotte's Web" and "A Complete Unknown" survive.
- The show folder name is preferred over the file name when the two agree (one is a prefix of the other, ignoring punctuation and case).
- Extras folders: featurettes, behind the scenes, deleted scenes, bloopers, making of, outtakes, gag reel, bonus/special features count anywhere; extras, bonus, specials, trailers, interviews, shorts, other, scenes, promos, teasers, webisodes count only inside a title's folder — directly under a library root they are a category.
- Built-in skip list: windows, program files, program files (x86), programdata, appdata, $recycle.bin, system volume information, recovery, perflogs, node_modules, .git, $windows.~bt, windows.old, msocache, steamapps, cache, .cache, temp, tmp, .incomplete. Hidden means the Windows Hidden attribute; the System attribute alone is not a reason to skip (folders with custom icons carry it).
- Watcher: 3 s debounce; only paths that are a video, a folder, or no longer exist, and that cross no skipped folder, count; availability is rechecked every 20 s.
- A manual scan waits for a running background scan instead of folding into it, so the numbers it reports are its own.
- Arrival toasts (Windows notifications) list up to six new titles and "… and N more", only for watcher, drive and download scans, and only with the notification setting on.

## Acceptance criteria

- **AC-1** (FR-1, FR-7–FR-11). Add a folder holding `Shows\Breaking Bad\Season 1\Breaking.Bad.S01E01.mkv`, `…S01E02.mkv`, `Season 2\breaking bad s02e01.mp4`, `Movies\Inception.2010.1080p.mkv`, `Movies\sample.mkv` and `notes.txt`. One series "Breaking Bad" with 3 episodes in category "Shows" and one movie "Inception" (2010) in "Movies" appear; the sample and the text file do not.
- **AC-2** (FR-10). A `Featurettes\Behind The Scenes\s01e02 - behind the scenes.mkv` under a season folder appears under the series as an extra labelled "Season 1 › Featurettes › Behind The Scenes" and the series still counts 2 episodes.
- **AC-3** (FR-12). Rename an episode file on disk, rescan: the series shows the new file name with the same paused position; the scan reports 1 renamed, 0 added, 0 removed.
- **AC-4** (FR-13). Delete an episode file, rescan: the episode is gone and the series' episode count drops by one.
- **AC-5** (FR-2). Adding `F:\Media\Anime\Movies` while `F:\Media\Anime` is a library is refused; adding `F:\Media` folds `F:\Media\Anime` in and its progress is kept.
- **AC-6** (FR-14). Unplug a drive mid-scan: nothing is removed; the library shows offline. Plug it back: it is scanned again on its own.
- **AC-7** (FR-17). With watching on, copy a film into a library: within a few seconds it appears and a "Folder change: 1 added" toast (and a Windows notification) is shown.
- **AC-8** (FR-16). Press Rescan all while the watcher's scan is running: the manual scan runs after it and reports its own numbers; no scan writes alongside another.

## Principles

- Respects all principles. P4: the scan walks the disk with no transaction open and on its own connection. P5: unreadable paths never count as gone.

## Open questions

None open. Decided while building: titles group by name only (D-003).

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08: Hardened (`dd4006d`): unreadable folders and a drive pulled mid-scan no longer delete rows; titles made of release-tag words survive; generic extras folders are categories at the top level; folders with the System attribute are scanned; startup ran one scan instead of two; the job queue replaced "already running" errors.
- 2026-10-08: Spec written from the code. Done.
