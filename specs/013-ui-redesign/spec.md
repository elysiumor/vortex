# 013. UI redesign: light and dark, hero and rows, Ctrl+K

| | |
| --- | --- |
| **Status** | Done (2026-10-08, `8b647f9`) — written from the code on 2026-10-08 |
| **Area** | Shell |
| **Roadmap** | Done: UI redesign |
| **Design doc** | `docs/FEATURES.md` §23 UI shell, §10 Home, §12 Title page |

## Problem

The 1.1.x interface was a utility: a dense grid, a plain top bar, dark only. The product owner wanted the library to look like a streaming front page in both light and dark (reference: an IMDb-style concept board), with a big featured title, sideways rows, large title pages with the backdrop, and keyboard access to everything.

## Goals

- A front page with a full-width hero and sideways-scrolling rows.
- Title pages built around the backdrop with the poster, metadata chips and round cast portraits.
- Light and dark themes, following the system by default, switchable from the top bar, Settings and Ctrl+K.
- A frosted top bar over hero pages, a quick-search palette, and consistent components (shadcn-vue).

## Non-goals

- Changing any rule about what is shown (those are specs 003–006).
- A glassmorphism-everywhere look: glass is used for the bar, badges and row arrows only.

## Where it lives

Every page; `App.vue`, the views, `src/components/ui`, `style.css`.

## Requirements

- **FR-1.** A theme setting (System, Light, Dark) must apply before the first paint, follow the OS when set to System, and be changeable from the top bar toggle, Settings and the palette, all staying in sync.
- **FR-2.** The top bar must be transparent over hero pages (Home, title pages) and turn frosted once the page scrolls or on other pages; it must fit the 1100 px default window.
- **FR-3.** Home must show a billboard hero with backdrop, title, meta chips, overview, resume bar, Play, More info and Shuffle; rows must scroll sideways by drag, edge arrows and keyboard, with See all where there is a full page.
- **FR-4.** Title pages must show the backdrop (or blurred poster), the poster, chips for rating, year, certification, runtime, seasons, status, language and genres, and cast as round portraits.
- **FR-5.** Cards must show a rating badge, a watched tick, a progress bar and a hover Play, with a gradient placeholder when there is no poster.
- **FR-6.** Ctrl+K must open a palette of titles, pages and actions.
- **FR-7.** Toasts must sit bottom-right with a close button and follow the theme.
- **FR-8.** Text must stay readable in light mode (labels over images and chips).

## Rules and edge cases

- The theme is stored per machine (localStorage), not in the database.
- A browser preview (`pnpm dev` without Tauri) renders sample data so the design can be checked without a library.

## Acceptance criteria

- **AC-1** (FR-1). Set the OS to light with the app on System: Vortex is light; toggle from the top bar: dark, and Settings shows Dark; Ctrl+K "Switch light / dark" toggles back.
- **AC-2** (FR-2). Home at 1100 px: the bar is transparent over the hero and all items fit; scroll: it turns frosted.
- **AC-3** (FR-3). Rows move with the arrows, drag, and the keyboard; a focused card out of view is scrolled in.
- **AC-4** (FR-4, FR-8). A matched title page in light mode: backdrop, poster, chips and cast are readable.
- **AC-5** (FR-6). Ctrl+K lists titles with posters and opens one on Enter.

## Principles

- Respects all principles.

## Open questions

Answered 2026-10-08: light and dark both; glass only where it reads well.

## Changelog

- 2026-10-08: Built (`8b647f9`) on shadcn-vue (Carousel, Command, Dialog, DropdownMenu, Tabs, Tooltip and others), Tailwind v4 tokens in `style.css`.
- 2026-10-08: Spec recorded. Done.
