---
name: spec-plan
description: >-
  Write the technical plan and task list for an approved Vortex spec. Use when
  the user runs /spec-plan with a spec number, or asks to plan an approved spec.
---

# Plan an approved spec

Turns an approved spec (`specs/NNN-name/spec.md`) into `plan.md` (how) and `tasks.md` (ordered steps). See `docs/spec-driven-development.md`.

## Steps

1. **Check the gate.** Open the spec. If its status isn't Approved, stop and say so: the spec needs approval first (`/spec`).
2. **Study the system.** Read the `docs/FEATURES.md` sections the spec lists, `docs/principles.md`, `docs/decisions.md`, and the code the change touches: `src-tauri/src/db.rs` (schema), the module for the area, `commands.rs` and `lib.rs` (registration), `src/lib/api.ts`, and the Vue views involved.
3. **Write `plan.md`** from `specs/_template/plan.md`:
   - data: tables, columns (ignored `ALTER TABLE` in `db::open`, how existing rows are filled), settings keys, files in the app data folder;
   - backend: modules and functions, which job it runs under (`jobs::Job`, `exclusive`), what runs on a blocking thread and outside the DB lock (P4), unit tests next to the existing ones;
   - events and commands (`api.ts`, `lib.rs` handler list);
   - UI: views and shadcn-vue components, the `refreshAll` path, toasts, light and dark;
   - cross-feature effects (Home, Continue watching, tray, Duplicates, watcher, rename, backup/restore, log), risks;
   - which `FEATURES.md` sections change.
   If planning shows the spec is wrong or incomplete, stop and propose a spec amendment instead of planning around it.
4. **Write `tasks.md`** from `specs/_template/tasks.md`: small ordered steps, each naming the `FR-n` it serves, then the verification table with one row per `AC-n`, then the close-out steps.
5. **Stop for agreement.** Summarise the plan (data changes, backend, UI, risks) and ask the product owner to agree it or ask for changes. For a small, low-risk spec, say so and suggest going straight to `/spec-build NNN`. Skip the stop if the product owner has said approval isn't needed.
6. **On agreement,** set `Status: Agreed (YYYY-MM-DD)` at the top of `plan.md` and offer `/spec-build NNN`.
