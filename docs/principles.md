# Principles

The rules every change to Vortex respects. A spec that needs an exception says so and why.

- **P1. Local, and only local.** Everything lives on the user's disk: the database, posters, logs, downloads. No account, no server, no telemetry. The only network calls are to TMDb (optional, with the user's own key), to the torrent swarm the user chose, and to the player on `127.0.0.1`.
- **P2. Never a source.** Vortex plays and downloads what the user brings it. There is no search, no index, no catalogue of content, and there never will be. This is what keeps the public repository on the right side of the law.
- **P3. The user's files are the user's.** Vortex reads them. It writes to them only when the user explicitly asks (a rename shown in full first, a Recycle Bin delete confirmed first) and every such action can be undone or recovered. Nothing is ever deleted outright.
- **P4. The window never freezes.** Scans, probes, network calls, process launches and file-existence checks run off the main thread and outside the database lock. A sleeping drive must not stall the UI.
- **P5. Progress survives everything.** Watch positions and history outlive renames, moves, folder switches, drive disconnects, backups and restores. A scan never deletes a row for a file it merely could not read.
- **P6. Warn, don't block.** Privacy and safety risks (an unprotected torrent, the system drive as a library) are explained plainly and asked once; the user decides.
- **P7. External players, exact positions.** Playback happens in the player the user already has. Vortex talks to it to read the real position where it can, and says so when it is only estimating.
- **P8. Secrets stay in the backend.** The TMDb key and proxy credentials are stored in the database, never sent to the UI, and logged only as present or absent.
- **P9. Diagnosable after the fact.** Every run writes a log with local timestamps, a startup summary, panics with backtraces and the renderer's own complaints, so a bug report can be read without reproducing it.
- **P10. One build at a time.** Release builds use most of the build machine's memory. Nothing else compiles while one runs, installers are checked by timestamp, and nothing is published to GitHub; the product owner installs locally.
