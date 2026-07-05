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

## 1a. Domain Hierarchy (refined Sprint 003, extended Sprint 003.5)

```
Mission Control
      │
      ▼
Operation Engine            (owns goals, deadlines, priority, notes,
      │                      selected build target, timeline, production plan)
      │  requests a hold, never owns inventory
      ▼
Reservation Engine          (architecture-only — see below)
      │
      ▼
Inventory Engine            (the single source of truth for everything owned)
      │
      ▼
   SQLite
```

**Operations do not own inventory.** An Operation (tracked today as a row
in `build_projects`; conceptually owned by the Operation Engine) never has
items "moved into" it. It asks the Reservation Engine to hold a quantity
against it; the Inventory Engine reflects that hold as reduced availability
on the underlying item. Example: 48 Broadcast Nodes total, 30 available, 18
reserved for Operation "Avatar" — the Broadcast Node row never changes
owner or location because of this, only its reserved/available split
changes. This mirrors real manufacturing ERP systems (soft allocation vs.
hard reservation vs. consumption) and is why `inventory_allocations` and
`inventory_reservations` are separate tables (migration 0003): allocations
are plan-level intent, reservations are enforced holds.

**The Operation Engine is the layer that owns an Operation's identity** —
its goal, deadline, priority, notes, which build target it has selected,
its timeline, and its production plan. It is a first-class concept as of
this sprint, with an interface (`src-tauri/src/operation.rs`:
`OperationEngine`, `OperationQuery`, `OperationProfile`) but no
implementation and no new schema — `build_projects` still carries only
name/target/status/coverage today. Extending it with real deadline/
priority/notes/timeline columns is deferred until the engine is actually
built, so this sprint intentionally adds no migration.

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
│ Cost Engine · Decision Engine                               │
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
| **Inventory Engine** | **The single data core.** Everything owned — ships, modules, minerals, ore, PI, components, blueprints, charges, fuel, structures, deployables — is inventory; categories are views over it, never separate systems. Answers what/where/how much/whose, and (placeholder formula, see below) how covered an operation is. `src-tauri/src/inventory/`. **Live as of Sprint 003.** | 003 |
| **Operation Engine** | Owns an Operation's identity: goal, deadline, priority, notes, selected build target, timeline, production plan. Never touches inventory directly — only requests reservations. **Interface-only** — `src-tauri/src/operation.rs` defines `OperationEngine`, `OperationQuery`, `OperationProfile`; `build_projects` remains the live (partial) backing table. | 003.5 (interface), later (implementation) |
| Reservation Engine | The seam between the Operation Engine and the Inventory Engine: hold/release quantity against an operation. **Architecture-only** — `src-tauri/src/inventory/reservation.rs` defines the trait and the schema (`inventory_reservations`) exists, but nothing calls it yet. | 003 (interface), later (implementation) |
| Build Target Engine | Pipeline orchestrator: resolve a selected target to its blueprint, drive the stages below, emit a build plan | 004 |
| Production Engine | Expand any blueprint into its material tree (ME/TE aware, recursive through components/reactions), map jobs to build projects, compute buildable-today | 004 |
| Shopping Engine | Diff requirements vs inventory vs in-progress jobs → missing-inputs list with acquisition suggestions | 004 |
| Cost Engine | Job install costs, material valuation from price snapshots, build vs buy | 005 |
| **Decision Engine** *(renamed from Recommendation Engine)* | Deterministic rule pipeline answering build/buy/mine/sell/research/copy/react questions from a `DecisionContext` (inventory + operation state); every output carries a machine-readable `reason`. LLMs may explain a decision in friendlier prose later; they never produce one. **Interface-only** — `src-tauri/src/decision.rs` defines `InventoryQuery`, `OperationQuery`-backed `DecisionContext`, `DecisionKind`, `Decision`, `DecisionRule`, `DecisionEngine`. The live Mission Control recommendation panel (seeded `recommendations` rows, unchanged) is this engine's working ancestor, not yet replaced. | 005 |

### Inventory Engine internals (Sprint 003)

```
src-tauri/src/inventory/
├── models.rs       DTOs: InventoryCategory, InventoryLocation, InventoryItem,
│                   InventorySummary, InventoryAllocation, InventoryReservation
├── repository.rs   InventoryRepository — the only SQL against inventory tables
├── provider.rs     InventoryProvider trait; MockInventoryProvider (live, demo-seeded);
│                   EsiInventoryProvider (stub — returns "not implemented")
├── engine.rs       InventoryEngine<P: InventoryProvider> — what every other
│                   system calls; engine_for(conn) wires today's provider
└── reservation.rs  ReservationEngine trait + NotYetImplementedReservationEngine —
                    architecture only, not wired to any command
```

Swapping `MockInventoryProvider` for a future `EsiInventoryProvider` is a
one-line change in `engine_for()` — no command signature, DTO, or frontend
code changes. Mission Control's "Inventory Coverage" now calls
`InventoryEngine::coverage_for_operation`, a placeholder formula (average of
a target's requirement-tier coverages) that will be replaced with true
item-level requirement matching once the Production Engine exists — it is
never a hardcoded literal on the project row.

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
1b. **002** — Mission Control rename, generic Build Target foundation with working selection, Factory Health console, UI Constitution. *(done)*
1c. **003** — Inventory Engine foundation: repository, provider seam, Inventory page, Reservation Engine and Recommendation Engine interfaces (architecture only). *(done)*
1d. **003.5** — Architecture refinement: Operation Engine introduced as a first-class concept; Recommendation Engine renamed to Decision Engine (`DecisionContext`, `DecisionKind`, `DecisionRule`). Interfaces only — no schema change, no production math, no live reservations. *(this sprint)*
2. **003a/004a** — ESI OAuth (PKCE), character management, token storage.
3. **003** — SDE import + Asset/Blueprint sync + Inventory Engine.
4. **004** — Industry jobs sync, Production + Shopping engines, Build Target Engine + Build Tracker on live data.
5. **005** — Cost + Recommendation engines, market/wallet, notifications.
6. **006+** — PI, Mining, Profit Calculator, Asset Safety Recovery.
