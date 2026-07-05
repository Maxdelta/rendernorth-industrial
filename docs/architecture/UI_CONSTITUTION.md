# RenderNorth Industrial — UI Constitution

Version 1.1 — Sprint 003 (adds category-rail + data-table pattern for Inventory)
Status: Ratified. The design system's law. `src/styles/theme.css` implements it; where code and this document disagree, this document wins and the code is a bug.

## 1. What the UI should feel like

**NASA mission console × Battlestar Galactica CIC × EVE industrial** — without copying CCP. An operator sits at this screen and runs a capital shipyard: dense telemetry, calm authority, machined surfaces, deliberate light. Information is instrumentation. Every glowing element earns its glow by carrying state.

The test for any new screen: *could this stay open on a second monitor for six hours and still feel like a command deck, not a webpage?*

## 2. What the UI must avoid

- **Generic SaaS dashboard** — no rounded white cards, no pastel gradients, no cheerful illustration, no floating shadows on everything.
- **Spreadsheet look** — tables exist, but never as the identity of a screen; data gets instrumentation (gauges, keels, states), not just rows.
- **Neon gamer RGB** — no color cycling, no saturated rainbow accents, no glow for glow's sake. One energy color (furnace amber), used with discipline.
- **Overdone animation** — nothing loops for decoration, nothing bounces, nothing parallaxes. Motion communicates a change of state or it doesn't exist.
- **CCP trade dress** — no EVE fonts, icons, hull art, or UI reproductions. Inspired mood only.

## 3. Color

Defined once in `theme.css`. No component may introduce a hex value.

| Token | Hex | Law |
|---|---|---|
| `--void` | `#07090D` | App background. Space. Nothing else may be darker. |
| `--deck` | `#0D141D` | Content wells, recessed areas, sidebar. |
| `--plate` | `#121B27` | Panel surface. The default "material." |
| `--plate-hi` | `#182434` | Hover / raised surface. |
| `--bulkhead` | `#24354A` | Every border, rule, divider, and grid line. |
| `--furnace` | `#F2A33C` | **The** accent. Energy, action, attention, progress, selection. |
| `--coolant` | `#4EC6DC` | Telemetry, informational data, links. Never for actions. |
| `--nominal` | `#62C97F` | OK / complete / healthy. |
| `--alert` | `#E2554F` | Blocked / critical / missing. |
| `--text` | `#D9E1EA` | Primary text. |
| `--text-dim` | `#7E8FA3` | Labels, captions, secondary. |

**The state language is constitutional:** amber = needs attention or carries energy, cyan = data, green = nominal, red = blocked. A color may never mean something different in two places. Color is never the *only* channel — every state pairs with a text label (accessibility floor).

## 4. Typography

| Role | Face | Rules |
|---|---|---|
| Display / headings / nav / buttons | Rajdhani 600–700 | Always uppercase, +0.06 to +0.18em tracking. Signage, not prose. |
| Telemetry (numbers, ISK, %, ids) | IBM Plex Mono 400–600 | Every number on a dashboard is mono. No exceptions. |
| Body / descriptions / reasons | System sans stack | Quiet sentence case. The only place lowercase lives. |

Type scale (px): 10/11 mono captions · 12–13 labels · 13.5–15 body · 18–26 panel/target names · 30–44 hero numerics. Do not invent sizes between these without amending this doc.

## 5. The chamfered plate (panel law)

Every panel is the signature **chamfered plate**: `clip-path` with 10px cut corners (top-right + bottom-left), 1px `--bulkhead` border, `--plate` surface, and a **3px status keel** on the left edge carrying the panel's state color.

- Keel colors follow the state language. A neutral panel keeps a `--bulkhead` keel.
- An `alert` keel may add a faint interior red wash (`inset` shadow ≤ 5% alpha). That is the maximum permitted "drama."
- No border-radius anywhere. No drop shadows for elevation. Depth comes from surface tokens (`--deck` recesses, `--plate-hi` raises).
- Panel titles: Rajdhani, dim, tracked, uppercase, top-left. One title per panel.

## 6. Progress & gauges

