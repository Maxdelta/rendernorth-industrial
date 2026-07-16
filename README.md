# RenderNorth Industrial

Open Beta 0.1.1 first launch is guided by an in-app setup wizard using the official RenderNorth Industrial CCP application, official CCP JSONL static data, character synchronization, and market refresh. See [Open Beta Onboarding](docs/OPEN_BETA_ONBOARDING.md) and the [Open Beta Release Checklist](docs/OPEN_BETA_RELEASE_CHECKLIST.md).

Public release history is maintained in [CHANGELOG.md](CHANGELOG.md) and `src/data/releases.json`. The permanent release procedure and reusable template are in [Release Process](docs/RELEASE_PROCESS.md).

Update discovery uses the official public GitHub Releases API and opens downloads in the default browser. Automatic checks can be disabled, and offline use remains fully supported. See [Update Availability](docs/UPDATES.md).

Desktop-first EVE Online industrial command center for capital-scale production of **any selected build target**  dreads, FAX, carriers, supers, Titans, structures, components, anything EVE industry data can manufacture. No ship is special-cased; the Avatar Titan is the first validation scenario, driven by a generic Build Target Engine pipeline: Select Build Target → Load Blueprint Requirements → Calculate Materials → Compare Inventory → Identify Missing Inputs → Recommend Next Action.

**Intelligence, not automation.** This is not a bot. It performs no gameplay automation, never touches the EVE client, and uses only CCP's official ESI API and Static Data Export. It analyzes your industrial empire and tells you what to build next  you fly the ship. The full rules live in [docs/PRODUCT_CONSTITUTION.md](docs/PRODUCT_CONSTITUTION.md).

## Stack

Tauri 2 · React 18 · TypeScript · Rust · SQLite (local, `rusqlite` bundled). Windows-first.

## Architecture docs

| Doc | Contents |
|---|---|
| [PRODUCT_CONSTITUTION.md](docs/PRODUCT_CONSTITUTION.md) | Mission, hard prohibitions, determinism doctrine |
| [SYSTEM_ARCHITECTURE.md](docs/SYSTEM_ARCHITECTURE.md) | Layers, engines, sync design, sprint sequencing |
| [DATABASE_SCHEMA.md](docs/DATABASE_SCHEMA.md) | SQLite schema, migrations, lifecycle rules |
| [ESI_INTEGRATION.md](docs/ESI_INTEGRATION.md) | OAuth PKCE flow, scopes, cache/error-limit policy |
| [UI_ARCHITECTURE.md](docs/UI_ARCHITECTURE.md) | Layout shell, route map, Mission Control composition |
| [architecture/UI_CONSTITUTION.md](docs/architecture/UI_CONSTITUTION.md) | Design-system law: color, type, panels, states, motion |
| [SPRINT_001_IMPLEMENTATION_PLAN.md](docs/SPRINT_001_IMPLEMENTATION_PLAN.md) | This sprint's scope and acceptance criteria |

## Running locally (Windows)

Prerequisites:

1. **Node.js 20+** — https://nodejs.org
2. **Rust (stable)** — https://rustup.rs
3. **Microsoft Visual Studio C++ Build Tools** and **WebView2** (preinstalled on Windows 11) — see the Tauri v2 prerequisites guide.

Then:

```powershell
npm install

# Full desktop app (Tauri shell + SQLite backend)
npm run tauri dev

# Frontend only, in a browser with mock data (no Rust needed)
npm run dev            # http://localhost:1420

# Production bundle (installer under src-tauri/target/release/bundle)
npm run tauri build
```

