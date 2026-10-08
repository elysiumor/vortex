# 007. File durations

| | |
| --- | --- |
| **Status** | Done (2026-09-19, `9ab6f90`; hidden console 2026-10-08, `dd4006d`) — written from the code on 2026-10-08 |
| **Area** | Library · Settings |
| **Roadmap** | Done: File durations |
| **Design doc** | `docs/FEATURES.md` §6 Durations |

## Problem

Progress bars, "minutes left", "90 % means watched" and the hours statistics all need each file's running time. Players report it only once a file plays. Reading it up front for thousands of files must not need the user to install anything for the common formats, must not slow the app, and must not flash a console window per file.

## Goals

- Every MKV, WebM, MP4, M4V and MOV file gets its duration without any extra install.
- Other formats get it when ffprobe is present, found automatically or pointed to.
- The job runs in the background after scans, never twice at once, and reports progress.

## Non-goals

- Reading codecs, resolution or audio tracks.
- Bundling FFmpeg.

## Where it lives

| Place | What |
| --- | --- |
| Background | After every scan that changed something, and at startup when the startup scan is off |
| Settings → File durations | ffprobe path (auto-detected hint), Browse, Read missing, result line |

## Requirements

- **FR-1.** After a scan, files without a duration must be read: MKV/WebM and MP4-family natively; everything else, and native failures, through ffprobe when one is configured or found.
- **FR-2.** ffprobe must be found automatically on PATH, in the usual install folders, scoop and WinGet, without the user doing anything; the user must also be able to point to one.
- **FR-3.** A file that could not be read must not be tried again on every scan.
- **FR-4.** No console window may appear while files are read.
- **FR-5.** Progress must be reported while the job runs and a summary at the end; "Read missing" must start the job by hand and show it running.
- **FR-6.** A duration learned later from the player must fill a missing one.

## Rules and edge cases

- Durations are whole seconds; zero or negative results count as unknown.
- The job selects only files never checked; a failure marks the file checked for good (Known gaps: no retry).
- Files on offline drives are skipped without being marked.
- Progress every 10 files; the "done" event also fires immediately when nothing is missing.

## Acceptance criteria

- **AC-1** (FR-1). After adding a folder of MKV and MP4 files, each episode row shows a duration and the Home resume bar has a length.
- **AC-2** (FR-2). With FFmpeg installed via WinGet and no path set, Settings shows "Auto-detected: …ffprobe.exe" and AVI files get durations.
- **AC-3** (FR-4). Adding a folder with hundreds of AVI files in the installed build flashes no console windows.
- **AC-4** (FR-5). "Read missing" shows "Reading…" then "Read N of M files" (or "All files already have a duration").

## Principles

- Respects all principles. P4: detection and probing run on their own thread.

## Open questions

None open.

## Changelog

- 2026-09-19: Built (`9ab6f90`), no spec.
- 2026-10-08 (`dd4006d`): `CREATE_NO_WINDOW` for ffprobe and `where`; detection cached per run.
- 2026-10-08: Spec written from the code. Done.
