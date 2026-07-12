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
| esi-universe.read_structures.v1 | Asset Manager | Resolve citadel names for asset locations |
| esi-characters.read_blueprints.v1 | Blueprint Manager | BPO/BPC inventory, ME/TE |
| esi-industry.read_character_jobs.v1 | Industry Jobs, Factory Status | Running/idle slots, in-progress output |
| esi-wallet.read_character_wallet.v1 | Wallet panel, Cost Engine | Liquidity for shopping plan |
| esi-markets.read_character_orders.v1 | Market Orders module | Open buy/sell exposure |
| esi-planets.manage_planets.v1 *(read use only)* | PI Manager (later sprint) | PI colony output vs. build-target requirements |
| esi-industry.read_character_mining.v1 | Mining Manager (later sprint) | Mining ledger vs. mineral needs |

Asset safety contents have limited ESI visibility; the Asset Safety Recovery module will work from the last-known asset snapshot plus user confirmation, and this document will be amended when that sprint is planned.

Each release lists its exact scope set in the auth screen before the browser opens. Adding a scope requires re-consent, never silent expansion.

## 4. Sync Engine Design (Sprint 002–003)

Sprint 011B implements the personal-character asset slice: sequential
`X-Pages` pagination, access-token refresh through the existing secure token
store, and one transaction per completed character snapshot. A fetch or write
failure retains the prior successful asset rows and records visible sync state.
Corporation assets and structure-name resolution are not part of this slice.

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
