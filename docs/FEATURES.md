# Vortex — feature specification as built

Version 1.1.11, 2026-10-08. This describes what the shipped code does, feature by feature, as a reference for future work. Every statement is taken from the source; nothing here is planned behaviour. Open problems and requests are collected at the end.

File references are to `src-tauri/src/*.rs` (backend) and `src/**` (UI).

---

## 1. Overview

- **What it is:** a Windows desktop media library (Tauri 2 + Rust backend, Vue 3 + TypeScript + Tailwind v4 + shadcn-vue front end, SQLite via rusqlite in WAL mode) with an embedded BitTorrent client (librqbit 9).
- **Window:** titled "Vortex", 1100 × 720 by default (`tauri.conf.json`). Single window, `main`.
- **Identifier:** `com.admin.vortex`. **Dev server port:** 3420. **Package manager:** pnpm only (`pnpm tauri dev`, `pnpm tauri build`; lockfile `pnpm-lock.yaml`).
- **Plugins:** single-instance (with deep-link), deep-link, opener, dialog, notification. Asset protocol scope: `$APPDATA/posters/*` (posters are served to the WebView from there, cache-busted by `?v=<tmdb_id>`).
- **Content Security Policy** (`tauri.conf.json` `csp`, `devCsp`): scripts and styles from the app only (inline styles allowed for Vue bindings), images from the app, the asset protocol, `image.tmdb.org`, `data:` and `blob:`, connections to the IPC bridge only (plus the Vite dev server in dev). Violations are forwarded to `vortex.log` as errors (`diag.ts`).

### Data folder `%APPDATA%\com.admin.vortex\`

