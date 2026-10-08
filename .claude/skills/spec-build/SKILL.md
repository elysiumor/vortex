---
name: spec-build
description: >-
  Build an approved and planned Vortex spec from its task list, verify every
  acceptance criterion, and close it by updating docs/FEATURES.md, the README,
  the roadmap and the spec index. Use when the user runs /spec-build with a spec
  number, or asks to implement or finish a planned spec.
---

# Build and close a spec

Implements `specs/NNN-name/` from its `tasks.md`, then closes it. See `docs/spec-driven-development.md`.

## Steps

1. **Check the gates.** The spec must be Approved and `plan.md` must exist (Agreed, or the product owner said to go ahead). If not, stop and point to `/spec` or `/spec-plan`.
2. **Start.** Set the spec's status to In progress and update `specs/README.md`. Check `git status`: another session may be building a different spec in the same checkout; stay inside your spec's files and never revert theirs.
3. **Build in task order.** Follow `tasks.md`, ticking each task (`[x]`) as it's finished. Follow `AGENTS.md`. Keep to the spec: if the work needs something the spec doesn't say, or contradicts it, stop and propose an amendment rather than deciding silently.
4. **Verify.**
   - `cd src-tauri && cargo test -j 1` (never while another cargo or `pnpm tauri` build runs; `-j 2` has crashed rustc) and `pnpm build`.
   - Check every acceptance criterion in the running app (`pnpm tauri dev` through the preview tools, or the installed build) against a real library, light and dark where the UI changed. Fill in the verification table in `tasks.md` with how each was checked and the result.
   - Report failures faithfully. Don't close a spec with an unverified criterion; say which ones are open and why.
   - When the product owner wants an installer: wait until nothing else compiles, run `CARGO_BUILD_JOBS=2 pnpm tauri build`, check the `.msi` and `-setup.exe` timestamps, hand over the paths. Never publish to GitHub.
5. **Close.**
   - Update `docs/FEATURES.md` so it describes what was built, and fix its Known gaps section.
   - Update `README.md` if the feature list, setup or stack changed.
   - Move the roadmap item to Done in `docs/roadmap.md`, with a one-line summary and a link to the spec.
   - Add a decision log entry (`docs/decisions.md`) for any lasting decision.
   - Set `Status: Done (YYYY-MM-DD, commit)` in the spec and update `specs/README.md`.
6. **Report.** Summarise what was built, how each acceptance criterion was verified, and which docs changed. Commit only if the user asks, referencing the spec (`Spec NNN: …`).
