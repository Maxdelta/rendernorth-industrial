# RenderNorth Industrial — Database Schema

Version 1.7 — Sprint 008 (migration 0008: Real Production Planner — static reference data, operation_build_targets, manual_inventory_entries, production_plan_snapshots, additive operations.is_demo)

**Sprint 003.5 note:** no migration this sprint. The Operation Engine and
Decision Engine (renamed from Recommendation Engine) were introduced as
architecture-only interfaces — see SYSTEM_ARCHITECTURE.md. The
`recommendations` table below is unchanged and remains live; conceptually
it is the working ancestor of what the Decision Engine will eventually
produce, not yet renamed or restructured, to avoid touching a working
feature ahead of the engine that will actually replace it.
Engine: SQLite (WAL mode, foreign keys ON). Migrations are numbered `NNNN_name.sql`, embedded in the Rust binary, applied in order, tracked in `schema_migrations`.

## Conventions

- All IDs from EVE (type_id, character_id, item_id, location_id, job_id) are stored as `INTEGER` exactly as ESI provides them.
- Timestamps are ISO-8601 UTC `TEXT`.
- ISK amounts are `REAL` for MVP (revisit as integer 1/100 ISK if precision issues appear).
- `sde_*` tables are rebuilt from the Static Data Export; `esi_*`-sourced tables are replaced per sync; `app_*`/project tables are user data and never bulk-replaced.

## Live tables (migrations 0001–0008)

### schema_migrations
| column | type | notes |
|---|---|---|
| version | INTEGER PK | migration number |
| applied_at | TEXT | |

### app_meta
Key/value store for install-level settings. Known keys: `seed_version`, `selected_project_id` (which build target Mission Control focuses on — written by the `select_build_target` command).
| column | type |
|---|---|
| key | TEXT PK |
| value | TEXT |

### characters
| column | type | notes |
|---|---|---|
| character_id | INTEGER PK | ESI character id (demo rows use negative ids) |
| name | TEXT NOT NULL | |
| corporation | TEXT | display only for now |
| is_demo | INTEGER NOT NULL DEFAULT 0 | seed rows flagged, purged when real auth lands |
| added_at | TEXT | |

### build_projects
A tracked construction goal for **any** selected build target — ship, structure, or component. Nothing in the schema is Avatar-specific; the seeded Avatar row is a demo scenario.
| column | type | notes |
|---|---|---|
| project_id | INTEGER PK AUTOINCREMENT | |
| name | TEXT NOT NULL | user label, e.g. "Avatar #1" or "Fortizar for staging" |
| target_type_id | INTEGER | FK to sde_types once the SDE lands (NULL for demo rows) |
| target_type_name | TEXT NOT NULL | display name until sde_types exists, then derived |
| status | TEXT NOT NULL | planned / active / paused / done |
| overall_progress | REAL NOT NULL | 0–1; Sprint 001 seeds this, later computed |
| created_at | TEXT | |

### build_requirement_groups
Progress per material tier of a project (drives the dashboard gauges).
| column | type | notes |
|---|---|---|
| group_id | INTEGER PK AUTOINCREMENT | |
| project_id | INTEGER NOT NULL FK → build_projects | |
| label | TEXT NOT NULL | Minerals / Capital Components / Advanced Components / PI |
| coverage | REAL NOT NULL | 0–1 |
| sort_order | INTEGER NOT NULL | |

### missing_materials
Sprint 001: seeded gap list. Later: output table of the Shopping Engine.
| column | type | notes |
|---|---|---|
| id | INTEGER PK AUTOINCREMENT | |
| project_id | INTEGER FK | |
| type_name | TEXT NOT NULL | later joins sde_types |
| quantity | INTEGER NOT NULL | |
| category | TEXT | mineral / component / pi |

### factory_snapshot
One-row-per-metric snapshot behind the Factory Status dashboard. Later sprints compute these from live tables; the DTO shape stays stable.
| column | type | notes |
|---|---|---|
| metric | TEXT PK | running_jobs, idle_characters, idle_bpos, wallet_isk, factory_health, idle_slots, blocked_jobs, isk_locked_in_jobs, projected_finish_days |
| value | REAL NOT NULL | |
| as_of | TEXT NOT NULL | |

### recommendations
*(conceptually the seed of the future Decision Engine — see SYSTEM_ARCHITECTURE.md §Domain Hierarchy; table name and shape unchanged this sprint)*
| column | type | notes |
|---|---|---|
| id | INTEGER PK AUTOINCREMENT | |
| project_id | INTEGER FK | |
| title | TEXT NOT NULL | "Start Capital Construction Parts" |
| reason | TEXT NOT NULL | JSON: rule id, inputs, thresholds (Determinism Doctrine) |
| priority | INTEGER NOT NULL | 1 = highest |
| created_at | TEXT | |
| dismissed_at | TEXT | |