- **Tier gauges** (per-requirement bars): flat track on `--deck` with `--bulkhead` border; fill color derives from coverage — ≥100% `--nominal`, ≥60% `--furnace`, below `--alert`. Percentage in mono at the right, same color as the fill.
- **Segmented gauge** (hero progress): discrete segments, lit in `--furnace` with ≤ 0.45-alpha glow. Segments read as machinery, not a loading bar.
- **Hull schematic**: floods bottom-up in `--furnace` behind a `--coolant` outline. Silhouettes may vary by target *category* in later sprints — never per individual hull (no-special-case rule applies to art too).
- Every gauge is an ARIA progressbar with value and label.

## 7. Warning & critical states

| State | Trigger (deterministic) | Presentation |
|---|---|---|
| Nominal | metric at/inside target | green keel/value + label |
| Attention | idle capacity > 0, coverage 60–99%, stale data | amber keel/value + label. Idle capacity is opportunity, not failure — amber, never red. |
| Critical | blocked jobs > 0, missing inputs > 0, coverage < 60% | red keel/value + label |

Factory status derivation is code + constitution: `blocked_jobs > 0 → Blocked`; else `health < 60% → Degraded`; else `Operational`. Changing thresholds requires amending both.

## 8. Buttons & controls

- **Action button** (`.target-select` pattern): transparent fill, 1px `--furnace` border, furnace Rajdhani label, uppercase; hover = `--furnace-dim` wash. Actions are amber because action = energy.
- **Disabled**: `--text-dim` label, `--bulkhead` border, `not-allowed` cursor, and a `title` explaining *when* it activates. Disabled controls are promises, not decoration.
- Destructive actions (future) use `--alert` borders and require confirmation.
- No filled/solid primary buttons — solid amber blocks would out-shout the telemetry.
- Labels are verbs that say what happens: "Change target", "Select", "Sync now". Never "Submit", "OK".

## 9. Spacing & layout

- 4px base grid; panels compose on an 8px rhythm; dashboard grid gap is 14px.
- Panel padding: 14–16px, +3px left for the keel.
- Dashboards are CSS grid: 4-column base; stat plates span 1, consoles/heroes span 4, detail panels span 2. Collapse to 2/1 columns below 1100px.
- Density is a feature: prefer one screen without scrolling for Mission Control at 1440×900.

## 10. Motion (subtle animation rules)

- Permitted: progress fills easing on mount/update (≤ 700ms, decel curve); hover surface shifts (instant to 150ms); a single slow ambient effect per screen maximum (e.g. the schematic's future scanline, ≥ 8s period, ≤ 10% opacity).
- Forbidden: looping decorative animation, pulsing text, spinners longer than 300ms without a status message, anything that moves in a user's peripheral vision while they read.
- `prefers-reduced-motion: reduce` collapses everything to final states. Non-negotiable.

## 10a. Category rail + table (Inventory pattern, Sprint 003)

- The category rail is a vertical list of plain buttons inside a chamfered plate, not a second sidebar — one active state at a time, furnace left-border + wash exactly like the main nav's active state, for visual consistency between navigation levels.
- Item tables are rows, not `<table>` grids with borders on every cell — a header row in dim tracked Rajdhani, data rows separated by single `--bulkhead` hairlines, no zebra striping (that reads as spreadsheet).
- Status text in a table is mono, uppercase, tracked, and colored by the same state law as everywhere else (nominal/furnace/alert/coolant) — never a colored pill/badge shape, which would look like a SaaS table.
- Numeric columns (quantity, reserved) are right-aligned mono; text columns (location, owner) truncate with ellipsis rather than wrap, to keep row height constant.

## 11. Dashboard panel rules

1. A panel answers one operator question; its title is that question's subject ("Factory Health", "Missing Inputs", "Current Operation").
2. Every number visible on a dashboard must be traceable to data (DTO field → table/metric). No cosmetic numbers.
3. Warning and critical states must be reachable by real data paths — if a panel can't turn red, it doesn't need a keel.
4. Recommendations always render title + reason + rule id + inputs. A recommendation without its rule id is a bug (Determinism Doctrine).
5. Empty states instruct: what this panel will show and what unlocks it.
6. Every panel must degrade gracefully to stale data with an "as of" affordance once sync exists.

## 12. Voice

Interface copy is operator-brief: plain verbs, sentence case in body text, uppercase only via the display face. No exclamation marks, no marketing tone, no apologies in errors — state what happened and what to do. The app never says "please" and never says "oops."

## Amendments

*(none yet — changes require an entry here)*
