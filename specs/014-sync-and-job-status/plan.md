# 014. Sync button, status pill, title page successor: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

One event channel for all background work: `jobs::report(app, job, label, done, total, detail)` emits `job-progress {job, label, done, total, detail, running: true}` (thinned per job to 150 ms, first and last always sent) and `jobs::finished(job)` sends `running: false`. Each job calls it from its loop: the scanner per 100 files walked and per 50 rows written (each library an equal slice), the poster fetch per title, the probe per file, the rename per title, `apply_remembered` per candidate. `App.vue` keeps one entry per running job and renders the highest-priority one in a pill, with a Sync button that calls `scan_libraries`. The title page sequences its loads and, on a null item, asks `item_for_paths` with the file paths it last showed.

Set aside: a persistent job log; per-job cancel beyond posters.

## Data

| Table / key / file | Change |
| --- | --- |
| none | — |

## Backend (`src-tauri/src`)

| Module | Change |
| --- | --- |
| `jobs.rs` | `report`, `finished`, thinning state; `Job::release()` (let go without serving a queued pass, used when a fetch is cancelled). |
| `scanner.rs` | progress calls in the walk and the write pass (needs the app handle; library index/count for the slice). |
| `tmdb.rs` | `CANCEL` flag, `cancel_fetch`, `fetch_pass -> bool`, `report` per title, `finished("posters")`; `apply_remembered` reports `memory`. |
| `probe.rs` | `report("durations", file name)`, `finished`. |
| `rename.rs` | `report("rename", title)` per planned title, `finished`. |
| `db.rs` | `item_for_path`. |
| `commands.rs`, `lib.rs` | `cancel_posters`, `item_for_paths`. |

## Events and commands

- `job-progress {job: scan|posters|durations|rename|memory, label, done, total, detail, running}`. UI: `api.onJobProgress`, `api.cancelPosters`, `api.itemForPaths`, `api.scanLibraries` (Sync).

## UI (`src`)

- `App.vue`: `jobs` map, `JOB_ORDER`, `activeJob`/`otherJobs`/`jobPct`/`jobTitle`, the pill (`lg:flex`), `syncLibrary` (also wired to Ctrl+K), `@replaced` on `SeriesView`.
- `SeriesView.vue`: `loadSeq` guard, successor lookup, `replaced` / `back` emits.
- `TmdbView.vue`: Stop button, percent in the progress line (spec 017).

## Cross-feature effects

- Rename (011), scan (001), posters (004), durations (007), memory (012) all report. Settings' own "Reading…"/"Fetching…" states remain for their cards.

## Risks

- The pill is hidden below `lg`; a narrow window shows only the spinning Sync icon.
- Progress from a job that panics: `release_on_panic` frees the job, but its last `running: false` may not be sent; the pill entry then lingers until the next run of that job.

## Design doc updates

- `docs/FEATURES.md` §3, §6–§8, §12, §23, §24, §25 (done in `5ba2545`).
