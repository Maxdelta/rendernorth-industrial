# RenderNorth Industrial — ESI Integration

Version 1.0 — Sprint 001 (design only; implementation begins Sprint 002)

## 1. Principles

- **Official channels only.** CCP ESI (`https://esi.evetech.net`) plus the official SSO (`https://login.eveonline.com`). Nothing else touches game data.
- **Read-only intelligence.** MVP requests no scopes that mutate game state.
- **Polite citizen.** Cache headers, ETags, and error-limit headers are honored unconditionally.
- **Local-only secrets.** Tokens never leave the machine except toward CCP's SSO endpoints.

## 2. Authentication: OAuth 2.0 Authorization Code + PKCE

Native-app flow (no client secret shipped in the binary):

1. App generates `code_verifier` + `code_challenge` (S256) and a random `state`.
2. App opens the **system browser** at `login.eveonline.com/v2/oauth/authorize` with client_id, scopes, challenge, state, and `redirect_uri=http://localhost:<port>/callback`.
3. A short-lived local HTTP listener (Rust, loopback only) receives the callback, validates `state`, captures `code`.
4. Rust exchanges `code` + `verifier` for access/refresh tokens at `/v2/oauth/token`.
5. Access token JWT is validated (issuer, audience, expiry); character identity read from it.
6. Tokens are encrypted at rest — Windows: DPAPI/Credential Manager via the `keyring` crate; refresh handled transparently by the ESI client.

Multiple characters = repeat the flow; one token set per character in `esi_tokens`.

## 3. Scopes (minimum set per module)

RNI-150 active scope set: `esi-assets.read_assets.v1` and
`esi-characters.read_blueprints.v1`. Characters
connected under Sprint 011A's identity-only grant must reauthorize through
Add / Reauthorize Character before their first asset sync. This is required
OAuth consent expansion; existing refresh tokens cannot silently gain scope.

| Scope | Used by | Why |
|---|---|---|
| esi-assets.read_assets.v1 | Asset Manager, Inventory Engine | What do I own / where is it |
| esi-assets.read_corporation_assets.v1 | Corporation Asset Viewer | Read corporation asset stacks and raw locations (Director only) |
| esi-characters.read_corporation_roles.v1 | Corporation Asset Viewer | Verify that the authorizing character has the required Director role |
| esi-corporations.read_divisions.v1 | Corporation Asset Viewer | Read custom corporation hangar division names (Director only) |
| esi-corporations.read_blueprints.v1 | Future Corporation Blueprint Viewer | Authorization requested now; endpoint use and synchronization are not implemented in REQ-001 |
| esi-universe.read_structures.v1 | Asset Manager | Resolve citadel names for asset locations |
| esi-characters.read_blueprints.v1 | Blueprint Manager | BPO/BPC inventory, ME/TE |
| esi-industry.read_character_jobs.v1 | Industry Jobs, Factory Status | Running/idle slots, in-progress output |
| esi-wallet.read_character_wallet.v1 | Wallet panel, Cost Engine | Liquidity for shopping plan |
| esi-markets.read_character_orders.v1 | Market Orders module | Open buy/sell exposure |
| esi-contracts.read_character_contracts.v1 | Contracts module | Personal contract list, item details, and auction bids |
| esi-planets.manage_planets.v1 *(read use only)* | PI Manager (later sprint) | PI colony output vs. build-target requirements |
| esi-industry.read_character_mining.v1 | Mining Manager (later sprint) | Mining ledger vs. mineral needs |

Asset safety contents have limited ESI visibility; the Asset Safety Recovery module will work from the last-known asset snapshot plus user confirmation, and this document will be amended when that sprint is planned.

Each release lists its exact scope set in the auth screen before the browser opens. Adding a scope requires re-consent, never silent expansion.

## 4. Sync Engine Design (Sprint 002–003)

Sprint 011B implements the personal-character asset slice: sequential
`X-Pages` pagination, access-token refresh through the existing secure token
store, and one transaction per completed character snapshot. A fetch or write
failure retains the prior successful asset rows and records visible sync state.
That personal snapshot remains separate from the later corporation snapshot.

