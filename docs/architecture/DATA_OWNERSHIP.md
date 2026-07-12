# Data Ownership Model

Canonical since Sprint 010 (Demo Data Retirement). Every table in RenderNorth
Industrial's schema belongs to exactly one of five ownership layers. New
tables should be classified into one of these when they're added — if a
table doesn't obviously fit, that's a signal to reconsider its design
before adding it, not a reason to invent a sixth layer.

## The five layers

### 1. Vocabulary Data
Fixed label/taxonomy definitions. Not instance data, not "real" or "demo"
— just the domain's controlled vocabulary. Never per-install-seeded
differently, never demo-flagged, never touched by any cleanup mechanism,
present in every install (fresh or upgraded) identically and permanently.

- `inventory_categories`, `inventory_states`
- Value-domain conventions that aren't even separate tables: operation
  statuses (`planned`/`active`/`blocked`/`paused`/`completed`), blueprint
  statuses (BPO/BPC, research/copy states), reservation concepts
  (`reason`, conflict types) — these are free-text conventions on real
  rows, not seeded rows themselves, so there's nothing to seed or clean.

**Test**: if you're tempted to ask "is this row real or demo?", it's not
vocabulary — vocabulary rows don't have that question.

### 2. Reference Data
Authoritative external data, imported by explicit user action, never
auto-seeded. Real from the moment it exists; there is no demo version.

- `eve_categories`, `eve_groups`, `eve_types`, `blueprint_products`,
  `blueprint_materials`, `sde_imports` — populated only by importing an
  official or sample CCP SDE export (Settings page).

### 3. User Data
Everything the user directly creates or owns. Never touched by any
cleanup mechanism, ever, under any circumstances — this is the one layer
where "destructive" is never an acceptable adjective.

- `operations` (`is_demo = 0`), `operation_build_targets`,
  `manual_inventory_entries`, `blueprints` (`is_demo = 0`),
  `inventory_reservations`/`reservation_events` tied to a real operation,
  `app_meta` (settings).

### 4. Runtime Data
Ephemeral application bookkeeping — about the app's own operation, not
about the domain. Not a feature. Not read by any command, query, or UI
outside its own narrow bootstrap purpose.

- `schema_migrations` — permanent, append-only, the source of truth for
  schema version. Never a demo/real distinction; it doesn't describe
  domain data at all.
- `install_state` — transient, self-clearing. See the Fresh-Install
  Lifecycle section below. On a healthy install this table's one relevant
  row exists for milliseconds within a single startup and is then gone.

### 5. Derived Data
Values computed from other data at query/request time, never stored as a
source of truth. If it were deleted and recomputed, nothing would be
lost — recomputing *is* the correct behavior, not a fallback.

