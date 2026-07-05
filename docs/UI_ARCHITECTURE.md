# RenderNorth Industrial — UI Architecture

Version 1.1 — Sprint 002 (Mission Control rename, top-level nav model). Design-system law lives in docs/architecture/UI_CONSTITUTION.md.

## 1. Target feeling

"I am running a capital shipyard from a command deck." Any manufacturable target, Titans included. Dark, industrial, military. Dense with telemetry but legible at a glance from a second monitor. EVE-inspired mood; zero CCP art, icons, fonts, or trade dress.

Explicitly not: a spreadsheet, a generic admin template, or neon-hacker green-on-black.

## 2. Design tokens (single source: `src/styles/theme.css`)

### Palette
| Token | Hex | Role |
|---|---|---|
| `--void` | #07090D | app background (space) |
| `--deck` | #0D141D | content wells |
| `--plate` | #121B27 | panel surface |
| `--plate-hi` | #182434 | raised surface / hover |
| `--bulkhead` | #24354A | borders, rules, grid lines |
| `--furnace` | #F2A33C | primary accent: energy, action, progress |
| `--coolant` | #4EC6DC | telemetry, links, informational |
| `--nominal` | #62C97F | OK state |
| `--alert` | #E2554F | critical / blocking |
| `--text` | #D9E1EA | primary text |
| `--text-dim` | #7E8FA3 | labels, secondary |

State language everywhere: **amber = attention/energy, cyan = data, green = nominal, red = blocked.** A panel's left "status keel" bar carries its state color.

### Typography
| Role | Face | Treatment |
|---|---|---|
| Display / headings / nav | **Rajdhani** (600–700) | uppercase, +0.08em tracking — squared military signage |
| Telemetry numbers | **IBM Plex Mono** | tabular figures for counts, ISK, percentages |
| Body / descriptions | system sans stack | quiet, unstyled prose |

Sprint 001 loads faces from Google Fonts with full system fallbacks; a later sprint vendors the font files into the bundle so the app is offline-clean.

### Geometry
- 4px spacing grid; panels on an 8px rhythm.
- **Signature element — the chamfered plate:** every panel is a `clip-path` polygon with one or two 10px cut corners plus a 3px status keel on the left edge, like stenciled equipment plating. This one device, used consistently, is what makes the app recognizably RenderNorth. Everything else stays quiet.
- No border-radius elsewhere; hairline `--bulkhead` rules; restrained glow (`box-shadow` in accent color at low alpha) only on live/alert elements.

### Motion
- Progress bars ease on mount (600ms); a slow 8s "scan" shimmer may run across the Titan schematic only.
- Everything honors `prefers-reduced-motion: reduce` (animations collapse to final state).

## 3. Layout shell

```
┌────────┬──────────────────────────────────────────────┐
│        │ TOPBAR  route title · sync freshness · clock │
│  NAV   ├──────────────────────────────────────────────┤
│  RAIL  │                                              │
│        │            ROUTE OUTLET (module page)        │
│ 220px  │                                              │
│        │                                              │
└────────┴──────────────────────────────────────────────┘
```

- **Nav rail** is a flat top-level model. Assets and Blueprints are no longer product identities — they become views inside Inventory and Production respectively: Mission Control · Build Targets · Inventory · Production · Industry · Logistics · Market Intelligence · Planning · Intelligence · Reports · Settings.
- **Topbar** shows the active module, data freshness ("DEMO DATA" badge while demo seeds drive the UI), and UTC clock (EVE time).

## 4. Route map (Sprint 002)

| Path | Page | Sprint 002 state |
|---|---|---|
| `/` | Mission Control (landing) | Full implementation on demo data |
| `/targets` | Build Targets | Working: list + selection drives Mission Control |
| `/inventory` | Inventory (Assets, Stockpiles, Locations, Asset Safety views) | Placeholder |
| `/production` | Production (Blueprints, Planner, Buildable-today views) | Placeholder |
| `/industry` | Industry (Jobs, Slots, Facilities views) | Placeholder |
| `/logistics` | Logistics (Shopping List, Hauling, Recovery views) | Placeholder |
| `/market` | Market Intelligence (Wallet, Orders, Prices views) | Placeholder |
| `/planning` | Planning (Build Tracker, Critical Path views) | Placeholder |
| `/intelligence` | Intelligence (Recommendation feed, Rule registry views) | Placeholder |
| `/reports` | Reports (Profit, History views) | Placeholder |
| `/settings` | Settings (Characters/ESI, Data, Preferences views) | Placeholder |

Placeholders share one `PlaceholderPage` component: module name, mission statement, planned capabilities, target sprint. Empty screens are invitations, not dead ends.

## 5. Mission Control composition

1. **Status strip** — four chamfered stat plates: Running Jobs, Idle Characters, Idle BPOs, Wallet. Idle counts > 0 carry amber keels (idle capacity is attention, not failure).
1b. **Factory Health console** — full-width seven-cell instrument row: Health %, derived Status (Operational/Degraded/Blocked), Idle Slots, Blocked Jobs, Missing Inputs (count from the selected target's gap list), ISK Locked in Jobs, Projected Finish. Keel follows derived status.
2. **Current Operation panel (hero)** — works for any selected target: "Selected Build Target" label, target name and class, a working "Change target" selector (inline picker; selection persists to app_meta under Tauri, in-memory in browser mode), overall progress as a large segmented gauge, an abstract hull elevation schematic that fills with `--furnace` as progress rises, and the requirement-tier gauges (labels come from data, not hardcoded) with state-colored keels.
3. **Missing materials panel** — the blocking list, quantities in mono, category tags; red keel because it blocks the build.
4. **Recommendation panel** — next deterministic action with its reason, rule id, and the machine-readable inputs the rule fired on; amber keel; visually a command, not a suggestion.

## 6. Frontend architecture rules

- Pages render DTOs from `src/lib/backend.ts` — the only file allowed to call Tauri `invoke`. In a plain browser (vite dev without Tauri) it falls back to `src/data/mock.ts`, so UI work never requires the Rust build.
- No business math in components beyond formatting (ISK abbreviation, percent rounding).
- Shared primitives live in `src/components`: `Panel` (chamfered plate), `StatCard`, `TierGauge`, `Sidebar`, `TopBar`, `PlaceholderPage`.
- State: React local state + loader effects for now. Introduce a store only when cross-page live sync state exists (Sprint 003), not before.

## 7. Accessibility floor

Keyboard-visible focus (amber outline), color never the only state channel (keels pair with text labels), WCAG-AA contrast on all text tokens against their surfaces, reduced-motion respected.
