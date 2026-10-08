# 013. UI redesign: plan

Spec: [spec.md](spec.md) · Status: Agreed (as built; recorded 2026-10-08)

## Approach

shadcn-vue components generated into `src/components/ui` (alert-dialog, avatar, badge, button, card, carousel, checkbox, command, dialog, dropdown-menu, input, progress, scroll-area, select, separator, skeleton, switch, tabs, textarea, toggle, toggle-group, tooltip) over Tailwind v4 design tokens in `style.css` (brand gradient yellow→lime→green, light and `.dark` palettes, glass and card-shadow utilities). `theme.ts` applies the class before mount; `MediaRow` wraps Embla carousel; `QuickSearch` wraps the Command dialog.

## Data

localStorage `vortex-theme`.

## Backend

None.

## Events and commands

None new.

## UI (`src`)

- `style.css`, `lib/theme.ts`, `main.ts`, `App.vue`, `HomeView.vue`, `SeriesView.vue`, `LibraryView.vue`, `MediaCard.vue`, `MediaRow.vue`, `QuickSearch.vue`, `EmptyState.vue`, `lib/preview.ts` (sample data for a plain-browser preview), every view's header typography.

## Cross-feature effects

- None on rules. The package manager moved to pnpm in the same release (D-015).

## Risks

- Carousel focus and arrow keys need custom handling (done in `MediaRow.onFocusIn` and `useGridKeys`).

## Design doc updates

- `docs/FEATURES.md` §23.
