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

On first launch the Rust core creates `rendernorth.db` in the app data directory (`%APPDATA%\com.rendernorth.industrial\` on Windows), applies migrations 0001–0002, and seeds the demo dataset. The dashboard footer shows the live data source (`sqlite` under Tauri, `mock` in a plain browser) plus the schema version from `health_check`.

## Status (Sprint 002)

**Mission Control** is the landing page: Factory Health console, Current Operation panel with a working build-target selector (five demo targets — Avatar, Navy Revelation, Apostle, Capital Construction Parts, Broadcast Node — all plain data, none special-cased), per-target missing inputs, and deterministic recommendations carrying rule id + inputs. Selection persists in SQLite (`app_meta`) under Tauri and in memory in browser mode. Top-level navigation: Mission Control, Build Targets, Inventory, Production, Industry, Logistics, Market Intelligence, Planning, Intelligence, Reports, Settings. No ESI integration yet — by design.
