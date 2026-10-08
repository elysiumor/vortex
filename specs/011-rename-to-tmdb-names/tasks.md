# 011. Rename files to TMDb names: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Retroactive record: built 2026-10-08 (`dd4006d`, `8b647f9`).

## Build

- [x] 1. Naming: safe names, base name, copies told apart, sidecars (FR-2–FR-4)
- [x] 2. Movie plan with remake guard and own-folder rule (FR-5, FR-7)
- [x] 3. Series plan: placement across TMDb seasons, episode names, season folders, show folder (FR-6, FR-7)
- [x] 4. Reads-back and collision checks, path limit (FR-8)
- [x] 5. Apply under exclusive: re-plan, disk rollback, library update, history (FR-9–FR-11)
- [x] 6. Undo (FR-10)
- [x] 7. Preview dialog, title page button, Settings row (FR-1)

## Verify

- [x] `cargo test -j 2`: `rename::tests::names_windows_accepts`, `episodes_are_placed_on_tmdb_seasons`, `a_series_keeps_its_place_in_the_library_after_renaming`, `only_a_folder_named_after_the_film_is_renamed`, `several_files_in_one_folder_get_told_apart`, `sidecars_follow_their_video`, `a_failed_step_puts_everything_back`, `library_paths_follow_a_folder_rename`; `db::tests::tmdb_matches_are_remembered_across_folder_switches` (rekey)
- [x] `pnpm build`
- [x] Acceptance criteria (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | `library_paths_follow_a_folder_rename` + product owner's bulk renames on 2026-10-08 (batches of 103, 9, 233, 2, 1 moves recorded in rename-history.json) | Pass |
| AC-2 | `a_series_keeps_its_place_in_the_library_after_renaming`, `episodes_are_placed_on_tmdb_seasons` | Pass |
| AC-3 | Code path (`movie_moves` partition); not separately exercised | Not re-verified |
| AC-4 | Code path (`reads_back`); not separately exercised | Not re-verified |
| AC-5 | `a_failed_step_puts_everything_back` | Pass |
| AC-6 | Product owner, 1.1.8 | Pass |

## Close

- [x] `docs/FEATURES.md` §8
- [x] README
- [x] Roadmap: Done
- [x] Decision log: D-014
- [x] Spec status Done
- [x] Spec index updated
