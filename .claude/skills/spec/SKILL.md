---
name: spec
description: >-
  Draft a new Vortex spec, or amend an existing one, before any code is written.
  Use when the user asks for a new feature, a change in behaviour, a new page,
  Settings card or background job, a schema change, or picks up a roadmap item,
  or runs /spec with an idea, a roadmap item or a spec number to amend.
---

# Draft a spec

Vortex is built spec-first (`docs/spec-driven-development.md`). This skill writes the **what and why** and then stops for the product owner's approval. It never writes application code.

## Steps

1. **Understand the ask.** Read the request, the matching item in `docs/roadmap.md`, `docs/principles.md`, and the `docs/FEATURES.md` sections for the area it touches. Skim the relevant Rust module or Vue view only to learn how things work today, so the spec describes a real change.
2. **Check it belongs.** If it breaks a principle (P2 above all: no search, no index, no catalogue), say so and stop, or ask whether to record an exception.
3. **Pick the spec.**
   - New: take the next number in `specs/README.md`, copy `specs/_template/spec.md` to `specs/NNN-short-name/spec.md`.
   - Amend: open the existing spec, edit it, and add a Changelog line. A scope or rule change sets the status back to Draft, waiting for approval again.
4. **Write it.** Fill every section of the template:
   - the problem and who has it, before any solution;
   - requirements as numbered, testable `FR-n` statements, grouped by place (page, Settings, tray, background);
   - where it lives and which events the UI reacts to;
   - exact rules and edge cases (offline drive, renamed file, no match, no key, no player, empty library, many titles, light and dark);
   - acceptance criteria `AC-n`, each tied to requirements and checkable in the running app;
   - non-goals, a principles check, and open questions.
   Describe behaviour, not code: no table, file or function names in the spec.
5. **Index it.** Add or update the row in `specs/README.md` (status Draft, area, today's date).
6. **Stop for approval.** Reply with a short summary of the spec (goal, the main requirements, what's out of scope) and the open questions, and ask the product owner to approve it or ask for changes. Use AskUserQuestion for questions with clear options. Don't plan or build until they approve — unless the product owner has said in this conversation that approval isn't needed; then record that in the header and carry on.
7. **On approval,** set `Status: Approved (YYYY-MM-DD)` in the spec header, fold the answers to the open questions into the spec, update the index, and offer `/spec-plan NNN`.