On first launch the Rust core creates `rendernorth.db` in the app data directory (`%APPDATA%\com.rendernorth.industrial\` on Windows), applies migrations 0001–0008, and seeds the demo dataset. The dashboard footer shows the live data source (`sqlite` under Tauri, `mock` in a plain browser) plus the schema version from `health_check`.

## Status (Sprint 008.2 — Real EVE Data Tonight)

**Mission Control** is the landing page: Factory Health console, Current Operation panel with a working build-target selector (five demo targets — Avatar, Navy Revelation, Apostle, Capital Construction Parts, Broadcast Node — all plain data, none special-cased), per-target missing inputs, and deterministic recommendations carrying rule id + inputs. "Inventory Coverage" is now genuinely computed by the Inventory Engine rather than a stored literal.

**Inventory** is live: a real Inventory Engine (`src-tauri/src/inventory/`) backs a category-rail + table page over 47 seeded demo items across every EVE ownership category — ships, blueprints, minerals, ore, PI, components, modules, fuel, and more. Summary cards show total assets, estimated value, unique types, known locations, and the reserved/available value split.

The domain hierarchy is now: **Mission Control → Operation Engine → Reservation Engine → Inventory Engine → SQLite.**

- **Operation Engine** (`src-tauri/src/operation/`) is now real, not just an interface. RenderNorth Industrial revolves around Operations — industrial intent like "Build Avatar" or "Prepare Titan Components" — not inventory or blueprints. Reads are live: list, priority queue, blocked detection (derived from dependencies, never a hand-set flag), upcoming completions, health, and full detail, all backed by migration 0004 (`operations`, `operation_timeline`, `operation_dependencies`). Mutation methods exist on `OperationEngine` but every one currently returns an explicit "architecture-only" error — no lifecycle mutation, and Operations still cannot request a reservation.
- **Reservation Engine** (`src-tauri/src/reservation/`) is now real as of Sprint 005 — see below.
- **Decision Engine** (`src-tauri/src/decision.rs`) remains interface-only: `DecisionContext` bundles inventory + operation state for a future deterministic build/buy/mine/sell/research/copy/react rule pipeline. The live Mission Control recommendation panel (seeded `recommendations` rows) is unchanged and is this engine's working ancestor.

**Mission Control is now an Operations Dashboard.** Alongside the existing Factory Health / Current Operation (Selected Build Target) / Missing Inputs / Next Recommendation panels, it now shows Operation Health, a full Current Operations list, a Priority Queue, Blocked Operations (with blocking genuinely derived from unfinished dependencies — Avatar shows Blocked despite its own status being "active"), and Upcoming Completions. Every row links to the new **Operations Workspace** (`/operations`), where selecting an operation shows its live Overview, Goal, Priority, Status, Progress, Notes, and Dependencies — with Production, Inventory, Blueprints, Shopping, Cost, and Timeline shown only as reserved placeholder cards, no implementation behind them yet.

The Reservation Engine now sits live between Inventory and Operations: `src-tauri/src/reservation/` (a sibling module to `inventory/`/`operation/`, same four-file shape) reads reservation summaries, per-reservation history, and conflict detection — always computed fresh from current data, never a stale stored flag. Migration 0005 extends `inventory_reservations` additively (a new `operation_id` column, backfilled from the existing `project_id`) and adds `reservation_events` (append-only: reserved/released/transferred/expired) and `reservation_conflicts`.

- **Mission Control** gains an **Inventory Commitment** panel (Total Inventory, Reserved, Available, Blocked, Unallocated) and a **Reservation Conflicts** panel that surfaces exactly two conflict types found in the live demo data — Capital Construction Parts held by two operations at once, and Auto-Integrity Preservation Seals reserved past what's on hand — plus a correctly-empty third check (missing-inventory) that just hasn't been triggered by any seeded data.
- The **Operations Workspace** gains a live **Reservations** section per operation: Reserved Minerals, Reserved Components, Reserved PI, and Missing Reservations (compared against that operation's `missing_materials`, where applicable).
- The **Inventory** page's item table gains **Reserved**, **Free**, and **Available** columns alongside the existing Quantity — Free nets out soft allocations too, a stricter bar than Available.

Reservation mutations (reserve/release/transfer) are declared on `ReservationEngine` but every one returns an explicit "architecture-only" error — nothing creates, releases, or transfers a reservation yet.

The Blueprint Engine now sits live as a sibling to Inventory, Operation, and Reservation: `src-tauri/src/blueprint/` (same four-file shape) reads blueprint records, summary counts, a global missing-blueprint report, and per-operation/cross-operation readiness — blueprints are industrial capability records (BPO vs BPC, ME/TE, runs remaining, research/copy status), not just inventory items. Migration 0006 adds `blueprints` and `operation_blueprint_requirements`; the existing "blueprints" category rows in `inventory_items` (migration 0003) are untouched, with a handful of new `blueprints` rows linking back to them where a natural match exists.

- A new **Blueprints** page (top-level nav) shows Total Blueprints, BPO/BPC counts, Research Complete, Copies (total remaining BPC runs), and Missing For Operations, plus the full blueprint library table and a Missing Blueprint Report.
- **Mission Control** gains a **Blueprint Readiness** panel: owned/missing/warning counts per operation, linking through to the Operations Workspace.
- The **Operations Workspace** gains a live **Blueprints** section: required blueprints, owned count, missing count, and research/copy warnings (an owned blueprint that's currently mid-research or mid-copy — a softer signal than missing).

Blueprint mutations (start research, start copy, acquire) are declared on `BlueprintEngine` but every one returns an explicit "architecture-only" error — nothing starts a research job, starts a copy, or acquires a blueprint yet.

The Production Requirement Engine now sits live as a sibling to Inventory, Operation, Reservation, and Blueprint: `src-tauri/src/production/` (same four-file shape) answers "what does this operation actually require to complete?" Migration 0007 adds `production_requirements` (a material, a quantity, an operation), `production_requirement_groups` (structural category scope), and `production_requirement_sources` (provenance). Coverage, shortage, and cross-operation bottleneck detection are computed fresh every time — never a stored flag.

- The **Production** page is now live (previously a placeholder): five summary cards (Total Requirements, Satisfied, Missing, Coverage %, Critical Bottlenecks), a filterable category rail + requirement tree, a Critical Bottlenecks panel, and a Shortage Report.
- **Mission Control** gains a **Production Readiness** panel with the same five figures.
- The **Operations Workspace** gains a live **Production Requirements** section: per-category coverage bars (Minerals, Components, Advanced Components, PI, Reaction Materials) plus overall coverage % and missing count for the selected operation.

Production Requirement mutations (declare a requirement, adjust a required quantity) are declared on `ProductionEngine` but every one returns an explicit "architecture-only" error — nothing here does production math, shopping, manufacturing job logic, or scheduling.

**Sprint 008 adds a real, non-demo vertical slice on top of all of the above** — see `docs/REAL_PRODUCTION_PLANNER.md` for full details. In one sentence: you can now import your own static data, search for a real build target, create a real operation, get a real recursively-calculated (ME-adjusted, cycle-protected) requirement tree, compare it against inventory you actually entered, and export a real shortage list — none of it hardcoded demo data.

**Sprint 008.2 adds a second, official import path on top of the sample fixture**: Settings → Static Data now offers "Import Official CCP SDE" (JSON Lines format, tolerant field-name matching) alongside "Import Sample Fixture" — both write into the exact same reference tables through one shared transactional upsert function. **The official path could not be verified against a real CCP export in this environment** — no such file was available — so it should be treated as unverified until tested against a real download; the sample fixture remains the only fully verified import path. Also new: a **Paste Inventory** workflow (Inventory page) for bulk-entering real stock from a spreadsheet paste — tolerant of tabs, commas, multiple spaces, and comma-grouped numbers, with a match/merge preview before anything is saved — and a **Flattened Materials** view (Production page) alongside the existing Build Tree, the direct replacement for a former Excel-based shortage list.

- **Static Data Import** (new `src-tauri/src/staticdata/` module) — Settings → Static Data reads a local directory of five CSV files (the community-standard CSV-derivative SDE shape: `invCategories.csv`, `invGroups.csv`, `invTypes.csv`, `industryActivityProducts.csv`, `industryActivityMaterials.csv`). Transactional, repeatable, never touches operations/inventory/reservations/settings. A small verified fixture ships at `fixtures/sample-sde-csv/`.
- **Real operation creation** — `OperationEngine::create_operation` is upgraded from an architecture-only stub to a real implementation. Operations → New Operation lets you search a real build target (once imported), set quantity and a blueprint source (an owned Blueprint Engine record, or a planning ME/TE/BPC assumption), and persists it — marked `is_demo = 0`, clearly distinct from the seeded scenario operations (`is_demo = 1`, badged `DEMO` throughout the app).
- **Real requirement calculation** — `production::calculate_plan` recursively expands blueprint material lines with real ME rounding, run calculation, shared-subcomponent summation, cycle detection, and max-depth protection. The Production page's "Real Production Plan" panel renders the resulting tree with expand/collapse.
- **Manual inventory** — a new, real `manual_inventory_entries` table (Inventory page's "Manual Inventory" section) lets you search an imported type, enter a quantity and location, edit, and remove — entirely separate from the demo-seeded inventory.
- **Material List export** — "Material List — Prices Not Included": copy to clipboard, CSV, or Markdown, generated client-side from the calculated shortage set. No prices, ever.

**Known limitations** (fully disclosed in `docs/REAL_PRODUCTION_PLANNER.md`): CSV-derivative SDE format only (not CCP's raw YAML package); no native file/directory picker (path is typed/pasted); sub-component ME defaults to 0 unless an owned blueprint's name matches; runs aren't yet reduced by on-hand stock of intermediate components; manual inventory can't yet be reserved to an operation. **No Rust compiler was available while building this sprint** — every SQL query and the calculation algorithm were hand-verified against real SQLite before being written into Rust, but the code itself has not been compiled; run `cargo test` and `cargo tauri dev` locally to confirm.

Top-level navigation: Mission Control, **Operations (live)**, Build Targets, Inventory (live), **Blueprints (live)**, **Production (live)**, Industry, Logistics, Market Intelligence, Planning, Intelligence, Reports, **Settings (live)**. No ESI integration yet — by design.
