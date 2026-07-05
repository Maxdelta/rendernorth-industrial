# RenderNorth Industrial — System Architecture

Version 1.0 — Sprint 001

## 1. Stack

| Concern | Choice | Notes |
|---|---|---|
| Shell | Tauri 2.x | Native Windows desktop, small footprint, Rust core |
| UI | React 18 + TypeScript | Vite build, react-router for module routes |
| Backend | Rust (Tauri core process) | All engines and sync live here |
| Storage | SQLite via `rusqlite` (bundled) | Single local file `rendernorth.db` |
| Auth | CCP ESI OAuth 2.0 + PKCE via system browser | Sprint 002+ |
| Static data | EVE SDE import pipeline | Sprint 003+ |

## 2. Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ PRESENTATION (React/TS)                                     │
│ Mission Control · Build Targets · Inventory · Production ·   │
│ Industry · Logistics · Market Intelligence · Planning ·     │
│ Intelligence · Reports · Settings                           │
└──────────────▲──────────────────────────────────────────────┘
               │ Tauri IPC (invoke) — typed DTOs only
┌──────────────┴──────────────────────────────────────────────┐
│ BUSINESS LOGIC (Rust)                                       │
│ Production Engine · Inventory Engine · Shopping Engine ·    │
│ Cost Engine · Recommendation Engine                         │
└──────────────▲──────────────────────────────────────────────┘
               │ Repository traits
┌──────────────┴──────────────────────────────────────────────┐
│ DATA (Rust)                                                 │
│ SQLite · migrations · repositories · query builders         │
└──────────────▲──────────────────────────────────────────────┘
               │ writes normalized rows
┌──────────────┴──────────────────────────────────────────────┐
│ SYNCHRONIZATION (Rust, async)                               │
│ ESI client (OAuth, cache/error-limit aware) · SDE importer  │
│ · optional price feeds (Janice/Fuzzworks, later)            │
└─────────────────────────────────────────────────────────────┘
```

**Dependency rule:** arrows point down only. Presentation knows DTOs; engines know repositories; sync knows the DB schema and external APIs. Nothing reaches upward or skips a layer.

## 3. Process Model

- **Tauri core (Rust):** owns the SQLite connection (behind a `Mutex`/pool), runs engines on demand, runs sync jobs on a background async runtime (tokio) with a scheduler in later sprints.
- **WebView (React):** stateless with respect to truth. It renders DTOs returned by `invoke` commands and issues user intents. No business math in the frontend beyond formatting.
- **IPC contract:** each Tauri command has a versioned request/response DTO defined once in Rust (`serde`) and mirrored in `src/lib/backend.ts`. Commands as of Sprint 002: `health_check`, `get_mission_control`, `list_build_targets`, `select_build_target(project_id)` — selection persists in `app_meta.selected_project_id` and every Mission Control read is generic over it.

## 4. Engines (Business Logic Layer)

The engines compose into one generic pipeline — the **Build Target Engine** — which every production workflow runs, for any target:

```
Select Build Target → Load Blueprint Requirements → Calculate Materials
  → Compare Inventory → Identify Missing Inputs → Recommend Next Action
```

**No ship is special-cased.** The pipeline takes any `type_id` that EVE industry data can manufacture — subcap, capital, supercapital, Titan, structure, or component. The Avatar is a seeded validation scenario, never a code path.

| Engine | Responsibility (all generic over target `type_id`) | Sprint |
|---|---|---|
| Build Target Engine | Pipeline orchestrator: resolve a selected target to its blueprint, drive the stages below, emit a build plan | 004 |
| Inventory Engine | Normalize assets across characters/structures; answer "what do I own / where is it" with rollups by location and category | 003 |
| Production Engine | Expand any blueprint into its material tree (ME/TE aware, recursive through components/reactions), map jobs to build projects, compute buildable-today | 004 |
| Shopping Engine | Diff requirements vs inventory vs in-progress jobs → missing-inputs list with acquisition suggestions | 004 |
| Cost Engine | Job install costs, material valuation from price snapshots, build vs buy | 005 |
| Recommendation Engine | Deterministic rule pipeline over the other engines' outputs; every output carries a machine-readable `reason` | 005 |

Engines are pure where possible: `fn(inputs) -> outputs`, with repositories injected. This keeps them unit-testable without a live DB.

## 5. Synchronization Layer

- **ESI client:** one module per resource family (assets, blueprints, industry jobs, wallet, market orders). Respects `expires`/ETag caching and the error-limit headers; exponential backoff; per-character token refresh.
- **SDE importer:** downloads/ingests the static export into `sde_*` tables (types, blueprints, activities, materials). Versioned; re-importable.
- **Sync state:** every sync writes a `sync_state` row (resource, character, last success, ETag, next allowed fetch) so the UI can show data freshness.

## 6. Error & Freshness Philosophy

Stale data is shown, never hidden: every dashboard panel can surface "as of <timestamp>". Sync failures degrade to last-known-good with a visible warning state, matching the command-center visual language (amber = degraded, red = blocked).

## 7. Directory Layout

```
rendernorth-industrial/
├── docs/                     # architecture documents (this set)
├── src/                      # React presentation layer
│   ├── components/           # Panel, Sidebar, StatCard, gauges…
│   ├── pages/                # one file per module route
│   ├── lib/backend.ts        # typed IPC bridge (mock fallback in browser)
│   ├── data/mock.ts          # Sprint 001 demo data (single source)
│   └── styles/               # theme tokens + app css
├── src-tauri/
│   ├── src/main.rs           # Tauri entry, command registration
│   ├── src/db.rs             # SQLite open/migrate/seed
│   ├── src/commands.rs       # IPC commands (thin: parse → engine → DTO)
│   ├── src/models.rs         # DTOs
│   └── migrations/           # numbered .sql files, embedded at compile time
└── package.json / vite.config.ts / tsconfig.json
```

## 8. Sprint Sequencing (macro plan)

1. **001** — Desktop shell, theme, dashboard with seeded demo data, DB foundation. *(done)*
1b. **002** — Mission Control rename, generic Build Target foundation with working selection, Factory Health console, UI Constitution. *(this sprint)*
2. **003a** — ESI OAuth (PKCE), character management, token storage.
3. **003** — SDE import + Asset/Blueprint sync + Inventory Engine.
4. **004** — Industry jobs sync, Production + Shopping engines, Build Target Engine + Build Tracker on live data.
5. **005** — Cost + Recommendation engines, market/wallet, notifications.
6. **006+** — PI, Mining, Profit Calculator, Asset Safety Recovery.
