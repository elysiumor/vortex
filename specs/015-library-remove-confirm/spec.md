# 015. Confirm before removing a library folder

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `5ba2545`, shipped in 1.1.11); installed-build check by the product owner pending |
| **Area** | Settings · Library |
| **Roadmap** | Now → Done: Confirm before removing a library folder |
| **Design doc** | `docs/FEATURES.md` §2 Libraries |

## Problem

Removing a library folder in Settings happened on one click. Everything under it left the library at once: titles, episodes, watch positions and history. There was no "are you sure", and the product owner reported that in the installed build the Remove control was not even visible (it was a ghost button, invisible until hovered in some themes).

## Goals

- Removing a folder always asks first and says what will go.
- The Remove control is plainly visible on every library row.

## Non-goals

- Deleting any file from disk; removal only changes the library.
- Undoing a removal (the TMDb memory already brings matches back when the folder is re-added).

## Where it lives

| Place | What |
| --- | --- |
| Settings → Libraries | File count and an outlined Remove on each row; a confirm dialog |

## Requirements

- **FR-1.** Each library row must show how many video files are filed under it and an outlined Remove button, visible in light and dark, online or offline.
- **FR-2.** Pressing it must open a confirm dialog naming the folder and saying that its N files leave Vortex with their watch progress and history, that nothing on disk is deleted, and that adding the folder again brings the titles back with their TMDb data.
- **FR-3.** Confirming must remove the library, toast "Removed <path> from the library" and refresh every view; cancelling must change nothing.
- **FR-4.** It must work for an offline folder (the count comes from the database).

## Rules and edge cases

- The file count is `COUNT(*)` of episodes in the library, extras included.
- The dialog's target is held outside the dialog's own state so closing it cannot cancel the action (the pattern used by the other confirm dialogs).

## Acceptance criteria

- **AC-1** (FR-1). In the installed build, every library row shows "N files" and Remove, light and dark.
- **AC-2** (FR-2, FR-3). Remove on a folder with 40 files: the dialog names the path and says "Its 40 files leave Vortex…"; Cancel leaves it; Remove folder removes it, toasts, and the Movies page updates.
- **AC-3** (FR-4). Remove an offline library: the dialog opens with its count and removal works.

## Principles

- Respects all principles. P3.

## Open questions

1. Should removal also offer "forget TMDb data for these titles"? Not done; the memory is what makes re-adding cheap. Left open for a later spec.

## Changelog

- 2026-10-08: Drafted from the product owner's request ("keep the option with a prompt saying all the files will be removed, then confirm"); approved under the standing instruction.
- 2026-10-08: Built in the parallel session (`5ba2545`) alongside spec 014; shipped as 1.1.11. Done.
