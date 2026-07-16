# Changelog

All notable user-facing changes to RenderNorth Industrial are recorded here. Releases are listed newest first and use semantic versions.

## [0.1.0] — 2026-07-15

### RenderNorth Industrial Open Beta 0.1

RenderNorth Industrial Open Beta introduces a complete local-first industrial workflow for EVE Online, from character synchronization and production planning through market valuation, procurement, and doctrine fleet readiness.

### Added

- Official read-only CCP SSO with Authorization Code + PKCE and multi-character management.
- Character asset and blueprint synchronization with human-readable location resolution.
- Owned-inventory and independent Jita market search with weighted acquisition and liquidation pricing.
- Production planning, economics, CCP-derived volumes, shopping lists, procurement state, and Multi-Buy, Discord, and CSV exports.
- Quartermaster doctrine creation, EFT import, fleet targets, readiness analysis, recursive build-input expansion, procurement cost, and hauling volume.
- Open Beta onboarding, the official RenderNorth CCP application, advanced custom Client ID support, static-data validation, diagnostics, support information, Windows installer, and portable packaging.

### Changed

- Normal navigation exposes only completed workflows and hides development controls unless debug mode is enabled.
- Quartermaster prioritizes fleet readiness and actionable READY / BUILD / BUY / BLOCKED results.
- Inventory and market search are separate workflows.
- Production economics support a selectable revenue basis and operation-specific persisted cost assumptions.

### Fixed

- Query failures no longer appear as empty inventory, and unowned market searches no longer imply zero value.
- Immediate-sale economics remain usable when listed-sale orders are unavailable.
- Cost assumptions can be applied, persisted, and reset.
- Resolvable locations display human-readable paths.
- Quartermaster identifies partial totals and expands manufacturing inputs without double-counting inventory.
- Static-data setup distinguishes archives, wrong folders, incomplete extraction, unsupported data, and inaccessible paths.

### Known Issues

- Corporation assets, corporation blueprints, industry jobs, and character market-order synchronization are not supported.
- A reported BPC selection issue in Production remains under investigation.
- Build-location and jump-distance scope are not implemented.
- The installer is unsigned and may trigger Windows SmartScreen.
- CCP JSONL static data must be downloaded and extracted manually.
- Automatic application updates are not implemented.
- Open Beta software may contain defects and incomplete workflows.

### Security & Privacy

- Character authorization uses official CCP SSO with Authorization Code + PKCE and read-only ESI scopes.
- RenderNorth Industrial never automates gameplay.
- Application data remains in the local SQLite database.
- Refresh tokens remain in Windows Credential Manager and are never exposed to the frontend or diagnostics.
- The distributed official CCP application does not include a Client Secret.
- Exported diagnostics exclude access tokens, refresh tokens, Client IDs, and credentials.
