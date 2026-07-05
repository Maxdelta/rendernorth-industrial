# RenderNorth Industrial — Sprint 001 Implementation Plan

Version 1.0

## Goal

A working desktop shell with the correct architecture: Tauri + React + TypeScript scaffold, SQLite foundation with migrations and seed data, the dark command-center theme, a fully mocked Factory Status dashboard, and placeholder routes for every planned module. No ESI integration this sprint — the foundation must be clean first.

## In scope

1. **Scaffold** — Tauri 2 project: Vite/React/TS frontend, Rust core, `tauri.conf.json`, icons, dev/build scripts.
2. **Docs** — the six architecture documents (this set) committed under `docs/`.
3. **Data foundation** — `db.rs`: open `rendernorth.db` in the app data dir, WAL + foreign keys, migration runner over embedded `migrations/0001_init.sql`, idempotent demo seed (flagged `is_demo`).
4. **IPC foundation** — `commands.rs` + `models.rs`: `health_check` (DB connectivity + migration version) and `get_factory_status` (reads seed rows into one DTO).
5. **Shell UI** — sidebar nav (grouped), topbar (module title, DEMO badge, UTC clock), route outlet.
6. **Theme** — `theme.css` tokens per UI_ARCHITECTURE.md; chamfered-plate panel primitive.
7. **Dashboard** — Factory Status page rendering the demo dataset: 34 running jobs, 2 idle characters, 7 idle BPOs, seeded demo target (Avatar) @ 61% rendered through the generic Selected Build Target panel, tier gauges (Minerals 100 / Capital 63 / Advanced 44 / PI 80), wallet 50.2B ISK, missing materials (812k Nocxium, 43 Auto-Integrity Preservation Seals, 18 Broadcast Nodes), recommendation "Start Capital Construction Parts" with reason line.
8. **Placeholders** — 12 module pages via shared `PlaceholderPage`.
9. **Browser fallback** — `backend.ts` mock path so `npm run dev` works without the Rust toolchain.

## Out of scope (deferred)

ESI OAuth, token storage, SDE import, any live sync, engines beyond stubs, notifications, settings persistence, font vendoring, auto-update, tests beyond compile checks.

## Task breakdown

| # | Task | Layer | Acceptance |
|---|---|---|---|
| 1 | Repo + toolchain scaffold | infra | `npm run dev` serves UI; `npm run tauri dev` launches window on Windows |
| 2 | Theme tokens + Panel primitive | presentation | Chamfered plates with status keels render per spec |
| 3 | Shell: Sidebar, TopBar, router | presentation | All 13 routes navigable |
| 4 | Migration runner + 0001_init | data | Fresh DB reaches version 1; re-run is a no-op |
| 5 | Demo seed | data | Seed matches the mock dataset; idempotent |
| 6 | `health_check`, `get_factory_status` | IPC | Dashboard shows `source: "sqlite"` when run under Tauri |
| 7 | Factory Status dashboard | presentation | Matches §7 dataset via generic Selected Build Target DTO; warning states on gaps/idle |
| 8 | Placeholder pages | presentation | Every module lists mission + planned features + sprint |
| 9 | README run instructions | docs | A new dev can run it from the README alone |

## Definition of done

- `npm install && npm run build` passes (TypeScript strict, Vite production build).
- `npm run tauri dev` on a Windows machine with Rust installed opens the shell, dashboard reads from SQLite, and `health_check` reports migration version 1.
- No layer violations: pages import only from `lib/`, `components/`, `data/`.
- Sprint Summary delivered for Memory Review.

## Risks / notes

- Rust compile could not be executed in the authoring environment (no cargo); `cargo check` on Windows is the first task when pulling this sprint. Rust code is written conservatively against tauri 2.x + rusqlite (bundled) to minimize surprise.
- Google Fonts require network on first launch; system fallbacks keep the app usable offline until fonts are vendored (Sprint 002 chore).
