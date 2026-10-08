# Vortex

A local media library for Windows with exact resume tracking and a bring-your-own-links torrent client. Tauri 2 + Rust (`src-tauri/src`), Vue 3 + TypeScript + Tailwind 4 + shadcn-vue (`src`), SQLite via rusqlite. Start with `docs/FEATURES.md`.

## Spec-driven development

Vortex is built spec-first (`docs/spec-driven-development.md`). The product owner approves every spec before code is written, unless they have said otherwise in the conversation.

- **A feature or a change in behaviour starts as a spec** in `specs/NNN-name/spec.md` (from `specs/_template/`). Use `/spec` (in other agents, follow `.claude/skills/spec/SKILL.md`; likewise `spec-plan` and `spec-build`). Write the spec, then **stop and ask for approval**.
- **After approval**, write `plan.md` and `tasks.md` (`/spec-plan NNN`), then build from the task list (`/spec-build NNN`).
- **If the work needs something the spec doesn't say**, stop and propose an amendment.
- **Close every spec**: verify each acceptance criterion in the running app, update `docs/FEATURES.md` (and its Known gaps), the README if the feature list changed, `docs/roadmap.md`, `docs/decisions.md` for lasting decisions, and `specs/README.md`; mark the spec Done with the commit.
- **No spec needed** for bug fixes that restore documented behaviour, copy or styling tweaks, refactors with no behaviour change, dependency bumps and docs-only changes. Still fix anything in `FEATURES.md` they make wrong.

## Rules that always apply (`docs/principles.md`)

- **Never a source.** No content search, index or catalogue, ever (P2, D-004).
- **The user's files are the user's.** Write to them only on explicit request, previewed and undoable; delete only to the Recycle Bin after a confirm (P3).
- **The window never freezes.** File checks, process launches, network and drive enumeration run on blocking threads and outside the DB lock; long jobs open their own connection (P4, D-010).
- **Progress survives** renames, moves, folder switches, unplugged drives, backup and restore (P5).
- **Warn, don't block** on privacy (P6). **Secrets** (TMDb key, proxy URL) never reach the UI or the log (P8).
- **One job at a time** per kind (scan, poster fetch, duration probe) through `jobs::Job`; renames, undo and restore run under `jobs::exclusive`.
- **Schema** changes are ignored `ALTER TABLE`s in `db::open`, additive, with a backfill where needed.
- **UI**: shadcn-vue components in `src/components/ui`, works in light and dark, at the 1100 × 720 default window. Every view exposes `reload()` and is refreshed through `refreshAll` in `App.vue`.

## Commands

```bash
pnpm tauri dev                     # dev app (Vite on 3420). Never run while a release build is going.
cd src-tauri && cargo test -j 1    # backend tests (44). Never alongside another compile; -j 2 has crashed rustc when memory was short.
pnpm build                         # vue-tsc + vite build
CARGO_BUILD_JOBS=2 pnpm tauri build   # installers in src-tauri/target/release/bundle/{msi,nsis}; check timestamps, pnpm exits 0 even if rustc failed
```

Only one compile at a time on the build machine (release builds use most of its memory). Installers are handed over as paths; nothing is published to GitHub (D-011).

## Where things are

| Need | Look in |
| --- | --- |
| What the app does, every rule, as built | `docs/FEATURES.md` |
| Why something is the way it is | `docs/decisions.md`, `docs/principles.md` |
| What's next | `docs/roadmap.md`, `specs/README.md` |
| Scanning, parsing, grouping | `src-tauri/src/scanner.rs`, `parser.rs`, `watcher.rs`, `jobs.rs` |
| Playback tracking | `src-tauri/src/player.rs` |
| TMDb | `src-tauri/src/tmdb.rs` |
| Rename | `src-tauri/src/rename.rs` |
| Torrents | `src-tauri/src/torrent.rs`, `deeplink.rs` |
| Database and queries | `src-tauri/src/db.rs` |
| Commands and events | `src-tauri/src/commands.rs`, `lib.rs`, `src/lib/api.ts` |
| UI shell and views | `src/App.vue`, `src/components/*.vue` |
