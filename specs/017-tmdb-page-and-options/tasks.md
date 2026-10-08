# 017. TMDb page and API options: tasks

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md)

Built 2026-10-08 (`5ba2545`), shipped as 1.1.11.

## Build

- [x] 1. `Prefs` and `load_prefs`, reloaded on `tmdb_*` setting changes (FR-4, FR-5, FR-6)
- [x] 2. Language, region, year parameter, certification country, image sizes, delay, timeout at every call site (FR-4)
- [x] 3. `include_adult` on search; manual always, automatic by setting; year fallback switch; auto-match gate (FR-3)
- [x] 4. `TmdbView.vue` with the six cards; Settings link; menu and palette entries (FR-1, FR-2)

## Verify

- [x] `cd src-tauri && cargo test -j 1`
- [x] `pnpm build`
- [ ] Every acceptance criterion checked in the installed build (below)

| AC | How it was checked | Result |
| --- | --- | --- |
| AC-1 | Dev build in the build session | Pass (dev) |
| AC-2 | Dev build: "Monella" found with adult included | Pass (dev) |
| AC-3 | Not re-verified | Open |
| AC-4 | Dev build | Pass (dev) |
| AC-5 | Code path (defaults in `Prefs::default`, clamps in `load_prefs`) | Not re-verified |

## Close

- [x] `docs/FEATURES.md` §7, §22
- [x] README unchanged
- [x] Roadmap: Done
- [x] Decision log: nothing new
- [x] Spec status Done with commit
- [x] Spec index updated
