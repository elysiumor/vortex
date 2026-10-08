# 016. Remakes as separate titles: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Not started: waiting for the answers to the spec's open questions.

## Build

- [ ] 1. `parser::group_key` and tests, incl. "Blade Runner 2049 (2017)" (FR-1, FR-7)
- [ ] 2. Scanner lookup for no-year files; logging (FR-2)
- [ ] 3. Upgrade `split_merged_titles` with tests on a fixture database; `schema_version` (FR-3)
- [ ] 4. Memory and rename keyed by `group_key`; remake guard kept (FR-4, FR-6)
- [ ] 5. Duplicates check and test (FR-5)
- [ ] 6. Dry run of the upgrade against a copy of the product owner's database

## Verify

- [ ] `cd src-tauri && cargo test -j 1`
- [ ] `pnpm build`
- [ ] Every acceptance criterion checked (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | | |
| AC-2 | | |
| AC-3 | | |
| AC-4 | | |
| AC-5 | | |

## Close

- [ ] `docs/FEATURES.md` §4, §7, §8, §15, §27 and Known gaps
- [ ] README (grouping line under Features)
- [ ] Roadmap: item moved to Done
- [ ] Decision log: amend D-003
- [ ] Spec status Done with commit
- [ ] Spec index updated