- `is_blocked` (operation status derivation), `coverage_fraction`,
  `missing_quantity`, blueprint readiness counts, reservation conflict
  detection, the entire recursive production-requirement tree
  (`calculate_plan`'s output) — all computed fresh on every call.
- `requirement_calculation_snapshots` is a deliberate, narrow exception:
  an optional persisted audit record of a derived calculation at a point
  in time, not a cache the app trusts instead of recomputing.

## Where demo/example data fits

**It doesn't — not as a sixth layer, and not inside the five above as a
first-class citizen.** Demo data (the Sprint 001–007 seeded scenario:
`operations` with `is_demo = 1`, `inventory_items` with
`source = 'demo'`, demo `blueprints`, the entire Sprint 007
`production_requirements` ledger, `build_projects` and its cascade
children, `factory_snapshot`, demo `characters`) historically lived
*inside* User-Data-shaped tables, distinguished by a flag. As of Sprint
010 it is retired from normal runtime entirely:

- Removed from every normal query, list, dashboard, and summary (backend
  filtering — `WHERE is_demo = 0` / `WHERE source != 'demo'` at the
  repository layer, not client-side toggling).
- Removed from a fresh install's actual data, via the fresh-install
  cleanup below — a brand-new database ends with zero demo rows.
- Preserved only as developer/test fixture data — hand-built inline in
  Rust test functions (see `operation::repository::tests` and
  `production::repository::tests` for the current pattern), not relied
  upon via incidental migration-chain content.
- An upgraded install's pre-existing demo rows are left physically in
  place (never destructively purged from an existing database file) but
  are invisible and non-contributing everywhere in normal use.

## Fresh-Install Lifecycle (`install_state`)

The full lifecycle of the one mechanism that tells a fresh install from
an upgraded one. This is Runtime Data (layer 4) — read this section
before touching it, since its entire value depends on staying exactly
this narrow.

| Event | What happens | Where |
|---|---|---|
| Created | `CREATE TABLE IF NOT EXISTS install_state (key TEXT PRIMARY KEY, value TEXT NOT NULL)` — every call to `migrate()`, idempotent, same pattern as `schema_migrations` itself. Not a numbered `.sql` migration. | Start of `db.rs::migrate()` |
| Written | Only when `current == 0` (this database file has never had a migration applied before this exact run): `INSERT OR IGNORE INTO install_state (key, value) VALUES ('demo_cleanup_pending', '1')`. At most once, ever, per database file. | `migrate()`, before migration 1 runs |
| Migrations run | Completely unaffected — `install_state` is not read or written during the migration loop. `schema_migrations` remains the sole source of truth for schema version. | `migrate()`, unchanged |
| Read | `SELECT value FROM install_state WHERE key = 'demo_cleanup_pending'` — on **every** call to `migrate()`, not just the first. | `migrate()`, after the migration loop |
| Cleanup dispatched | Only if the row above exists. | Same function |
| Cleared | Only after cleanup succeeds. | Same function |
| Ignored | For every database where `current > 0` the first time this code ever runs against it — the marker is simply never written, so nothing downstream ever activates. | N/A — the absence is the mechanism |

**What it is not:**
- Not a replacement for `schema_migrations` — schema version truth stays
  exactly where it's always been.
- Not consulted anywhere outside `db.rs::migrate()` — no command, no
  query, no UI reads it. It has no runtime feature meaning.
- Not a general "demo mode" flag. Whether a given row is demo or real is
  determined solely by that row's own `is_demo`/`source` column, always
  — never inferred from `install_state`'s presence or absence.
- Not permanent. Its value on a healthy, uninterrupted fresh install is
  a few milliseconds of existence within one startup, then gone forever.

**Crash safety**, in every scenario:
- Crash before the marker write: it's written on the very next call.
- Crash mid-migration-chain: the marker (already durably on disk from
  the earliest point) survives; migrations resume from wherever
  `schema_migrations` says they left off (pre-existing behavior); once
  they finish, cleanup runs.
- Crash after the last migration, before cleanup: marker is untouched,
  found and acted on immediately next startup.
- Crash mid-cleanup: marker is only cleared *after* cleanup succeeds, so
  it's still present; next startup retries. Every statement in
  `run_demo_seed_cleanup` is a `DELETE ... WHERE ...` — deleting
  already-gone rows is a no-op, so retrying is always safe.
- Upgraded database: the marker was never written in the first place
  (`current > 0` the first time this code sees it), so there is nothing
  to resume, retry, or accidentally trigger.

## Build Targets — a deliberately unresolved case

`build_projects` and its schema (the "Build Targets" concept) don't
map cleanly onto the five layers above: they were designed to eventually
hold Reference-Data-linked rows (`target_type_id` was always meant to
point at real imported types) but currently only ever hold demo/example
rows, with a legacy UI (`/targets`) that's been retired. The concept and
underlying schema are retained, unrouted, pending a product decision on
whether it becomes a planning sandbox, quick estimator, production
preview, or pre-operation workflow — at which point it will resolve into
User Data (if it starts persisting something the user creates) or stay
closer to Derived Data (if it remains a non-persistent preview). Noted
here explicitly so it isn't miscategorized by omission.
