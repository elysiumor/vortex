# NNN. Title: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

<!-- Small, ordered steps. Each names the requirements it serves. Tick them off as they're done ([x]). -->

## Build

- [ ] 1. Data: schema and settings keys (FR-…)
- [ ] 2. Backend rules and jobs, with unit tests (FR-…)
- [ ] 3. Commands and events (FR-…)
- [ ] 4. UI (FR-…)
- [ ] 5. Cross-feature effects (FR-…)

## Verify

- [ ] `cd src-tauri && cargo test -j 1`
- [ ] `pnpm build` (vue-tsc)
- [ ] Every acceptance criterion checked (below), light and dark where the UI changed

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | | |

## Close

- [ ] `docs/FEATURES.md` updated to describe what was built (and Known gaps corrected)
- [ ] README updated (feature list, setup or stack), if they changed
- [ ] Roadmap: item moved to Done with a link to this spec
- [ ] Decision log entry, if a lasting decision was made
- [ ] Spec status set to Done, with the date and commit
- [ ] Spec index (`specs/README.md`) updated