## Planned tables (documented now, migrated when their sprint lands)

### Inventory Engine (migration 0003, live now)

- **inventory_states**(key PK, label, sort_order) — controlled vocabulary: available, reserved, allocated, manufacturing, research, reaction, in_transit, asset_safety, contract, delivery, destroyed. Every `inventory_items` row references one; most states are placeholders with no logic yet.
- **inventory_categories**(key PK, label, sort_order) — 17 categories (ships, blueprints, minerals, ore, compressed_ore, ice, ice_products, pi, reaction_materials, components, capital_components, advanced_components, modules, charges, fuel, structures, deployables). Categories are views over one inventory, never separate stores.
- **inventory_locations**(location_id PK, name, kind, system_name, region_name) — `kind` is free text (station/structure/asset_safety/contract today) so new location kinds never require a schema change.
- **inventory_items**(item_id PK, type_name, category_key FK, quantity, location_id FK, character_id FK, corporation_id, container_item_id FK self, contract_id, delivery_id, state FK inventory_states, unit_value, source, synced_at) — the single inventory truth. `corporation_id`/`container_item_id`/`contract_id`/`delivery_id` are present and nullable now so future corp ownership, containers, contracts, and deliveries slot in without another migration touching this table's shape.
- **inventory_allocations**(id PK, item_id FK, project_id FK build_projects, quantity, created_at) — soft, plan-level earmarking of inventory against an operation. Does not reduce availability.
- **inventory_reservations**(id PK, item_id FK, project_id FK build_projects nullable, **operation_id FK operations nullable — added additively by migration 0005**, quantity, reason, created_at, released_at) — hard hold reducing available quantity while `released_at` is unset. `project_id` is left in place for backward compatibility; `operation_id` is the column the Reservation Engine reads and writes going forward (backfilled from `project_id` for existing rows, since operations 1–5 share that id space). The engine that creates/releases these rows (`ReservationEngine`) has live reads as of Sprint 005; mutation methods exist but all return an "architecture-only" error — no command creates or releases a reservation yet.

### Reservation domain (migration 0005, live now)

- **reservation_events**(id PK, reservation_id FK inventory_reservations, event_type, quantity, from_operation_id FK operations nullable, to_operation_id FK operations nullable, reason, created_at) — append-only history. `event_type` is reserved / released / transferred / expired. History only; nothing writes to this table automatically.
- **reservation_conflicts**(id PK, item_id FK inventory_items, conflict_type, description, detected_at) — `conflict_type` is overlapping_operations / exceeds_stock / missing_inventory. Seeded rows are illustrative snapshots; the live `ReservationEngine::conflicts()` query recomputes all three checks fresh from current reservation + inventory state on every read rather than trusting this table — same "derive, don't trust a stale flag" precedent as `is_blocked` and `coverage_for_operation`. This table exists so a future scan can persist detection history without changing the read shape.

### Operation domain (migration 0004, live now)

- **operations**(operation_id PK, goal, target_type_name nullable, priority, status, progress, notes, deadline nullable, created_at, updated_at) — an Operation is industrial intent ("Build Avatar", "Prepare Titan Components"); `target_type_name` is nullable because a goal need not name a single build target. Shares its id space with `build_projects.project_id` for operations 1–5 this sprint (see SYSTEM_ARCHITECTURE.md, Domain Hierarchy); operation 6 has no `build_projects` counterpart.
- **operation_timeline**(id PK, operation_id FK, label, status, sort_order, target_date nullable) — ordered milestones. Repository-complete; the Operations Workspace UI does not render these yet (shown as a reserved placeholder).
- **operation_dependencies**(id PK, operation_id FK, depends_on_operation_id FK, reason) — one operation can depend on another. `is_blocked` is always derived from this table (an operation's own `status = 'blocked'`, OR anything it depends on isn't `completed`) — never a hand-set literal read directly off a row.

### Blueprint domain (migration 0006, live now)

