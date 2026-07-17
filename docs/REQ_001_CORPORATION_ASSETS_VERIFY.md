# REQ-001 — Corporation Asset Synchronization Phase 1

Status: **Implemented — Live Director synchronization pending beta verification**

REQ-001 adds read-only corporation asset synchronization and an isolated
Inventory viewer. It does not make corporation assets available to Production,
Quartermaster, Procurement, reservations, or any ownership aggregation.

## Verified

- Database migration 0020 is additive and registered for fresh installations
  and upgrades.
- The complete authorization scope set is requested without removing any
  existing scope.
- Existing characters can reauthorize through the established PKCE flow.
- Character capability status distinguishes granted and missing permissions.
- Corporation asset synchronization validates CCP's required Director role.
- Missing Director access produces a precise, user-facing explanation instead
  of being presented as an empty successful inventory.
- Inventory implements Personal, Corporation, and Both viewing filters.
- Corporation, personal, manual, and demo ownership sources remain separate.
- Corporation Blueprint scope authorization is visible as "Authorized, not yet
  implemented" when granted.
- No corporation-blueprint endpoint, storage, synchronization, display,
  Production integration, or Quartermaster integration exists in Phase 1.
- Rust builds, automated tests, frontend builds, and diff validation pass.

## Pending live beta verification

These items are pending because no currently connected test character holds
CCP's actual Director corporation role. They are access limitations, not known
software failures.

- Successful corporation asset synchronization using a real Director character.
- Nonzero live corporation asset import.
- Multiple-corporation synchronization.
- Live corporation location resolution.
- Live division labels across real corporation data.

## Security and data boundaries

- Access and refresh tokens remain inside the Rust ESI boundary; refresh tokens
  are stored through the existing Windows Credential Manager token store.
- The frontend receives asset and sync metadata, never token contents.
- Diagnostics include granted scope names and safe sync metadata, but exclude
  access tokens, refresh tokens, authorization codes, client secrets, and PKCE
  verifier values.
- Corporation snapshots use dedicated tables and are not merged with
  `character_assets`, `manual_inventory_entries`, or demo rows.

## Requested ESI scopes

1. `esi-assets.read_assets.v1`
2. `esi-characters.read_blueprints.v1`
3. `esi-universe.read_structures.v1`
4. `esi-assets.read_corporation_assets.v1`
5. `esi-characters.read_corporation_roles.v1`
6. `esi-corporations.read_divisions.v1`
7. `esi-corporations.read_blueprints.v1` — authorized for a later Phase 2;
   unused by REQ-001.
