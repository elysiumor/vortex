# Decision log

Lasting technical and product decisions, with the reason. Newest last. A spec that makes a decision that outlives it adds a line here.

| # | Date | Decision | Why |
| --- | --- | --- | --- |
| D-001 | 2026-09-19 | **Tauri 2 + Rust backend, Vue 3 front end, SQLite via rusqlite (bundled, WAL).** No SQLx. | Single-user desktop app; synchronous microsecond queries fit; nothing to install on the user's PC; SQLx's compile-time checking would cost a database at every build on a machine where builds are already the bottleneck. |
| D-002 | 2026-09-19 | **External players only.** Vortex launches PotPlayer, VLC, mpv or MPC-HC and reads the position from them; it has no built-in player. | The user's player already handles every codec and subtitle; the value is the exact resume, not another player. |
| D-003 | 2026-09-19 | **Titles are grouped by name key** (`sort_key`: lowercase alphanumerics of the parsed title), unique per kind. | Every season, release and rename of one show lands on one title. Known cost: remakes with the same name merge (roadmap, spec 016). |
| D-004 | 2026-09-20 | **Torrents: a transport, never a source.** No search, no index, no catalogue, ever. | Keeps the public repository legal. The user brings magnet links and .torrent files. |
| D-005 | 2026-09-20 | **Privacy: warn, don't block.** An unprotected swarm is shown as a persistent badge and asked about once; it is never refused. | The user's decision; most of the time they are behind a VPN anyway. |
| D-006 | 2026-09-20 | **A SOCKS5 proxy is the kill switch on Windows.** DHT, uTP, local discovery, the listener and UDP trackers are off in proxy mode. | librqbit's adapter binding is Linux/macOS only, so on Windows the proxy itself must carry every connection; anything that can't be proxied is switched off instead of leaking. |
| D-007 | 2026-09-20 | **Downloads stage in `<save in>\.incomplete\<hash>` and move into the library on completion; seeding stops then.** | The scanner never sees half-finished files; the folder watcher picks up the finished one like any arrival. |
| D-008 | 2026-09-20 | **Streams play in the external player over a local HTTP URL; keep/discard is asked when the player closes.** | Same as D-002; a stream is not a download until the user says so. |
| D-009 | 2026-09-20 | **`librqbit-core` on `sha1-crypto-hash`, `reqwest` 0.13 on `native-tls`.** | Keeps `aws-lc-sys` (needs CMake + NASM, absent on the build machine) out of the build. |
| D-010 | 2026-09-23 | **Nothing that can block runs on the main thread or under the database lock**: file existence, process launches, network, drive enumeration go to blocking threads; scans use their own connection. | Sleeping and unplugged drives froze the window for seconds (releases 1.1.1 to 1.1.3). |
| D-011 | 2026-09-23 | **No GitHub releases.** Installers are built locally and installed by the product owner. | Several releases went out with unreproduced bugs; the product owner tests locally first. |
| D-012 | 2026-10-08 | **Posters and backdrops are stored by TMDb id**, not by library title id. | Title ids are reused after folders are removed; a new title could show an old title's picture. Images now survive folder switches and are shared by every title matched to the same film. |
| D-013 | 2026-10-08 | **TMDb data is kept across folder switches** (`tmdb_memory`, `tmdb_cache`, `tmdb_seasons`), keyed the way the scanner groups titles. | The user switches folders often; re-asking TMDb cost API quota and minutes. |
| D-014 | 2026-10-08 | **Renames are previewed, all-or-nothing per title, recorded for undo, and run while scans are held off.** | Files are the user's (P3); a half-renamed series would come back from the next scan as two titles. |
| D-015 | 2026-10-08 | **Thin LTO, `panic = "unwind"`, line tables kept, pnpm only.** | Fat LTO's final step ran out of memory on the build machine; a panic in a background task must not kill the app; backtraces must name a line; one lockfile. |
| D-016 | 2026-10-08 | **Spec-driven development** from this date; the app as built is described in `docs/FEATURES.md`. | See `docs/spec-driven-development.md`. |