### Corporation assets (REQ-001 Phase 1)

- Official operations: `GET /corporations/{corporation_id}/assets/`,
  `GET /characters/{character_id}/roles/`, and
  `GET /corporations/{corporation_id}/divisions/`; public character and
  corporation information routes identify the current corporation and name.
- Required scopes: `esi-assets.read_corporation_assets.v1`,
  `esi-characters.read_corporation_roles.v1`, and
  `esi-corporations.read_divisions.v1`.
- CCP requires the authorizing character to hold the corporation `Director`
  role. Missing scopes require reauthorization; a missing role produces an
  explicit permission error and leaves the last successful snapshot intact.
- Asset pagination is driven by `X-Pages` with at most 1000 records per page.
  The route's documented cache duration is 3600 seconds.
- Corporation identity, custom hangar division names, item hierarchy, and
  human-readable terminal locations are stored/read independently of personal
  assets. Raw IDs and flags remain available when a structure is inaccessible.
- Phase 1 is view-only. Corporation rows never enter Production, Quartermaster,
  Procurement, manual inventory, personal inventory totals, or demo data.
- `esi-corporations.read_blueprints.v1` is included in authorization to avoid
  an immediate second consent cycle for Phase 2. REQ-001 does not call the
  corporation-blueprints endpoint or store, display, or consume its records.

### Character blueprints (RNI-150)

- Official operation: `GET /characters/{character_id}/blueprints/` (the
  current `latest` compatibility route; operation version v2 in the legacy
  versioned specification).
- Required scope: `esi-characters.read_blueprints.v1`.
- Pagination: request page 1, read `X-Pages`, then request every remaining
  page sequentially. The route is cached for up to 3600 seconds; ESI may also
  return standard cache metadata such as `Expires`, `ETag`, and `Last-Modified`.
- `item_id`: unique inventory item identifier for this blueprint record.
- `type_id`: blueprint item type; resolved through imported CCP type data.
- `location_id`: raw location or containing-item identifier. No name
  resolution is performed in RNI-150.
- `location_flag`: ESI inventory flag describing where the item is stored.
- `material_efficiency` / `time_efficiency`: the blueprint's ME and TE levels.
- `runs`: `-1` means an original (BPO); a non-negative value is the remaining
  licensed runs on a copy (BPC). This is the authoritative BPO/BPC rule.
- `quantity`: commonly `-1` for a singleton original and `-2` for a copy; a
  positive value can represent a stack of untouched originals. It is stored
  exactly, but is not used instead of `runs` to classify BPO/BPC.
- Production selection rule: manual ownership already explicitly selected by
  an operation remains authoritative. For an intermediate product with no
  manual owned blueprint, synchronized ME is applied only when exactly one
  enabled ESI blueprint item can produce that product. If multiple candidates
  exist, all are displayed and none is auto-selected.

### Human-readable locations (RNI-151)

- Required player-structure scope: `esi-universe.read_structures.v1`.
- Authenticated route: `GET /universe/structures/{structure_id}`.
- Public routes: `GET /universe/stations/{station_id}`,
  `/universe/systems/{system_id}`, `/universe/constellations/{constellation_id}`,
  and `/universe/regions/{region_id}`.
- Requests use `X-Compatibility-Date: 2026-07-12`. Resolved terminal metadata
  is normalized in `location_cache`; raw IDs remain on asset/blueprint rows.
- Parent item IDs are walked through the synchronized character asset snapshot
  with cycle detection and a 32-segment maximum. Inaccessible structures are
  labeled honestly with their raw ID and never assigned a guessed name.

### Personal Commerce (RNI-159 Phase 1)

- Active personal orders use `GET /characters/{character_id}/orders` with
  `esi-markets.read_character_orders.v1`. The current route is not paginated;
  ETag and Last-Modified cache metadata are retained and a `304 Not Modified`
  response preserves the existing snapshot without rewriting rows.
- Personal contracts use `GET /characters/{character_id}/contracts` with
  `esi-contracts.read_character_contracts.v1`. Every page is fetched from the
  `X-Pages` count. CCP returns contracts involving the character that are no
  older than 30 days, plus contracts still in progress.
