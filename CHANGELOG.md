# Changelog

All notable user-facing changes to RenderNorth Industrial are recorded here. Releases are listed newest first and use semantic versions.

## [0.1.3] — 2026-07-17

### RenderNorth Industrial Open Beta 0.1.3

Open Beta 0.1.3 adds Phase 1 corporation asset synchronization and isolated corporation inventory viewing for authorized industrial corporations.

### Added

- Corporation Asset Synchronization (Phase 1).
- Personal / Corporation / Both inventory filters.
- Director role validation.
- Corporation capability detection.
- Corporation Blueprint authorization support for a future implementation.

### Improved

- Corporation permission guidance now distinguishes authorization from the actual Director corporation role.
- Authorization diagnostics now report safe capability and scope metadata.
- Character capability display now shows corporation asset and future corporation blueprint authorization states.

### Fixed

- Missing Director permissions are reported precisely instead of appearing as an empty successful corporation inventory.

### Security & Privacy

- Corporation permissions are read-only.
- Diagnostics explicitly exclude tokens, authorization codes, client secrets, PKCE verifier values, and credential material.
- No authentication secrets are exposed to the frontend.

### Known Limitations

- Live corporation synchronization requires an EVE character with the actual Director corporation role.
- Corporation Blueprint synchronization is not yet implemented.
- Corporation assets are not yet used by Production or Quartermaster.

## [0.1.2] — 2026-07-16

### RenderNorth Industrial Open Beta 0.1.2

Open Beta 0.1.2 fixes synchronized ESI blueprint selection in Production and adds clearer BPC run-capacity guidance.

### Added

- Source-aware Production blueprint identity for manual and synchronized ESI ownership.
- Detailed blueprint candidate information including owner, source, ME, TE, remaining runs, and location.
- Clear guidance when no owned BPC has enough remaining runs.

### Changed

- Production blueprint matching now uses CCP blueprint-to-product type relationships instead of name matching.
- Production now validates BPC run capacity against requested output quantity.
- Saved operations preserve explicit manual or synchronized blueprint identity.

### Fixed

- Synchronized ESI BPOs and BPCs can now be selected in Production.
- Selected synchronized blueprint ME and TE now drive Production calculations.
- Disabled-character blueprints are excluded from Production candidates.
- Zero-run and insufficient-run BPCs are rejected correctly.
- Existing manual-blueprint operations remain compatible.
- Unrelated blueprints cannot be submitted for a selected product.

### Known Issues

- Corporation assets are not yet supported.
- Character market-order synchronization is not yet supported.
- Build-location and facility selection are not yet implemented.
- The installer is unsigned and may trigger Windows SmartScreen.
- Application updates require manual download and installation.

## [0.1.1] — 2026-07-16

### RenderNorth Industrial Open Beta 0.1.1

Open Beta 0.1.1 adds user-facing release history and update-availability notifications so beta testers can see what changed and know when a newer build is available.

### Added

- What's New page.
- User-facing release history with Added, Changed, Fixed, Known Issues, and Security & Privacy sections.
- Persistent viewed/unviewed release indicator.
- Update availability checks through the public GitHub Releases API.
- Update Available notification.
- Manual Check for Updates.
- Daily, Weekly, and Never update-check preferences.
- Remind Me Later and Skip This Version controls.
- Installer and portable-package download actions.
- Safe update metadata in diagnostics.

### Changed

- Settings → About is organized into About, What's New, Support, and Diagnostics.
- Release packages use deterministic installer and portable ZIP filenames.
- Release and packaging documentation now defines a permanent versioning workflow.

### Fixed

- Users no longer need to manually discover whether a newer beta build exists.
- Current stable semantic versions correctly outrank prereleases with the same core version.
- Failed or offline checks no longer report the application as Up to Date.
- Cached successful update information is preserved after later failures.

### Known Issues

- Corporation assets are not yet supported.
- Corporation blueprints and industry jobs are not yet supported.
- Character market-order synchronization is not yet supported.
- A reported BPC selection issue in Production remains under investigation.
- Build-location and jump-distance scope are not implemented.
- The installer is unsigned and may trigger Windows SmartScreen.
- CCP JSONL static data must be downloaded and extracted manually.
- Automatic update installation is not implemented.
- Updates open in the browser and require manual installation.
- Open Beta software may contain defects and incomplete workflows.

### Security & Privacy

- Update checks use the public GitHub Releases API and send no EVE data, Client ID, tokens, credentials, diagnostics data, or machine identifier.
- Update diagnostics expose only safe update metadata and exclude EVE data, Client IDs, tokens, credentials, and secrets.

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
