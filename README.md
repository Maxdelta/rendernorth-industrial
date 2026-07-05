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

On first launch the Rust core creates `rendernorth.db` in the app data directory (`%APPDATA%\com.rendernorth.industrial\` on Windows), applies migrations 0001–0003, and seeds the demo dataset. The dashboard footer shows the live data source (`sqlite` under Tauri, `mock` in a plain browser) plus the schema version from `health_check`.

## Status (Sprint 003.5 — architecture refinement)

**Mission Control** is the landing page: Factory Health console, Current Operation panel with a working build-target selector (five demo targets — Avatar, Navy Revelation, Apostle, Capital Construction Parts, Broadcast Node — all plain data, none special-cased), per-target missing inputs, and deterministic recommendations carrying rule id + inputs. "Inventory Coverage" is now genuinely computed by the Inventory Engine rather than a stored literal.

**Inventory** is live: a real Inventory Engine (`src-tauri/src/inventory/`) backs a category-rail + table page over 47 seeded demo items across every EVE ownership category — ships, blueprints, minerals, ore, PI, components, modules, fuel, and more. Summary cards show total assets, estimated value, unique types, known locations, and the reserved/available value split.

The domain hierarchy is now: **Mission Control → Operation Engine → Reservation Engine → Inventory Engine → SQLite.**

- **Operation Engine** (new, `src-tauri/src/operation.rs`) is a first-class architecture concept: an Operation owns its goal, deadline, priority, notes, selected build target, timeline, and production plan — and nothing else. It never owns inventory; it only requests reservations. Interface-only — no schema change, `build_projects` remains the live (partial) backing table.
- **Reservation Engine** (`src-tauri/src/inventory/reservation.rs`) is the seam an Operation calls through to hold inventory. Architecture-only — the trait and schema (`inventory_reservations`) exist, nothing calls it yet.
- **Decision Engine** (renamed from Recommendation Engine, `src-tauri/src/decision.rs`) answers build/buy/mine/sell/research/copy/react questions from a `DecisionContext` bundling inventory + operation state. Deterministic and rule-based; LLMs may explain a decision later, never produce one. Interface-only — the live Mission Control recommendation panel (seeded `recommendations` rows) is unchanged and is this engine's working ancestor.

Top-level navigation: Mission Control, Build Targets, Inventory (live), Production, Industry, Logistics, Market Intelligence, Planning, Intelligence, Reports, Settings. No ESI integration yet — by design.
