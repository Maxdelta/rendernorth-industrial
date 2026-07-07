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

## 1a. Domain Hierarchy (refined 003, extended 003.5, Operation Engine live in 004, Reservation Engine live in 005, Blueprint Engine live in 006, Production Requirement Engine live in 007)

```
Mission Control
      │
      ▼
Operation Engine            (owns goals, deadlines, priority, notes,
      │                      selected build target, timeline, production plan)
      │  requests a hold, never owns inventory
      ▼
Reservation Engine          (owns who owns inventory — reads live, mutations stubbed)
      │
      ▼
Inventory Engine            (the single source of truth for everything owned)
      │
      ▼
   SQLite

Blueprint Engine             (sibling to the above — owns industrial capability:
                              BPO/BPC, ME/TE, runs, research/copy status,
                              blueprint readiness by operation)

Production Requirement Engine (sibling to the above — owns required inputs:
                              what quantity of what material an operation
                              needs, coverage/shortage always derived live
                              against Inventory Engine data)
```

Blueprint Engine and Production Requirement Engine sit alongside
Inventory, Operation, and Reservation rather than inside the vertical
chain above — an Operation's blueprint readiness and requirement coverage
are questions answered independently of each other and of whether its
inventory is reserved, the same way its reservation state is independent
of its own lifecycle fields.

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
its goal, deadline, priority, notes, target, timeline, and progress. As of
Sprint 004 it is real, not just an interface: `src-tauri/src/operation/`
(mirroring the Inventory module's shape — `models.rs`, `repository.rs`,
`provider.rs`, `engine.rs`) backed by migration 0004's `operations`,
`operation_timeline`, and `operation_dependencies` tables. Reads (list,
priority queue, blocked detection, upcoming completions, health, detail)
are live. Mutations (create/reprioritize/reschedule/annotate, and
requesting a reservation) are declared on `OperationEngine` but each
returns an explicit "architecture-only" error — no lifecycle mutation
exists yet, and an Operation still cannot actually request a hold from the
Reservation Engine.

**The Reservation Engine owns who owns inventory** — reservation
quantities, reservation history, and reservation conflicts. As of Sprint
005 it is real: `src-tauri/src/reservation/` (same four-file shape —
`models.rs`, `repository.rs`, `provider.rs`, `engine.rs`), a sibling
module to `inventory/` and `operation/`, not nested inside either. It
extends migration 0003's `inventory_reservations` table additively
(migration 0005 adds an `operation_id` column, backfilled from the
existing `project_id`) and adds `reservation_events` (append-only history:
reserved/released/transferred/expired) and `reservation_conflicts`.
Reads (summary, detail, history, conflict detection, inventory
commitment, per-operation reservation totals) are live. Conflict
detection is always computed fresh from current reservation + inventory
state — the same "derive, don't trust a stale flag" precedent as
`is_blocked` and `coverage_for_operation` — never a status a person has to
remember to update. Mutations (reserve/release/transfer) are declared on
`ReservationEngine` but each returns an explicit "architecture-only"
error; `OperationEngine::request_reservation` is the call this engine will
answer once its own mutations exist.

