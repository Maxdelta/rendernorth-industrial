# RenderNorth Industrial — Database Schema

Version 1.2 — Sprint 003 (migration 0003: Inventory Engine foundation — categories, locations, items, allocations, reservations, lifecycle states)

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

## Live tables (migrations 0001–0003)

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
- **inventory_reservations**(id PK, item_id FK, project_id FK build_projects nullable, quantity, reason, created_at, released_at) — hard hold reducing available quantity while `released_at` is unset. Schema and seed data exist; the engine that creates/releases these rows (`ReservationEngine`) is an interface stub this sprint — no command creates a reservation yet.

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