| Entry | Purpose |
|---|---|
| `vortex.db` (+ `-wal`, `-shm`) | The database (schema in §27). |
| `posters\` | `{kind}-{tmdb_id}.jpg` posters and `{kind}-{tmdb_id}-backdrop.jpg` backdrops, at the sizes chosen on the TMDb page (w342 / w1280 by default). |
| `torrents\` | librqbit session persistence (JSON), `dht.json`, `dests.json` (per-torrent destinations). |
| `rename-history.json` | Last 20 applied rename batches, for Undo. |
| `vortex.log`, `vortex.log.1`, `vortex.log.0` | This run, previous run, rolled-over first half of a long run. |
| `backup-snapshot.db`, `restore-incoming.db` | Temporary files during backup / restore. |
| `vortex.db.before-restore`, `.before-restore.1` | The database set aside by the last two restores. |

---

## 2. Libraries

Settings → Libraries (`SettingsView.vue`, `db.rs` libraries section).

- **Add folder:** folder picker → `add_library` → list reload → immediate scan (`scan()`), scan summary toast.
- **Add whole drive:** dialog listing drives (`list_drives`: mount point, label, used/total bar, removable). A drive already added shows "Added". The system drive (`C:`) first gets a confirm dialog (slow, clutter) with "Pick a folder instead" / "Add whole drive". After adding: toast "Added X. Scanning in the background…" then a scan.
- **Overlap rules** (`db::add_library_from_ui`): paths compared case-insensitively, `/`→`\`, no trailing separator. The same folder returns the existing library. A folder *inside* an existing library is refused ("… is already part of the library …"). Existing libraries *inside* the new folder are folded into it: their episodes are re-pointed to the new library and the old rows deleted, progress intact.
- **Name** = last path component (or the whole path if none). **Available** = `Path::is_dir()` at the time asked; probed off the main thread (`fill_availability`).
- **Remove:** an outlined "Remove" button per row, next to the row's file count (`Library.file_count`). A confirmation dialog names the folder and its file count and says that nothing on disk is deleted. `DELETE FROM libraries` cascades to its episodes, which cascades to watch progress and history; media items left without episodes are pruned. The TMDb memory (§7) survives, so adding the folder again brings the titles back matched.
- **Folders to skip:** textarea, one name per line, saved to setting `ignore_dirs` on change; compared lowercase against each folder name below a library root. Always skipped (`scanner::DEFAULT_IGNORED_DIRS`): windows, program files, program files (x86), programdata, appdata, $recycle.bin, system volume information, recovery, perflogs, node_modules, .git, $windows.~bt, windows.old, msocache, steamapps, cache, .cache, temp, tmp, .incomplete — plus hidden folders.
- **Rescan all:** disabled while scanning or with no libraries; shows "N files · N added · N removed · N offline" from the last manual scan.
- The Settings list shows a green/red dot for available/offline per library.

---

## 3. Scanning

`scanner.rs`, `jobs.rs`.

### Triggers and reasons

| Reason | Trigger |
|---|---|
| `startup` | App start, if `rescan_on_startup` (default on). Otherwise only the duration probe runs. |
| `manual` | The top-bar **Sync** button, Settings "Rescan all", "Add folder/drive", Ctrl+K "Rescan libraries" (`scan_now`: waits for a running scan rather than folding into it, returns this scan's own stats). Sync refuses with a toast while a scan is already running. |
| `tray` | Tray menu "Rescan libraries". |
| `watch` | Folder watcher burst (§5). |
| `drive` | A library that was offline came back (§5). |
| `torrent` | A download finished and moved into place (§18). |

### Job model (`jobs::Job`) and progress

`jobs::report(job, label, done, total, detail)` emits `job-progress` for every background job on one channel, thinned to one event per 150 ms per job except the first and last step; `jobs::finished(job)` sends `running: false`. Jobs: `scan`, `posters`, `durations`, `rename`, `memory` (TMDb data restored after a scan). `Job::release()` lets a stopped job go without serving a pass queued meanwhile.

Three jobs: scan, poster fetch, duration probe. Each runs one pass at a time. A request made while a pass is running sets an `again` flag and is served by one more pass when the current one ends (never refused, never queued more than once). Callers that must *own* the job (Settings rescan, rename apply/undo, backup restore) wait for it; a running owner yields to waiters between passes so a stream of folder changes cannot starve them. `jobs::exclusive(f)` runs `f` with no scan running or starting and then runs any scan deferred meanwhile with reason `watch`. A panic releases the job.

### What a scan does (`scan_all` → `scan_library` per available library)

1. Offline libraries are listed in `libraries_skipped`. One library failing does not stop the others.
2. **Pass 1 — walk** (no transaction): `WalkDir`, symlinks not followed; directories skipped when their lowercase name is in the ignore list or they are hidden (Windows Hidden attribute; dot-prefix elsewhere). Files kept when they are video (§4 extensions), ≥ 20 MB, and their name does not contain "sample". Unreadable entries are remembered. For each file: size, mtime, `parser::parse`, sidecar subtitles (same folder, name starts with the video's stem, extensions srt/ass/ssa/sub/vtt/idx/sup, joined by `|`).
3. If the library root is no longer a directory after the walk, nothing is written ("went offline during the scan").
4. **Pass 2 — write** inside an `IMMEDIATE` transaction: series episodes (kind Series, episode number, not extra) are upserted first; then the rest, plain movies before bonus material.
5. **Row matching:** rows of this library whose path was not seen are gone — unless the path is under an unreadable entry (left alone). A gone row whose `(size, modified)` matches a newly inserted row is a rename/move: `move_progress` carries watch progress, history, `added_at`, still, rating, duration, title, overview and air date to the new row and the stat counts `renamed` instead of `added`. Other gone rows count as `removed`. Gone rows are deleted.
6. `prune_empty_items`: media items with no episodes are deleted.
7. Stats: `libraries_scanned`, `libraries_skipped`, `files_seen`, `added`, `removed`, `renamed`, `extras`, `added_titles` (first six labels, "Title SxxEyy" or title).

### After each scan (`scan_once`)

- Emits `scan-done {reason, stats}` (or `scan-error`).
- Windows toast "N new in your library" with the added titles, when reason is `watch`/`drive`/`torrent`, something was added, and `notify_new` is on.
- If the scan changed anything (added/removed/renamed) **or** reason is `startup`: `tmdb::apply_remembered` (emits `library-changed` if it applied anything), `probe::probe_missing`, and `tmdb::fetch_missing(false)` if `tmdb_connected` and `tmdb_auto_match` (default on).
- **Progress** (`job-progress`, job `scan`): "Scanning library · <name> · N files" while a library is walked (every 100 files, bar indeterminate), then "Updating library" with rows written / rows to write (every 50). Each library is an equal slice of the percentage.
- Tray menu rebuilt.
- Scans run on their own DB connection so the shared one is never held for minutes.

---

## 4. Parsing and grouping (`parser.rs`, `scanner.rs`)

- **Video extensions:** mkv, mp4, avi, mov, wmv, flv, webm, m4v, ts, mpg, mpeg, m2ts, vob, 3gp, ogv.
- **Episode patterns, in order:** `SxxEyy` (with `E?yy` / `-yy` continuations as ranges), `1x02`, `Season N Episode M`; then, with the season from the parent folder, `E05` / `Ep 05` / `Episode 5` or ` - 05`; then "ShowFolder\Show - 05.mkv" (season from an `S2`-style marker in the stem, else 1).
- **Season folders:** names that are or contain `Season N`, `Series N`, `S01` (so "Show (2019) Season 1 S01 + Extras (…)" counts).
- **Year:** 19xx/20xx delimited by separators or brackets. A bracketed year wins; otherwise the last year not at the start of the name; a name that starts with a year keeps it as title ("2012 (2009)", "1917"); the end of a range ("1999-2007") is not a year.
- **Title cleaning:** release-group `[…]` prefix dropped, `.`/`_` → space, cut at the first release tag that has some title before it (a tag at the very start is the title: "Dual", "Internal Affairs", "Season of the Witch"). Text before a found year is cut only at unambiguous *tech* tags, so "Charlotte's Web (2006)" and "A Complete Unknown (2024)" survive. Trailing `-–—([+` and whitespace trimmed.
- **Series title:** the show folder (grandparent when the parent is a season folder) is preferred when it agrees with the file name (one key is a prefix of the other), so every season groups under one title; else the file name's title; else the folder.
- **Movie title:** the stem before the year; if empty, the parent folder's title ("The Matrix (1999)\1999.mkv").
- **Extras (`in_extras_dir`):** specific names (featurettes, behind the scenes, deleted scenes, bloopers, making of, outtakes, gag reel, bonus/special features) count at any depth; generic names (extras, bonus, specials, trailers, interviews, shorts, other(s), scenes, promos, teasers, webisodes) count only *inside* a title's folder — directly under a library root they are a category.
- **Grouping key:** `sort_key` = lowercase alphanumerics of the title. Media items are unique per `(kind, sort_key)`; a later file with the same key joins the item, `year` and `category` keep their first non-null value. Consequence: remakes with the same title merge (§29).
- **Category:** the first folder under the library root, unless that folder is the title's own (its key starts with the title key) or a season folder, in which case the library name. Editable per title afterwards.
- **Bonus attachment order** for a file that is extra, a series file without episode number, or inside a season folder: (1) the nearest ancestor folder that already holds real episodes of a series → that series, labelled relative to the show's top folder ("Season 1 › Featurettes › Behind The Scenes"); (2) otherwise the owner folder (first ancestor that is neither extras nor season folder, else the root): a movie whose main file sits in it → that movie; the root with exactly one series under it → that series; (3) else a series named after the owner folder (merges with the real series when its episodes arrive).

---

## 5. Folder watcher (`watcher.rs`)

- One `notify` debouncer (3 s) over every available library root, recursive. Every 20 s the loop re-reads libraries, the ignore list and availability: new/reconnected roots are watched, offline ones unwatched (logged "library offline" / "library back online"); a library coming back triggers a `drive` scan.
- An event is relevant when the path below the root crosses no ignored/`.incomplete` folder and is a video, a directory, or no longer exists. A burst is collapsed into one `watch` scan of all libraries (the first path and the extra count are logged).
- Both reactions require the `watch_folders` setting (default on).

---

## 6. Durations (`probe.rs`)

- Sources: `.mkv`/`.webm` via the `matroska` crate; `.mp4`/`.m4v`/`.mov` via the `mp4` crate; everything else (and native failures) via `ffprobe` when available: `ffprobe_path` setting, else detection cached per run (`where ffprobe`, `C:\ffmpeg\bin`, Program Files, chocolatey, scoop shims, WinGet packages). Child processes use `CREATE_NO_WINDOW`.
- One pass selects `duration_secs IS NULL AND duration_checked = 0`; success stores the duration, failure sets `duration_checked = 1`. Checked files are never selected again — the Settings "Read missing" button runs the same query, so a file that failed once is not retried (§29).
- Events: `durations-progress {done,total,found}` every 10 files, `durations-done` at the end (also emitted immediately with total 0 when nothing is missing); `job-progress` (job `durations`, "Reading durations", file name) per file. Settings shows "Reading…" while running and "All files already have a duration" / "Read N of M files" after.
- A length reported by the player during playback is stored when the episode had none.

---

## 7. TMDb integration (`tmdb.rs`, `TmdbView.vue`)

Everything TMDb lives on its own **TMDb page** (More menu, Ctrl+K, and a link card in Settings): Connection, Matching, API options, Posters & details, Unmatched titles, File names.

### Key and connection
- TMDb page → Connection: paste a v3 key or v4 read token. `connect_tmdb` verifies it with a search ("Inception", 2010) before storing `tmdb_key` and `tmdb_connected=1`. v4 tokens (start with `eyJ`) go in the `Authorization: Bearer` header, v3 keys as `api_key` query param.
- The key never reaches the UI: `get_settings` replaces it with `"set"`, `get_tmdb_status` returns `••••` + last 4 chars (null when not connected). "Remove" (confirm dialog) blanks the key and sets `tmdb_connected=0`; posters stay.
- All requests use the preferences below (`tmdb::Prefs`, loaded from the `tmdb_*` settings at start and whenever one changes); user agent `vortex-media-manager`.

### Matching and API options (TMDb page)
| Setting | Default | Effect |
|---|---|---|
| `tmdb_auto_match` | on | Fetch posters for unmatched titles after every scan that changed something. |
| `tmdb_adult` | off | `include_adult=true` on automatic searches. Fix match **always** includes adult titles (TMDb hides them otherwise; "Monella" never matched "Frivolous Lola" because of this). |
| `tmdb_year_fallback` | on | When nothing matches the file's year, search again without it. |
| `tmdb_language` | `en-US` | `language` on every call (titles, overviews, genres, episode names). Changing it clears the genre-name cache; already fetched text stays until Refresh. |
| `tmdb_region` | empty | `region` on movie searches (which country's release dates the year filter uses). |
| `tmdb_year_mode` | `any` | Movie year filter: `year` (any release) or `primary_release_year` (original release). Series always use `first_air_date_year`. |
| `tmdb_cert_country` | `US` | Whose certification is shown; another country's as fallback. |
| `tmdb_poster_size`, `tmdb_backdrop_size`, `tmdb_still_size`, `tmdb_profile_size` | w342, w1280, w300, w185 | Image sizes from TMDb's configuration (posters and backdrops are downloaded at that size from then on; stills and cast photos are remote URLs). |
| `tmdb_delay_ms` | 120 | Pause between calls in a long run (also between season fetches). Max 10 000. |
| `tmdb_timeout_secs` | 20 | Per-request timeout, 5–120. |
| `tmdb_cache_days` | 90 | Age after which details are refetched on their own; 0 keeps them for ever. |

Details always append `credits,videos,external_ids,release_dates` (movies) / `content_ratings` (series); keywords, reviews, recommendations and watch providers are never requested.

### Automatic matching (`fetch_missing` / `fetch_pass`)
- Started after a scan that changed something (if connected and auto-match is on), by "Fetch missing posters", or "Retry unmatched" (force). Needs a key, else error "Add your TMDB API key in Settings first". **Stop** (TMDb page button, or the X on the status pill) sets a cancel flag: the pass ends at the next title, `posters-done` carries `current: "stopped by you"`, and no pass queued meanwhile runs; titles not reached stay unchecked for the next fetch.
- Candidates: titles with no `poster_path` and `poster_checked = 0`; force also includes checked titles that have **no** `tmdb_id`. A matched title is never searched again (a hand-picked match with no poster would be overwritten by the top search result).
- Per title: with `tmdb_id` → `retry_poster` (download by the remembered/cached poster path, mark checked); without → `best_match` = search with the library year, else (with `tmdb_year_fallback`) without year, first result → `apply_match(manual=false)`; no result → `poster_checked=1` and remembered as not found. An auth/network error stops the whole run (`posters-done` with `current` = the error). `tmdb_delay_ms` sleep between titles.
- Events: `posters-progress {done,total,matched,current:title}` per title, `posters-done` at the end, and `job-progress` (job `posters`, "Fetching posters", current title). The TMDb page shows a progress bar + "done / total · % · matched · current" and toasts "Posters: N of M matched" or "Poster fetch stopped: …"; the top-bar pill shows the same everywhere.

### `apply_match` (automatic or Fix match)
1. Poster downloaded to `posters\{kind}-{tmdb_id}.jpg` unless already there (non-empty). A failed download keeps the match and clears `poster_checked` so the next fetch retries only the image.
2. `set_tmdb`: tmdb_id, poster path, overview, rating; `poster_checked=1`.
3. Genre names resolved through the genre list (one network call per kind per run).
4. `fetch_episode_titles` (cache first), `get_details(refresh=false)` (network if not cached — one extra API call per newly matched title), `apply_cached_details` (rating, genres, TMDb collection → §14).
5. `remember_match` into the memory (manual flag).

### Fix match (title page)
Search dialog prefilled with the title; `search_tmdb(kind, query, year=null)` with `include_adult=true`; results show poster, year, rating, overview; picking one calls `apply_match(manual=true)`. A manual match is never replaced by an automatic one (memory `manual` rule), and changing the match drops the old poster path in favour of the new tmdb id's file.

### Details (`get_details`)
- Cached per `(kind, tmdb_id)` in `tmdb_cache`; served from cache unless `refresh` or older than `tmdb_cache_days` (90; 0 = never). With no key, a stale cache is served rather than an error.
- Fetched with `append_to_response=credits,videos,external_ids,release_dates` (movie) / `content_ratings` (tv). Normalised fields: title, original title, tagline, overview, genres, runtime (movie runtime / first episode run time), rating (1 dp), vote count, release/first-air date, last air date, status, certification (`tmdb_cert_country` first, else any), language, backdrop (downloaded once to `-backdrop.jpg` at `tmdb_backdrop_size`), cast (sorted by order, max 24, photo at `tmdb_profile_size`, remote), directors, writers (department Writing), creators, companies (networks then production companies, max 4), YouTube trailer (official preferred), IMDb id, homepage, TMDb URL, seasons (number > 0: name, episode count, air date, overview, poster at `tmdb_poster_size`, remote), fetched date, collection name.
- A fetch also writes rating/genres/collection onto the title. Refresh (title page button) deletes the backdrop, refetches, refetches episode titles (forced) and toasts "Details refreshed".

### Episode titles (`fetch_episode_titles`)
Per season present in the library: season JSON cached in `tmdb_seasons`; refetched when the cache lacks an episode the library has (a show still airing) or when forced. Stores per episode: name, overview, air date, still (remote URL at `tmdb_still_size`), rating. `tmdb_delay_ms` sleep per fetched season. 404 = season numbering differs → skipped.

### Memory across folder switches (`tmdb_memory`, `apply_remembered`)
- Keyed by `(kind, sort_key)`; holds tmdb_id (NULL = searched, nothing found), manual flag, poster path, overview, rating, genres, library year. Written on every match and not-found; a manual entry is only replaced by another manual pick or the same id. Renames re-key it (`rekey_memory`).
- After a scan, titles with `tmdb_id IS NULL AND poster_checked = 0` are looked up: a remembered id within ±1 year of the library year is applied from disk (poster re-downloaded only if the file is missing), cached details and episode titles applied, no API call; a remembered not-found marks the title checked. Reported as `job-progress` job `memory` ("Restoring TMDb data"). Logged as "TMDb data reused from earlier, no API calls".
- `migrate_image_names` at startup and after restore renames legacy `{item_id}.jpg` posters to tmdb-id names and drops legacy backdrops.
- The TMDb page shows "Saved on this PC: N matched titles, N detail pages, N images (N MB)" and an **Unmatched titles** list (every title without a `tmdb_id`, first 150, each opening its page for Fix match).

---

## 8. Rename to TMDb names (`rename.rs`, `RenameDialog.vue`)

- **Entry points:** "Rename files" on a matched title's page (that title only); TMDb page → File names "Review renames…" (every movie and series) and "Undo last rename".
- **Preview** (`rename_preview`): a plan per title: `target` ("Inception (2010)"), `moves` (files first, then folders), `skipped` reason, `notes`. Dialog lists each title with a checkbox (all ticked), its moves as "old → new" (folders, then up to 6 files, "Show all N"), notes in warning colour, a collapsible "Skipped: N" list, and "N already named after TMDb". Nothing on disk changes here.
- **Names:** movies `Title (Year).ext`; episodes `Show (Year) - SxxEyy - Episode Title.ext`, double episodes `S01E01-E02` without a title; episode title cut to 80 chars; `safe_name` turns `: ` into ` - `, drops `<>"/\|?*` and control chars, collapses spaces, trims trailing dots/spaces, appends `_` to reserved names (CON, PRN, AUX, NUL, COMn, LPTn). TMDb's spelling becomes the library title when it keys the same as the new file name.
- **Several main files in one folder:** `- pt1/pt2` when names carry cd/disc/part markers; else `- 1080p` etc. when every file has a distinct quality tag; else `- 1`, `- 2` by sorted file name.
- **Sidecars:** non-video files in the same folder named after the video's stem followed by a non-alphanumeric char (`Movie.en.srt`, `Movie_eng.srt`, `Movie.nfo`) move with it; a sidecar that also matches a *longer* video stem in the folder belongs to that one ("It" vs "It.Follows").
- **Remake guard (movies):** a file whose own name (or parent folder) carries a year more than one off TMDb's is left alone with a note, and then no folder is renamed. If no file matches the year the title is skipped ("No file here is from YYYY").
- **Series:** all or nothing. Every main episode must place on TMDb (`place`): numbers TMDb has are kept; a number past its season's count is counted straight through TMDb's seasons (anime) unless the name spelled `SxxEyy` out, the season is 0, or — inside a season folder — the count lands in another season. Any file that fails, or two files for one episode that only a counter could tell apart, skips the whole series with the reason. Episode names come from the library, then from TMDb's season list for seasons not yet cached.
- **Folders:** a season folder holding one season becomes `Season 01` (`Specials` for 0) only when it sits inside the show's own folder, is the title's alone, and no other folder would get the same name. The title's own folder (show folder, or the movie's folder when all its files share one folder and no remake note) is renamed to the base name only when its name reads as the title, it is not a library root and holds no library, no other title has files under it, and the target does not already exist.
- **Safety checks:** `reads_back` re-parses the final path — it must come back as the same kind, the same year (movies), a non-empty title, and must not key onto another title already in the library ("would be filed together with …"). Duplicate targets, existing targets, unavailable files and final paths over 250 characters all skip the title.
- **Apply** (`rename_apply`, under `jobs::exclusive`): each chosen title is planned again against the disk, renamed on disk in order (a failing step puts every earlier step of that title back), then the library is updated in one transaction: episode paths and file names, folder prefixes, subtitle paths, then `resync_title` (title + sort_key from the new file name, memory re-keyed). If the library update fails the files are put back. `job-progress` (job `rename`, "Renaming files", title) is reported per title. Report: renamed / unchanged / failed `[title, reason]`; the UI toasts "Renamed N titles" with an Undo action and one error toast per failure. The batch of moves is appended to `rename-history.json` (20 kept).
- **Undo** (`rename_undo`): reverses the most recent batch (refuses if any original path exists again), updates the library the same way, toasts "Rename undone (N changes reversed)".
- After a rename the TMDb page emits `changed` so every view reloads.

---

## 9. Playback and tracking (`player.rs`)

- **Detection:** PotPlayer (`PotPlayerMini64/Mini.exe`), VLC, MPC-HC (incl. K-Lite), mpv, plus `%USERPROFILE%\scoop\shims`. On first run the first detected player is saved (`player_kind`, `player_path`). Settings shows detected players as buttons, a path field, Browse, and a kind select (potplayer, vlc, mpc-hc, mpv, custom = "Other (no resume)"); a warning when none is set.
- **Launch:** file path first; subtitle = first existing sidecar, preferring `.en.`/`.eng.`/`english` (`--sub-file=` for VLC/mpv, `/sub` for MPC-HC; PotPlayer/MPC-HC also load same-name sidecars themselves). Start position = 0 if completed else the saved position: PotPlayer `/seek=hh:mm:ss`, VLC `--start-time=`, MPC-HC `/startpos hh:mm:ss`, mpv `--start=`. VLC also gets `--extraintf=http --http-host=127.0.0.1 --http-port=<free> --http-password=<nonce>`; mpv gets `--input-ipc-server=\\.\pipe\<nonce>`.
- No configured/existing player → the file opens with the Windows default app, untracked; the UI warns "Opened with the default app…". The file must exist ("File is not available. Is the drive connected?").
- `touch_progress` on play: `last_watched = now`, `hidden = 0` (brings the title back to Continue watching).

### Tracker thread (every 2 s)
- Readings: PotPlayer via `WM_USER` messages on its window (status, time, length); VLC via `/requests/status.json` (state, time, length, playing file name); mpv via the IPC pipe (`time-pos`, `duration`). Unknown players: no link, clock only.
- Position written every 15 s while playing; a player-reported length is stored if the episode had none.
- **End detection:** the child exited and (for a per-launch link we heard from) that is the end; shared links (PotPlayer window, adopted VLC window, clock-only) keep tracking while any process of that exe name runs. A newer playback started by Vortex (`SESSION` counter) ends any shared-link tracker, and any per-launch tracker that never heard from its player.
- **VLC single instance:** the last VLC window Vortex could talk to is remembered; when a launch exits without ever answering, the tracker adopts that window if it reports our file. A VLC that reports a different file (`Elsewhere`) ends our tracking with the last position read.
- "Stopped" from the player counts as finished only if the furthest position reached ≥ 90 % of the duration (or the duration is unknown).
- **Result:** sessions shorter than 15 s with no exact reading are discarded. Position = furthest position if the player reached the end, else the last exact reading, else start + elapsed; clamped to the duration; `completed` when ≥ 90 %. `set_progress`, history row (source `player`, `exact` flag).
- **Autoplay:** with `autoplay_next` (default on) and completed, `next_episode` = the next non-extra episode in season/episode order whose file exists (series only). If the player was still open (ended in player) the next episode is started at once; otherwise the UI shows the "Up next" card: 10 s countdown, Play now, cancel. Toasts: "Playing next: …", "Marked as watched", or "Progress saved at h:mm:ss (estimated)".
- `playback-ended` event (§24) and a tray rebuild follow every session.
- **Streams** (`play_url_tracked`): same launch and tracker over a URL; the engine's callback receives position/duration/exact when the player closes.

| Player | Resume | Position | Auto-play next |
|---|---|---|---|
| PotPlayer | Yes | Exact | Same window |
| VLC | Yes | Exact | Same window |
| mpv | Yes | Exact | Countdown |
| MPC-HC | Yes | Estimated | Countdown |
| Other | No | Estimated | Countdown |

---

## 10. Watch progress and Home

- **Continue watching** (`db::continue_watching`): the most recently touched episode per title (ties broken by highest season/episode); rows hidden with X excluded; finished films excluded; a finished series episode is replaced by the next unwatched episode after it (if any). Limit 20 on Home, 3 in the tray.
- **Dismiss (X on a Home card):** `hidden = 1` for that episode; progress kept; playing it again un-hides. `hide_all_from_home` exists as a command (no UI).
- **Mark watched / unwatched:** per episode (toggle; unwatched deletes the progress row) and per title ("Mark all watched" inserts completed rows for every file incl. extras; "Mark all unwatched" deletes them). Manual watched marks add a history row with source `manual`.
- **Set paused time:** per episode, input accepts `23:14`, `1:05:00` or bare minutes; saved as not completed.
- **Reset progress** (episode details panel) deletes its row. **Reset all watch data** (Settings, confirm) deletes all progress and history.
- **Home** (`HomeView.vue`): loads continue items, all media, collections.
  - **Hero:** the first continue item ("Continue watching", with resume bar, minutes left, episode label) else a random unwatched matched title ("Featured"); the Shuffle button picks "Tonight's pick" (toast "Everything is watched. Impressive." when none unwatched). Backdrop from details (loaded per pick with a sequence guard), else blurred poster. On reload the same title is kept with fresh data; a finished/dismissed/removed pick is replaced.
  - **Rows:** Continue watching; Recently added (24 by `added_at`); Top rated (rating ≥ 7.5, only with ≥ 3); "More <genre>" (the genre with most watched titles, ≥ 3 unwatched in it); Movies (30, unwatched first then newest) with See all; Series (30) with See all; Collections in progress (12 non-empty, unfinished).
  - **Empty library:** a welcome panel with "Add a folder" → Settings.

---

## 11. Movies / Series pages (`LibraryView.vue`)

- Lists `list_media(kind)`; title filter box; Watched filter (All/Unwatched/Watched); Category select (only when > 1 category); Genre select; Tag select; Sort: A→Z, Newest year, Recently added, Recently watched, Highest rated, Largest (saved in localStorage `vortex-sort`).
- Card subtitle per sort: year always; + first two genres (A→Z), ★ rating, GB, "added …", "watched …". Badge: "N / M watched" (series) or Watched/Unwatched. Watched tick when all episodes done; progress bar for series.
- Play on a card: the partially watched episode, else the first unwatched, else the first.
- **Smart lists:** "Smart list" button saves the current filters plus optional min rating, untouched-for-days, from-year, with a name; stored as JSON in setting `smart_lists` (`id, name, kind, watched, category?, genre?, tag?, minYear?, maxYear?, minRating?, untouchedDays?`). They appear under More and open a LibraryView with chips showing the rules; the trash icon in the menu deletes one. `maxYear` is in the type but has no input.
- Skeleton grid while loading; "Nothing here yet" / "No matches" empty states.

---

## 12. Title page (`SeriesView.vue`)

- Loads `get_media_item`, `list_episodes`, `get_details(false)`, tags and collections. Loads are sequenced so an older load cannot overwrite a newer one. If the item no longer exists (a rescan after an external rename filed its files under a new title, or they were deleted), `item_for_paths` with the previously listed file paths finds the successor and the page switches to it (`replaced` event); with no successor it toasts "This title is no longer in the library" and goes back. The skeleton is only shown before the first load.
- **Hero:** backdrop (or blurred poster); Back; category button (edit in place with a datalist of known categories); Fix match; Refresh (matched only); Rename files (matched only); Mark all watched/unwatched. Poster (or initial). "SERIES/MOVIE · category", TMDb title (library title fallback), original title, rating badge + votes, year, certification, runtime (+ "/ ep"), seasons · episodes, status, language, genre chips, tagline, overview.
- **Primary button** label: "Resume SxxEyy at h:mm" / "Play SxxEyy" / "Play again from S01E01" / "Watch again" / "Play" / "No file"; disabled when the file is unavailable. Also Trailer (YouTube), IMDb, TMDb link, folder reveal (show folder, one level up from a season folder). "Not matched to TMDB yet…" hint when unmatched.
- **People:** Created by, Director, Writers (first 4), Network/Studio, Released/Aired → last air date.
- **Tags:** chips with remove; input with datalist of all tags (Enter or blur adds). **Collections:** chips with remove; "+ collection" dropdown of the rest; "New collection…" dialog creates and adds.
- **Cast:** 8 avatars (photo or initials) with character; "Show all N" up to 24.
- **Episodes:** grouped by season (TMDb season name or "Season N"; "Unsorted" for none) with "x of y on disk · year" and the season overview. Row: still with hover-play, progress bar and watched tick (series) or quality tag / ✓ / ▶ (movies); `SxxEyy` + TMDb title or prettified file name; episode rating and air date; overview; duration, size, subtitle count tooltip, "watched … ago" / "paused at … · ago", "file not available". Buttons: details toggle, set paused time, mark watched/unwatched, Play/Resume. Double-click plays.
- **Details panel:** file, folder, full path, exact size, modified, duration, subtitles, status; Show in Explorer, Copy path, Copy folder, Reset progress.
- **Extras:** grouped by label, each with Explorer, watched toggle, Play.
- Footer "Data from TMDB · fetched YYYY-MM-DD". Dialogs: Fix match (§7), Rename (§8), New collection.

---

## 13. Search

- **Search box** (top bar, `/` focuses, Esc clears): any non-blank query replaces the page with `SearchView` after a 150 ms debounce. `db::search`: titles whose name contains the query (case-insensitive, 30 max) as cards; episodes whose TMDb title or file name matches `LIKE %q%` (40 max) as rows with code, title, file name, paused position, Play. "Titles, people, episodes" is only the placeholder — people are not searched.
- **Ctrl+K palette** (`QuickSearch.vue`): all titles (poster, year · kind · first genre), "Go to" every page (TMDb included), actions: "Search everything for …" (fills the search box), "Switch light / dark", "Rescan libraries" (same as the Sync button).

---

## 14. Collections and tags

- **Tags:** free text per title (`set_item_tags` replaces the set); unused tags are deleted automatically; used by the Tag filter and smart lists.
- **Collections** (`tags.kind = 'collection'`, ordered members): Collections page lists cards (poster = first member's local poster, else the TMDb collection poster URL; "N titles · from TMDB"; watched/total badge and bar). Create by name; inside one: Rename, Delete (list only), Play next (first member with unwatched episodes → its first unwatched episode), rows with position, move up/down (`set_collection_order`), remove. Add from a title page.
- **TMDb collections:** when details name a `belongs_to_collection`, the collection is created on first sight (name, poster URL) and the title attached; members are re-sorted by year then title. `backfill_tmdb_collections` runs at startup for titles matched before this existed. `tags.overview` exists but nothing writes it.

---

## 15. Duplicates (`DuplicatesView.vue`)

- Groups: movies with more than one main (non-extra) file; series episodes with the same season/episode more than once. Header "N groups · X recoverable" (sum of all but the largest file per group). The More menu shows the group count as a badge (refreshed with every view reload).
- Row: quality tag, file name, folder · size · duration · modified · "has watch progress" · "not available"; Explorer; Delete → confirm → `trash_episode`: file to the Recycle Bin (`trash` crate), row deleted, empty items pruned.

---

## 16. History (`HistoryView.vue`)

- One row per file at its most recent session (`ROW_NUMBER` over `episode_id`), up to 300, grouped by Today / Yesterday / date. Row: time, title (+ code and episode title), "Finished" or "Stopped at", "marked by hand" / "estimated"; remove entry; Play.
- Tiles: sessions in the last 30 days, episodes finished this year, hours this year (player sessions: full duration when completed and known, else position), sessions all time.
- "Clear history" (confirm) empties the log; watched marks and positions stay.

---

## 17. Statistics (`StatsView.vue`, `db::stats`)

Tiles: hours watched (player sessions, 1 dp), movies watched / total, episodes watched / total (non-extra), series in library, on disk (TB/GB). Charts: hours per month (last 12 months with activity); genres you finish most (top 8 genres among titles with ≥ 1 completed episode); storage by category (top 10, "Uncategorised"); series in progress (10 most recently watched, with "w / total · ago"); "Gathering dust" (unfinished series untouched ≥ 60 days, 10).

---

## 18. Downloads (`torrent.rs`, `DownloadsView.vue`)

### Engine
- Starts lazily when first needed and `torrent_dir` is set (`torrent_status` from the Downloads page or Settings starts it). "Apply" in Settings stops and restarts it with the new folder/proxy/limits (the old runtime is dropped on a background thread). Needs the folder to exist.
- Session, DHT state and resume data persist in `torrents\`; a saved session is resumed in the background at app start — ordinary downloads get a completion watcher, streams never kept/discarded are paused once initialised.
- Rate limits `torrent_down_kbps` / `torrent_up_kbps` (0 = unlimited) apply at start. Stream server: read-only librqbit HTTP API on `127.0.0.1:<random port>`.
- **Proxy:** `torrent_proxy` must start with `socks5://` or `socks5h://`. In proxy mode DHT, listeners (TCP/uTP), and local discovery are off; UDP trackers are stripped from magnets and a `.torrent` file becomes a magnet carrying only its HTTP(S) trackers. Status `protected` = proxy set. The page shows "Protected via proxy" / "Unprotected — your IP is visible to peers" / "Not running".
- **One-time warning:** without a proxy, the first Stream or Download shows "No proxy configured" → "Download unprotected" sets `torrent_unprotected_ack=1`.

### Adding
- Input accepts a magnet link or a bare info hash (40 hex / 32 base32 → magnet). "Download…" and "Import Torrent File" (`.torrent` picker) run `inspect`: list-only add with a 60 s metadata timeout (messages differ in proxy mode); refuses a torrent already in the session.
- Picker dialog: Save in (default `torrent_dir`, Browse), Create subfolder (default on, name defaults to the torrent name, sanitised), destination preview, file list with sizes (video files ticked by default, else all; All/None), "N of M files · size", Cancel / Stream instead / Start download (needs ≥ 1 file and a folder).
- `add`: free-space check on the Save-in drive ("Not enough free space…"), staging `<save in>\.incomplete\<info hash>` (hidden; `.incomplete` is skipped by scanner and watcher), `only_files`, overwrite on; the destination is remembered in `dests.json`.

### Streaming
- "Stream" (box, picker, deep link): if the same link finished and moved earlier, the largest video under its `completed_path` plays from the library (tracked when it is a library episode) — "Already in your library — playing …". If already in the session, it streams that torrent. Otherwise it inspects, takes the largest video, adds it as **ephemeral** into `torrent_dir` with a subfolder named after the torrent, then streams it.
- `stream_file`: waits until the file has min(2 % of its size, 8 MB) (librqbit fetches first and last pieces first) with a 90 s timeout ("No peers found yet…" / "Still buffering after 90 seconds…"); opens `http://127.0.0.1:<port>/torrents/<id>/stream/<file>/<name>` in the player from the remembered position, tracked. Only selected video files can be streamed.
- When the player closes (newest stream of that torrent only): an ephemeral unfinished torrent is paused; the position is stored (0 when ≥ 90 % of the duration); `stream-ended` is emitted. For ephemeral torrents the page asks "Keep <name>?": **Keep in library** (becomes a normal download, watcher armed, resumed), **Discard** (removed, files deleted), **Decide later** (stays paused with a Watch button). Non-ephemeral: toast "Stopped … at …".
- Row buttons for ephemeral torrents: Watch (stream again, resuming), Keep.

### Completion
A per-torrent watcher polls every 2 s; once finished and live (and not ephemeral) it forgets the torrent in librqbit (files kept), moves the selected top-level entries: with a subfolder into `<save in>\<subfolder>` (a torrent already wrapped in one folder is renamed, not nested; name collisions get " (2)"…), without one directly into Save in; remembers `completed_path`; removes the per-hash staging folder; emits `torrent-done {name, moved}` ("Download finished: …" toast, or "(files stayed in .incomplete)") and triggers a `torrent` scan.

### List and detail
- Rows: name, state badge (Checking/Downloading/Seeding/Paused/Error), "Stream" badge for ephemeral, progress bar, "done of total (%)", down/up speed, peers, ETA, error. Actions: Resume / Pause / Remove (dialog: Keep files / Delete files — delete removes the per-hash staging folder or the torrent's top entries and reports anything left behind).
- Expanded: **Files** (path, size, %, Play for included video files — opens the stream URL untracked), **Info** (elapsed, remaining, wasted, downloaded, uploaded, seeds connected/seen, speeds, peers connecting/queued, limits, share ratio, status; save as, total size, pieces × length, created on/by, hash, comment), **Peers** (address, client, via, state, downloaded, uploaded), **Trackers** (protocol badge; UDP shown struck through "Skipped behind proxy"), **Graphs** (down/up over the last 3 minutes at 2 s samples, peak).
- Bottom bar: DHT ("Off (proxy mode)", "Off", "N nodes", or "no nodes — UDP may be blocked"), speeds and totals, live peers, session uptime.
- Polling: 2 s, one tick at a time; details fetched only for expanded rows on non-Files tabs; expanded state dropped for removed torrents.
- Settings → Downloads: default Save-in folder (Browse, library folder shortcuts), SOCKS5 proxy, "Open magnet links with Vortex" switch, KB/s limits, Apply, status line.
- **Not built:** seed-ratio limits and scheduling (seeding simply stops when a torrent is forgotten on completion).

---

## 19. Deep links and single instance (`deeplink.rs`)

- `vortex://` is registered on every start. `magnet:` is registered only while `magnet_handler` is on (switch in Settings; registered/unregistered immediately; the switch toasts and reverts on error).
- Accepted forms: `magnet:…`, `vortex://stream?magnet=<encoded>` or `?url=<encoded magnet>`. Delivery: window shown, URL queued in `Pending`, `open-url` emitted. The Downloads page's `streamFrom` handles it; `pending_open_urls` is drained by App.vue on mount for links that launched the app.
- A second launch hands its arguments to the running instance and fronts the window.

---

## 20. Tray and window (`tray.rs`, `lib.rs`)

- Tray icon "Vortex": left click shows/unminimises/focuses the window. Menu: up to 3 continue-watching entries ("Title  SxxEyy  (mm:ss)", disabled when the file is offline) that play on click, "Nothing in progress" when none; Open Vortex; Rescan libraries; Quit. Rebuilt after scans, playback end, dismiss/hide-all, reset watch data, restore.
- Closing the window hides it when `close_to_tray` (default on); otherwise the app exits (logged "window closed"). Quit from the tray or Settings "Quit Vortex" (logged "quit from the app").

---

## 21. Backup and restore (`backup.rs`)

- **Back up now:** save dialog (`vortex-backup-YYYY-MM-DD.zip`); `VACUUM INTO` on a fresh connection makes a consistent snapshot including the WAL; zip = `vortex.db` (deflated) + `posters/*` (stored). Records `last_backup` (date) and `last_backup_path`; toast with size and image count.
- **Restore…:** zip picker → confirm dialog (current library/history/settings replaced; a copy kept as `vortex.db.before-restore`). Steps: extract `vortex.db` to `restore-incoming.db` and check it has `libraries`, `episodes`, `watch_progress`; refuse while a poster fetch or duration probe is running; under `jobs::exclusive` and the DB lock: close the live connection, delete `-wal`/`-shm`, rotate the previous `.before-restore` to `.1`, rename live → `.before-restore`, incoming → live, open it. Any failure puts the previous database back and reports the error. Then posters from the zip overwrite files in `posters\`, every `poster_path` is re-pointed into this machine's folder, image names migrated, `library-restored` emitted (UI closes any open title and reloads), tray rebuilt.

---

## 22. Settings reference

| Key | Default | Set from | Effect |
|---|---|---|---|
| `rescan_on_startup` | on | Settings switch | Startup scan (else only the duration probe). |
| `watch_folders` | on | Settings switch | Watcher scans on changes and drive return. |
| `notify_new` | on | Settings switch | Windows toast for watch/drive/torrent arrivals. |
| `autoplay_next` | on | Settings switch | Next episode after a completed one. |
| `close_to_tray` | on | Settings switch | Close hides instead of exiting. |
| `ignore_dirs` | empty | Settings textarea | Extra folder names skipped by scanner and watcher. |
| `player_kind`, `player_path` | auto-detected first run | Video player card | Launch and tracking method. |
| `ffprobe_path` | empty | File durations card | Overrides detection. |
| `tmdb_key`, `tmdb_connected` | empty / unset | TMDb page | Key (masked to UI), connected flag gating auto fetch. |
| `tmdb_auto_match`, `tmdb_adult`, `tmdb_year_fallback`, `tmdb_language`, `tmdb_region`, `tmdb_year_mode`, `tmdb_cert_country`, `tmdb_poster_size`, `tmdb_backdrop_size`, `tmdb_still_size`, `tmdb_profile_size`, `tmdb_delay_ms`, `tmdb_timeout_secs`, `tmdb_cache_days` | see §7 | TMDb page | Matching rules and API parameters (§7). Any `tmdb_*` change reloads `tmdb::Prefs`. |
| `smart_lists` | — | Library pages | JSON array of smart lists. |
| `torrent_dir`, `torrent_proxy`, `torrent_down_kbps`, `torrent_up_kbps` | empty / 0 | Downloads card (Apply) | Engine config. |
| `torrent_unprotected_ack` | unset | Unprotected warning | Suppresses the warning after "Download unprotected". |
| `magnet_handler` | off | Downloads card switch | `magnet:` association. |
| `last_backup`, `last_backup_path` | — | Backup | Shown in Settings and the reset confirm. |

localStorage: `vortex-theme` (system/light/dark), `vortex-sort` (library sort).

---

## 23. UI shell (`App.vue`)

- **Top bar:** logo (Home), nav Home / Movies / Series / Collections / Downloads, "More" menu (History, Statistics, Duplicates with count badge, TMDb, smart lists with delete icons), search box with "Ctrl K" hint, the **status pill**, the **Sync** button, theme toggle (light/dark; Settings offers System too), Settings.
- **Status pill:** shown while any background job runs (wide windows only, `lg:`): spinner, the job's label and detail ("Fetching posters · Inception"), percent when the extent is known, "+N" for further running jobs (all listed in the tooltip), an X to stop a poster fetch, and a 2 px bar along the bottom (pulsing while indeterminate). Jobs are shown in the order rename, scan, memory, posters, durations. Fed by `job-progress`; an entry disappears on its `running: false` event.
- **Sync button:** runs `scan_libraries` (manual scan), toasts "Synced: N files, N added, N renamed, N removed, N offline", and refreshes every view; its icon spins while syncing or while any job runs; a toast refuses while a scan is already running. The bar is transparent over hero pages (Home, title pages) and turns frosted after 24 px of scroll or on other pages. Navigating scrolls to the top.
- **Keyboard:** Ctrl+K palette; `/` focuses search; Esc closes a dialog/menu, else clears search, else leaves a title page. On cards (`useGridKeys`): arrows (row-aware), Home/End, Enter opens, P plays; focus inside a carousel row scrolls the row itself.
- **Cards** (`MediaCard`): poster or gradient with initial and title; rating badge; watched tick; dismiss X (continue row); hover Play (or "Offline"); progress bar; title/subtitle/meta/badge. Rows (`MediaRow`) are drag-free carousels with edge arrows and See all.
- **Toasts:** vue-sonner bottom-right, rich colours, close button. `scan-done` toast only for non-manual scans with added/renamed/removed > 0: "Folder change: …", "Drive connected: …", "Download finished: …", "Rescan: …".
- **Refresh:** `refreshAll` reloads every mounted view (Home, library, title, history, duplicates, collections, stats, duplicate count) once per 120 ms burst, triggered by `playback-ended`, `scan-done`, `library-restored`, `library-changed`, `durations-done` (found > 0), `posters-done`, the Sync button, Settings `scanned` emits (after manual scans, library removal, restore, reset) and the TMDb page's `changed` (after connecting a key or renaming).
- **Up next card:** bottom-right, 10 s countdown, Play now, X.
- **Theme:** `initTheme` before first paint; system preference followed when "system".
- **Browser preview:** `pnpm dev` opened in a plain browser (no Tauri bridge, DEV only) serves sample data from `src/lib/preview.ts` and no events.

---

## 24. Events (backend → UI)

| Event | Payload |
|---|---|
| `job-progress` | `{job: scan\|posters\|durations\|rename\|memory, label, done, total (0 = unknown), detail, running}` |
| `scan-done` | `{reason, stats: ScanStats}` |
| `scan-error` | error string |
| `library-changed` | none (remembered TMDb data applied after a scan) |
| `library-restored` | none |
| `durations-progress`, `durations-done` | `{done, total, found}` |
| `posters-progress`, `posters-done` | `{done, total, matched, current}` (`current` = title, or the error on an aborted run) |
| `playback-ended` | `{episode_id, position_secs, completed, exact, next_episode_id, next_label, auto_started}` |
| `torrent-done` | `{name, moved[]}` |
| `stream-ended` | `{id, name, position_secs, duration_secs, finished, ephemeral, exact}` |
| `open-url` | magnet string |

---

## 25. IPC commands (names)

- **Libraries/scan:** list_libraries, add_library, remove_library, scan_libraries, list_drives, default_ignored_dirs.
- **Browsing:** list_media, get_media_item, item_for_paths, list_categories, set_item_category, list_episodes, continue_watching, search, stats.
- **Progress:** set_progress, clear_progress, hide_from_home, hide_all_from_home, reset_watch_data, set_item_watched.
- **Playback:** play_episode, detect_players, reveal_path.
- **Durations:** probe_durations, detect_ffprobe.
- **TMDb:** fetch_posters, cancel_posters, search_tmdb, apply_tmdb_match, test_tmdb_key, connect_tmdb, disconnect_tmdb, get_tmdb_status, fetch_episode_titles, get_details, tmdb_store.
- **Rename:** rename_preview, rename_apply, rename_undo, rename_can_undo.
- **History/duplicates:** list_history, history_stats, delete_history, clear_history, find_duplicates, trash_episode.
- **Tags/collections:** list_tags, get_tag, create_tag, rename_tag, delete_tag, set_item_tags, add_to_collection, remove_from_collection, set_collection_order, collection_items.
- **Backup/settings/app:** create_backup, restore_backup, get_settings, set_setting, quit_app, reveal_log, log_frontend.
- **Torrents:** torrent_status, torrent_restart, torrent_inspect, torrent_add, torrent_list, torrent_pause, torrent_resume, torrent_remove, torrent_play, torrent_detail, torrent_session_status, torrent_stream, torrent_stream_existing, torrent_keep, torrent_discard, set_magnet_handler, pending_open_urls.

Every command is timed in the UI (`timedInvoke`); failures and calls ≥ 250 ms are logged.

---

## 26. Logging and diagnostics (`logging.rs`, `diag.ts`)

- `vortex.log` in the data folder, local wall-clock timestamps, targets shown. At start the previous `vortex.log` becomes `vortex.log.1` and any `vortex.log.0` is deleted; a run exceeding 20 MB rolls its first half to `vortex.log.0`.
- Default filter `info,ui=debug,vortex_lib=debug,librqbit=info,librqbit_dht=warn`; `VORTEX_LOG` env var overrides it.
- Startup summary: version/OS, player, player path, whether a TMDb key exists and is connected, ffprobe path, rescan/watch flags, torrent folder, whether a proxy is set (never its value), each library with availability. "Vortex closing" with a reason on clean exits, so a log that just stops is a crash. Panics are logged with a backtrace.
- UI → log: uncaught errors and rejections, Content Security Policy violations, long tasks ≥ 200 ms, render stalls (< 30 fps over 5 s with a frame gap > 100 ms, window visible), slow commands ≥ 250 ms, "UI ready" with size and DPR. Messages queue until the bridge is up.
- Settings "Show log" reveals `vortex.log` in Explorer.

---

## 27. Data model (`db.rs`)

- `libraries(id, path UNIQUE, name, added_at)`.
- `media_items(id, kind, title, year, sort_key, category, tmdb_id, poster_path, overview, poster_checked, rating, genres; UNIQUE(kind, sort_key))`.
- `episodes(id, media_item_id →cascade, library_id →cascade, path UNIQUE, file_name, season, episode, size, modified, duration_secs, duration_checked, title, overview, air_date, still_path, rating, added_at, extra, subtitles)`.
- `watch_progress(episode_id PK →cascade, position_secs, completed, last_watched, hidden)`.
- `history(id, episode_id →cascade, at, position_secs, completed, exact, source 'player'|'manual')`.
- `settings(key, value)`.
- `tags(id, name, kind 'tag'|'collection', tmdb_collection_id, poster_url, overview; UNIQUE(kind, name NOCASE))`, `item_tags(tag_id, media_item_id, position)`.
- `tmdb_details` (legacy, per title), `tmdb_cache(kind, tmdb_id, json, fetched_at)`, `tmdb_seasons(tmdb_id, season, json, fetched_at)`, `tmdb_memory(kind, sort_key, tmdb_id, manual, poster, overview, rating, genres, year, updated_at)`.
- Schema is created with `IF NOT EXISTS`; columns added with ignored `ALTER TABLE`s; backfills (genres/ratings from legacy details, `added_at` from mtime, memory/cache from legacy tables) are idempotent. `PRAGMA journal_mode=WAL; foreign_keys=ON`.

---

## 28. Build and release

- `pnpm tauri dev` (Vite on 3420) / `pnpm tauri build` (installers in `src-tauri\target\release\bundle\msi\Vortex_<v>_x64_en-US.msi` and `nsis\Vortex_<v>_x64-setup.exe`). Version lives in `package.json`, `tauri.conf.json`, `Cargo.toml`.
- Release profile: `codegen-units=1`, thin LTO, `opt-level=3`, `panic=unwind`, `debug=line-tables-only`, not stripped.
- Backend tests: `cd src-tauri && cargo test -j 1` (44 tests; `-j 2` can crash rustc with `STATUS_STACK_BUFFER_OVERRUN` when memory is short). Only one compile at a time on the build machine; use `CARGO_BUILD_JOBS=2` for release builds; `npm`/`pnpm` exit 0 even when the Rust build failed, so check installer timestamps.
- No GitHub releases or tags; installers are built and installed locally.
- Cargo note: `librqbit-core` pinned to `sha1-crypto-hash`, `reqwest` 0.13 to `native-tls`, to keep `aws-lc-sys` (CMake + NASM) out of the build.

---

## 29. Known gaps and open requests (2026-10-08)

- **Remakes merge:** titles group by name only, so "Dune (1984)" and "Dune (2021)" are one item; Duplicates offers to delete one as a copy. The rename guard protects files; proper grouping is a pending proposal.
- **Status pill width:** the top-bar pill is hidden below the `lg` breakpoint (narrow windows); the Sync icon still spins there.
- **Automatic matching takes the first search result.** A title match score or a "prefer exact title" rule is not implemented; wrong first results need Fix match.
- **Durations never retried** once `duration_checked` is set, even via "Read missing".
- **Image size changes** apply only to images fetched afterwards; existing posters/backdrops are not re-downloaded.
- **Smart list `maxYear`** has no input; **collection `overview`** is never filled.
- **Torrents:** seed-ratio limits and scheduling not built; "Apply" in Downloads settings pauses about a second (engine restart).
- **Search** placeholder mentions people, but cast is not searched.