- **blueprints**(blueprint_id PK, type_name, is_copy, me_level, te_level, runs_remaining nullable, character_id FK characters, location_id FK inventory_locations, status, inventory_item_id FK inventory_items nullable, created_at, updated_at) — a blueprint as an industrial capability record, not just an inventory item. `is_copy` distinguishes BPO (0, infinite runs, `runs_remaining` NULL) from BPC (1, finite `runs_remaining`). `status` is idle / researching / copying / in_use. `inventory_item_id` links back to the coexisting simple row in `inventory_items` where one naturally exists (migration 0003's "blueprints" category) — nullable, since several Sprint 006 blueprints (e.g. the Navy Revelation Blueprint) are new records with no such counterpart.
- **operation_blueprint_requirements**(id PK, operation_id FK operations, type_name, required_me nullable, required_te nullable, reason) — what an operation needs, independent of whether it's owned. Missing-blueprint detection and research/copy warnings are always a live comparison against this table and `blueprints`, never a stored flag — same "derive, don't trust a stale flag" precedent as `is_blocked` and `ReservationEngine::conflicts`.

### Production Requirement domain (migration 0007, live now)

- **production_requirements**(id PK, operation_id FK operations, category_key FK inventory_categories, type_name, required_quantity, created_at) — the concrete ledger: what quantity of a specific material an operation requires. `type_name` matches `inventory_items.type_name` the same way `missing_materials` and `operation_blueprint_requirements` already do; owned/shortage/coverage are computed by joining on it live, never stored here.
- **production_requirement_groups**(id PK, operation_id FK operations, category_key FK inventory_categories) — structural scope only: which categories are declared relevant to an operation. Never a source of coverage numbers — an operation can declare it needs Reaction Materials before a single line item is itemized.
- **production_requirement_sources**(id PK, requirement_id FK production_requirements, source_kind, contributed_quantity, note) — provenance: why a requirement's quantity was justified. A requirement can in principle have more than one contributing source (a blueprint material line plus a manual buffer); this sprint seeds exactly one per requirement.

Coverage, shortage, and cross-operation "critical bottleneck" detection (a material required by 2+ operations, unmet in at least one) are always computed fresh — same "derive, don't trust a stale flag" precedent as `is_blocked`, `ReservationEngine::conflicts`, and the Blueprint Engine's missing-blueprint report.

### Static Data Import + real planner (migration 0008, live now)

- **sde_imports**(id PK, source_path, format, source_build nullable, imported_at, status, type_count, group_count, category_count, blueprint_count, material_count, error_summary nullable) — append-only audit log of every import run. Reference tables below are upserted/replaced in place, never versioned per-import. `format` distinguishes source: `csv_bundle` (sample fixture) or, as of Sprint 008.2, `jsonl_official` (official CCP SDE) — no schema change was needed to add the second format, since `format` was already a free-text column.
- **eve_categories**(category_id PK, name, published) / **eve_groups**(group_id PK, category_id FK, name, published) / **eve_types**(type_id PK, name, group_id FK, published, is_manufacturable, volume_m3 nullable) — the game's own taxonomy, imported from the local CCP SDE. `volume_m3` is the single source for item, stack, requirement, shortage, shopping, and quote volume; a missing CCP value remains unknown. These tables are entirely separate from `inventory_categories` (migration 0003), which is RenderNorth's ownership-bucket vocabulary — the two are never merged.
- **blueprint_products**(blueprint_type_id, product_type_id FK eve_types, quantity) / **blueprint_materials**(id PK, blueprint_type_id, material_type_id FK eve_types, quantity) — manufacturing-activity blueprint data. `is_manufacturable` on `eve_types` is denormalized at import time from `blueprint_products`.
- **operation_build_targets**(operation_id PK FK operations, type_id FK eve_types, quantity_requested, blueprint_mode, owned_blueprint_id FK blueprints nullable, selected_blueprint_source nullable, manual_blueprint_id FK blueprints nullable, character_blueprint_character_id nullable, character_blueprint_item_id nullable, assumed_me, assumed_te, assumed_is_bpc, assumed_runs nullable, created_at, updated_at) — the real build target, quantity, and source-aware selected blueprint for one operation. `owned_blueprint_id` remains for compatibility with pre-0019 manual selections; new ESI selections use the character/item composite identity and are revalidated against the replaceable synchronized snapshot.
- **manual_inventory_entries**(id PK, type_id FK eve_types nullable, quantity, location_name, created_at, updated_at) — real, user-entered inventory, kept entirely separate from the demo-seeded `inventory_items`.
- **production_plan_snapshots**(id PK, operation_id FK operations, sde_import_id FK sde_imports nullable, calculated_at, requested_quantity, inputs_json, result_json) — optional audit record of a calculated plan. Never the source of truth for a live view, which always recalculates.
- **operations.is_demo** (additive `ALTER TABLE`, same pattern as migration 0005's `inventory_reservations.operation_id`) — defaults existing seeded rows to `1`; the real Sprint 008 `create_operation` mutation inserts `0`.

  See docs/REAL_PRODUCTION_PLANNER.md for the full CSV format, calculation rules, and data ownership boundaries.

### Procurement workflow (migration 0017, live now)

- **operation_procurement_lines**(operation_id, type_id, status, notes, updated_at) — mutable operation-specific workflow state only. Required, owned, and shortage quantities are never copied here; the shopping list derives them from the live production plan.
- **operation_procurement_events**(event_id, operation_id, type_id, previous_status, new_status, notes, event_type, created_at) — append-only status/note and reset history. Reset removes current line state while preserving these audit events. Neither table mutates synchronized or manual inventory.

### Quartermaster doctrine domain (migration 0018, live now)

- **doctrine_groups** — doctrine identity, description, category, fleet notes, active state, and version.
- **doctrine_fits** — imported EFT fits, desired fleet quantity, normalized hull reference, source, and original EFT text.
- **doctrine_fit_items** — normalized CCP type references, detected item kind, and per-fit quantity. Fleet demand and readiness remain live derived results rather than stored totals.

### Corporation asset synchronization (migration 0020, live now)

- **corporations** — corporation identity and the connected character most recently used to authorize its read-only snapshot.
- **corporation_asset_sync_state** / **corporation_character_sync_state** — corporation- and character-facing sync status, Director verification, timestamps, counts, pages, and last error.
- **corporation_divisions** — custom hangar division names keyed by corporation and division number.
- **corporation_assets** — replaceable per-corporation item snapshots with type, quantity, raw location, flag, singleton state, derived division, and sync time.

These tables are deliberately separate from `character_assets`,
`manual_inventory_entries`, and demo inventory. Migration 0020 does not add
corporation ownership to any Production, Procurement, Quartermaster, or
inventory-aggregation query.

### Personal Commerce (migration 0021, live now)

- **character_market_order_sync_state** / **character_contract_sync_state** —
  per-character last attempt, last success, status, count, page count, cache
  metadata where supported, and last error.
- **character_market_orders** — replaceable active personal-order snapshots,
  keyed by character and order ID. Prices and escrow are exact decimal text.
- **character_contracts** — replaceable personal-contract list snapshots,
  keyed by character and contract ID. Status, direction participants, type,
  availability, dates, locations, and exact decimal amounts remain source data.
- **character_contract_detail_state**, **character_contract_items**, and
  **character_contract_bids** — separately cached, lazy-loaded contract detail.

Migration 0021 is additive and does not alter personal/corporation inventory,
Production, Quartermaster, Procurement, pricing, reservations, or operation
tables. Commerce ownership remains personal and isolated by character.

### Sync layer (Sprint 003a–004a)
- **esi_tokens**(character_id PK, access_token_enc, refresh_token_enc, expires_at, scopes)
- **sync_state**(resource, character_id, etag, last_success_at, next_allowed_at, last_error, PRIMARY KEY(resource, character_id))

### SDE (Sprint 004)
- **sde_types**(type_id PK, name, group_id, category_id, volume, portion_size)
- **sde_blueprints**(blueprint_type_id PK, product_type_id, activity, time, max_production_limit)
- **sde_blueprint_materials**(blueprint_type_id, activity, material_type_id, quantity, PK(blueprint_type_id, activity, material_type_id))

### Live game data (Sprint 003–005)
- **assets**(item_id PK, character_id, type_id, location_id, location_flag, quantity, is_singleton, synced_at)
- **blueprints**(item_id PK, character_id, type_id, location_id, me, te, runs, is_copy, synced_at)
- **industry_jobs**(job_id PK, character_id, installer_id, blueprint_type_id, product_type_id, activity, runs, status, start_date, end_date, station_id, synced_at)
- **wallet_snapshots**(id PK, character_id, balance, as_of)
- **market_orders**(order_id PK, character_id, type_id, is_buy, price, volume_remain, issued, synced_at)
- **price_snapshots**(type_id, source, buy, sell, as_of, PK(type_id, source, as_of))

### Requirements expansion (Sprint 004)
- **build_requirements**(project_id, type_id, required_qty, on_hand_qty, in_progress_qty, PK(project_id, type_id)) — the computed diff, produced by the generic Build Target Engine for any target, that feeds the Shopping Engine and Build Tracker.

## Integrity & lifecycle rules

- `PRAGMA foreign_keys = ON` at every connection open; `journal_mode = WAL`.
- Demo data is always flagged (`is_demo`, negative character ids) so Sprint 002 can purge it in one statement without touching user projects.
- Sync writers use transactions per resource per character; readers never see half-synced state.
- No destructive migration may drop user tables (`build_projects`, `recommendations` history) without an export path.
