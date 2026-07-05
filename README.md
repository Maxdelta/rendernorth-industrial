# RenderNorth Industrial

Desktop-first EVE Online industrial command center for capital-scale production of **any selected build target** — dreads, FAX, carriers, supers, Titans, structures, components, anything EVE industry data can manufacture. No ship is special-cased; the Avatar Titan is the first validation scenario, driven by a generic Build Target Engine pipeline: Select Build Target → Load Blueprint Requirements → Calculate Materials → Compare Inventory → Identify Missing Inputs → Recommend Next Action.

**Intelligence, not automation.** This is not a bot. It performs no gameplay automation, never touches the EVE client, and uses only CCP's official ESI API and Static Data Export. It analyzes your industrial empire and tells you what to build next — you fly the ship. The full rules live in [docs/PRODUCT_CONSTITUTION.md](docs/PRODUCT_CONSTITUTION.md).

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

On first launch the Rust core creates `rendernorth.db` in the app data directory (`%APPDATA%\com.rendernorth.industrial\` on Windows), applies migrations 0001–0004, and seeds the demo dataset. The dashboard footer shows the live data source (`sqlite` under Tauri, `mock` in a plain browser) plus the schema version from `health_check`.

## Status (Sprint 004 — Operation Domain Foundation)

**Mission Control** is the landing page: Factory Health console, Current Operation panel with a working build-target selector (five demo targets — Avatar, Navy Revelation, Apostle, Capital Construction Parts, Broadcast Node — all plain data, none special-cased), per-target missing inputs, and deterministic recommendations carrying rule id + inputs. "Inventory Coverage" is now genuinely computed by the Inventory Engine rather than a stored literal.

**Inventory** is live: a real Inventory Engine (`src-tauri/src/inventory/`) backs a category-rail + table page over 47 seeded demo items across every EVE ownership category — ships, blueprints, minerals, ore, PI, components, modules, fuel, and more. Summary cards show total assets, estimated value, unique types, known locations, and the reserved/available value split.

The domain hierarchy is now: **Mission Control → Operation Engine → Reservation Engine → Inventory Engine → SQLite.**

- **Operation Engine** (`src-tauri/src/operation/`) is now real, not just an interface. RenderNorth Industrial revolves around Operations — industrial intent like "Build Avatar" or "Prepare Titan Components" — not inventory or blueprints. Reads are live: list, priority queue, blocked detection (derived from dependencies, never a hand-set flag), upcoming completions, health, and full detail, all backed by migration 0004 (`operations`, `operation_timeline`, `operation_dependencies`). Mutation methods exist on `OperationEngine` but every one currently returns an explicit "architecture-only" error — no lifecycle mutation, and Operations still cannot request a reservation.
- **Reservation Engine** (`src-tauri/src/inventory/reservation.rs`) remains architecture-only — the trait and schema (`inventory_reservations`) exist, nothing calls it yet.
- **Decision Engine** (`src-tauri/src/decision.rs`) remains interface-only: `DecisionContext` bundles inventory + operation state for a future deterministic build/buy/mine/sell/research/copy/react rule pipeline. The live Mission Control recommendation panel (seeded `recommendations` rows) is unchanged and is this engine's working ancestor.

**Mission Control is now an Operations Dashboard.** Alongside the existing Factory Health / Current Operation (Selected Build Target) / Missing Inputs / Next Recommendation panels, it now shows Operation Health, a full Current Operations list, a Priority Queue, Blocked Operations (with blocking genuinely derived from unfinished dependencies — Avatar shows Blocked despite its own status being "active"), and Upcoming Completions. Every row links to the new **Operations Workspace** (`/operations`), where selecting an operation shows its live Overview, Goal, Priority, Status, Progress, Notes, and Dependencies — with Production, Inventory, Blueprints, Shopping, Cost, and Timeline shown only as reserved placeholder cards, no implementation behind them yet.

Top-level navigation: Mission Control, **Operations (live)**, Build Targets, Inventory (live), Production, Industry, Logistics, Market Intelligence, Planning, Intelligence, Reports, Settings. No ESI integration yet — by design.