`build_projects` (migrations 0001–0002) is untouched and remains the live
backing table for the existing Build Targets / Inventory Coverage flow.
For Sprint 004, `operations.operation_id` intentionally shares the same
1–5 numbering as `build_projects.project_id` so the demo data tells one
coherent story across both tables; operation 6 ("Prepare Titan
Components") has no `build_projects` counterpart, proof an Operation need
not map 1:1 to a single hull. Consolidating the two tables is deferred
until the Operation Engine is ready to take over target selection
entirely.

**The Blueprint Engine owns industrial capability** — blueprints are
industrial capability records, not just inventory items. As of Sprint 006
it is real: `src-tauri/src/blueprint/` (the same four-file shape —
`models.rs`, `repository.rs`, `provider.rs`, `engine.rs`), a sibling
module to `inventory/`, `operation/`, and `reservation/`. Migration 0006
adds `blueprints` (BPO vs BPC, ME/TE level, runs remaining, research/copy
status) and `operation_blueprint_requirements` (what an operation needs,
independent of whether it's owned). `inventory_items`'s existing
"blueprints" category rows (migration 0003) are untouched — a handful of
`blueprints` rows link back to them via a nullable `inventory_item_id`,
same coexistence pattern as `operations`/`build_projects`. Reads (list,
detail, summary, missing-blueprint report, per-operation and
cross-operation readiness) are live; missing-blueprint detection and
research/copy warnings are always computed fresh from current data, never
a stored flag. Mutations (start research, start copy, acquire) are
declared on `BlueprintEngine` but each returns an explicit
"architecture-only" error.

**The Production Requirement Engine owns required inputs** — "what does
this operation actually require to complete?" As of Sprint 007 it is
real: `src-tauri/src/production/` (the same four-file shape), a sibling
module to `inventory/`, `operation/`, `reservation/`, and `blueprint/`.
Migration 0007 adds `production_requirements` (a material, a quantity, an
operation), `production_requirement_groups` (structural category scope
per operation — never a source of computed numbers), and
`production_requirement_sources` (provenance: why a requirement's
quantity was justified). Coverage, shortage, and cross-operation
"critical bottleneck" detection are computed in Rust over rows fetched
from SQLite rather than in increasingly complex nested queries — always
derived fresh, never a stored flag. `requirements_for_build_target`
forwards to the same operation-scoped breakdown query rather than
duplicating it, since operations 1–5 share id space with
`build_projects.project_id`. Mutations (declare a requirement, adjust a
required quantity) are declared on `ProductionEngine` but each returns an
explicit "architecture-only" error.

## 2. Layer Model

```
┌─────────────────────────────────────────────────────────────┐
│ PRESENTATION (React/TS)                                     │
│ Mission Control · Operations · Build Targets · Inventory ·  │
│ Blueprints · Production (live) · Industry · Logistics ·     │
│ Market Intelligence · Planning · Intelligence · Reports ·   │
│ Settings                                                     │
└──────────────▲──────────────────────────────────────────────┘
               │ Tauri IPC (invoke) — typed DTOs only
┌──────────────┴──────────────────────────────────────────────┐
│ BUSINESS LOGIC (Rust)                                       │
│ Production Requirement Engine · Inventory Engine ·          │
│ Blueprint Engine · Shopping Engine · Cost Engine ·           │
│ Decision Engine                                              │
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
| **Operation Engine** | Owns an Operation's identity: goal, deadline, priority, notes, target, timeline, progress, dependencies. Never touches inventory directly — only requests reservations (once the Reservation Engine is real). **Live (reads) as of Sprint 004** — `src-tauri/src/operation/` (models/repository/provider/engine, mirroring Inventory); `OperationEngine` also exposes mutation methods that currently all return "architecture-only" errors. `OperationQuery` (the narrow read-only seam for the Decision Engine) is implemented by `OperationEngine` itself. | 004 (reads), later (mutations) |
| **Reservation Engine** | Owns who owns inventory: reservation quantities, reservation history, reservation conflicts (detect only, never resolves). **Live (reads) as of Sprint 005** — `src-tauri/src/reservation/` (a sibling module to `inventory/`/`operation/`, mirroring their shape); mutation methods (reserve/release/transfer) currently all return "architecture-only" errors. | 005 (reads), later (mutations) |
| **Blueprint Engine** | Owns industrial capability: BPO vs BPC, ME/TE level, runs remaining, research/copy status, blueprint readiness (owned/missing/warnings) by operation. Never touches inventory, reservation, or operation-lifecycle tables. **Live (reads) as of Sprint 006** — `src-tauri/src/blueprint/` (a sibling module, same four-file shape); mutation methods (start research, start copy, acquire) currently all return "architecture-only" errors. | 006 (reads), later (mutations) |
| **Production Requirement Engine** | Owns required inputs: what quantity of what material an operation needs, coverage/shortage/critical-bottleneck detection, always derived live against Inventory Engine data. Never touches inventory, reservation, operation-lifecycle, or blueprint tables directly. **Live (reads) as of Sprint 007** — `src-tauri/src/production/` (a sibling module, same four-file shape); mutation methods (declare requirement, adjust quantity) currently all return "architecture-only" errors. | 007 (reads), later (mutations) |
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
1d. **003.5** — Architecture refinement: Operation Engine introduced as a first-class concept; Recommendation Engine renamed to Decision Engine (`DecisionContext`, `DecisionKind`, `DecisionRule`). Interfaces only — no schema change, no production math, no live reservations. *(done)*
1e. **004** — Operation Domain Foundation: migration 0004 (`operations`, `operation_timeline`, `operation_dependencies`); real `operation/` module with live reads (list, priority queue, blocked detection — derived from dependencies, not a hand-set flag — upcoming completions, health, detail); Mission Control becomes an Operations Dashboard; new Operations Workspace page. Mutations remain architecture-only; no production math, reservation logic, decision logic, ESI, or blueprint/shopping calculations. *(done)*
1f. **005** — Reservation Engine Foundation: migration 0005 (additive `operation_id` column on `inventory_reservations`, plus `reservation_events` and `reservation_conflicts`); real `reservation/` module with live reads (summary, detail, history, conflict detection, inventory commitment, per-operation reservation totals). Mission Control gains an Inventory Commitment panel and a Reservation Conflicts panel; the Operations Workspace gains a live Reservations section; the Inventory page gains Reserved/Free/Available columns per item. Mutations (reserve/release/transfer) remain architecture-only; no production math, ESI, scheduling, manufacturing, shopping, or Decision Engine rules. *(done)*
1g. **006** — Blueprint Domain Foundation: migration 0006 (`blueprints`, `operation_blueprint_requirements`); real `blueprint/` module with live reads (list, detail, summary, missing-blueprint report, per-operation and cross-operation readiness). New top-level Blueprints page; Mission Control gains a Blueprint Readiness panel; the Operations Workspace gains a live Blueprints section (required/owned/missing/research-copy warnings). Mutations (research, copy, acquire) remain architecture-only; no ESI, SDE import, production math, full build trees, manufacturing job logic, shopping logic, or decision rules. *(done)*
1h. **007** — Production Requirement Engine Foundation: migration 0007 (`production_requirements`, `production_requirement_groups`, `production_requirement_sources`); real `production/` module with live reads (lines, detail, categories, summary, shortages, cross-operation critical bottlenecks, per-operation and build-target breakdown). Production page becomes live (requirement tree, category filters, shortage report, critical bottlenecks); Mission Control gains a Production Readiness panel; the Operations Workspace gains a live Production Requirements section (per-category coverage bars). Mutations (declare requirement, adjust quantity) remain architecture-only; no ESI, SDE import, shopping logic, manufacturing jobs, production scheduling, or Decision Engine rules. *(this sprint)*
2. **003a/004a** — ESI OAuth (PKCE), character management, token storage.
3. **003** — SDE import + Asset/Blueprint sync + Inventory Engine.
4. **004** — Industry jobs sync, Production + Shopping engines, Build Target Engine + Build Tracker on live data.
5. **005** — Cost + Recommendation engines, market/wallet, notifications.
6. **006+** — PI, Mining, Profit Calculator, Asset Safety Recovery.