- Contract items and auction bids use
  `GET /characters/{character_id}/contracts/{contract_id}/items` and
  `/bids`. These details are loaded only when a user opens a contract, cached
  locally, and never requested row-by-row while rendering the dashboard.
- Active orders are replaceable per-character snapshots. Contracts are
  per-character upserts so terminal activity and local read state survive the
  rolling CCP response window. Failed syncs preserve the prior successful
  data; a successful empty response is a valid empty state. Corporation orders
  are excluded, and no corporation contract endpoint is called.
- ISK decimals are stored as exact text received from ESI. Dashboard totals
  use deterministic integer-cent arithmetic rather than binary floating point.
- Commerce data is display-only and remains outside Production, Quartermaster,
  Procurement, Inventory ownership, reservations, and market pricing caches.

### Commerce Activity Center (RNI-161)

- Completed activity uses
  `GET /characters/{character_id}/orders/history` with the existing
  `esi-markets.read_character_orders.v1` scope.
- The official CCP specification describes up to 90 days of cancelled and
  expired orders, a 3,600-second cache, `page` pagination, and `X-Pages`.
  RenderNorth fetches every page sequentially.
- The only authoritative ESI history states are `cancelled` and `expired`.
  RenderNorth never invents a `sold` state. Sold/bought quantity is derived as
  `volume_total - volume_remain`, with order side determining the wording.
- ESI supplies no order-close timestamp. Activity Center "today" and "week"
  summaries use the local `first_seen_at` discovery time and label that
  limitation in the UI.
- New/Seen is local presentation state. Mark Read and Mark All Read update only
  the local database and never call a CCP write endpoint.

### Contract Operations (RNI-162)

- Active and Activity views reuse the existing synchronized personal-contract
  response. Filters, unread state, KPI metrics, and item grouping make no
  additional ESI requests.
- Terminal contract rows are retained locally as an activity ledger after they
  leave CCP's rolling response window.
- `date_issued`, `date_accepted`, `date_completed`, and `date_expired` are
  displayed only when supplied by CCP. No cancellation timestamp is inferred.
- Local first-observed time supports New Activity for terminal states whose
  terminal timestamp CCP does not supply.
- Contract item grouping changes presentation only; raw record IDs and
  quantities remain in the synchronized item cache.

- **Per-resource fetchers** with a shared client: `assets`, `blueprints`, `industry_jobs`, `wallet`, `orders`.
- **Cadence:** driven by ESI's own `expires` header per endpoint — never poll faster than the cache timer. Manual "Sync now" respects the same limits (button disabled until `next_allowed_at`).
- **ETags:** stored in `sync_state`; `304 Not Modified` costs no error budget and no DB write.
- **Error limits:** ESI returns `X-ESI-Error-Limit-Remain` / `-Reset`. Below a safety floor (remain < 20) all fetchers pause until reset. 420 responses halt sync globally for the window.
- **Pagination:** `X-Pages` driven, fetched sequentially per resource to keep burst size polite.
- **Failure policy:** last-known-good rows stay; `sync_state.last_error` recorded; UI shows amber "stale" badges with the age of data.

## 5. Static Data Export (SDE)

- Imported (Sprint 003) into `sde_*` tables from the official conversion (SQLite flavor), keyed by SDE version in `app_meta`.
- Used for: type names, blueprint material trees, activity times, portion sizes — everything the Build Target Engine needs to expand any manufacturable target offline.
- SDE updates are user-triggered ("Update static data"), never silent.

## 6. Optional Price Feeds (post-MVP, off by default)

Janice / Fuzzworks / ESI market endpoints feed `price_snapshots(type_id, source, buy, sell, as_of)`. The Cost Engine reads snapshots only — it never calls the network — so pricing sources are swappable and the app stays deterministic per snapshot.

## 7. What the Integration Will Never Do

- No endpoints that act on the game world beyond read scopes listed above.
- No third-party services receiving the user's tokens or character data.
- No polling wars, header spoofing, or cache-buster tricks.
- No scraping of the EVE client, its cache, or its logs.
