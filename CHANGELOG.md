# Changelog

All notable user-facing changes to RenderNorth Industrial are recorded here. Releases are listed newest first and use semantic versions.

## [0.1.7] — 2026-07-25

### RenderNorth Industrial Open Beta 0.1.7

Commerce now turns completed Market Orders and Contracts into durable operational activity: users can see what changed, what needs attention, and what has reached a terminal state without losing local read history.

### Highlights

- Commerce Activity Center separates active Market Orders from completed activity.
- Contract Operations separates active Contracts from completed, cancelled, expired, new, and seen activity.
- Durable read/unread state survives navigation, restart, and resynchronization.
- Contract Details groups identical items while preserving access to every synchronized CCP item record.

### New

- Completed Orders activity with New and Seen states.
- Mark Read and Mark All Read controls with persistent unread badges.
- Dynamic Bought and Sold wording with Remaining / Original quantity presentation.
- Active and Activity Contract views.
- Nine interactive Contract KPI cards with synchronized manual filters.
- Contract lifecycle timelines using only timestamps supplied by CCP.
- Grouped Contract Details with Show Individual Items expansion.
- Contract metrics prepared for later Mission Control aggregation.

### Improved

- Character, direction, state, type, availability, location, date, and text filtering.
- Large Market Order and Contract histories render in dedicated scroll regions with stable headers.
- KPI cards toggle filters cleanly and remain synchronized with manual filter changes.
- Contract states use clearer operational wording.

### Architecture

- Market Orders and Contracts share one durable Commerce activity architecture.
- Migrations 0022 and 0023 preserve first-observed, last-observed, and seen state locally.
- Activity views reuse synchronized snapshots and do not issue duplicate ESI synchronization requests.
- Mission Control-ready metrics remain owned by Commerce rather than duplicating Commerce business logic.

### Performance

- Client-side filtering supports thousands of synchronized records.
- Activity and Contract filtering reuse local snapshots with no row-level ESI requests.
- Dedicated table scrolling keeps large histories responsive.

### Bug Fixes

- Seen Market Order and Contract activity no longer becomes New again after resynchronization.
- Contract Details now uses an opaque, isolated modal surface so Commerce content cannot bleed through it.
- Sticky headers remain readable and stable while large activity tables scroll.

### Known Limitations

- Personal Market Orders and Contracts only; corporation Commerce is not supported.
- CCP Market Order history is limited to the history returned by CCP and lacks an authoritative terminal timestamp for every state.
- Contract history outside CCP’s synchronization window cannot be reconstructed.
- Cancelled or expired daily Contract metrics may use RenderNorth’s local first-observed time when CCP provides no terminal timestamp.
- Contract item-name search depends on locally synchronized Contract item details.
- Mission Control does not display the prepared Commerce metrics in this release.

### Security & Privacy

- Commerce remains read-only and cannot create, modify, cancel, accept, or bid on EVE orders or Contracts.
- No new ESI scopes were added.
- Activity data remains local and diagnostics continue to exclude private order and Contract contents, credentials, tokens, authorization codes, Client Secrets, and PKCE values.

## [0.1.6] — 2026-07-20

### RenderNorth Industrial Open Beta 0.1.6

Commerce is now faster to refresh and easier to explore.

### Added

- Sync Market Orders directly from Commerce.
- Sync Contracts directly from Commerce.
- Sync Commerce for both datasets.
- Interactive Market Order summary cards.
- Interactive Contract summary cards.
- One-click filtering from Commerce KPIs.
- Active-filter indicators with click-to-clear behavior.

### Improved

- Manual Commerce filters and KPI highlights now remain synchronized.
- Contract KPI cards automatically open the Contracts module.
- Selected-character sync targets only the selected character.
- All Characters sync targets all enabled eligible characters.
- Last-sync status refreshes directly on the Commerce page.
- Sticky headers no longer overlap summary cards.
- Large datasets are easier to navigate.

### Fixed

- Manual Commerce filter changes no longer leave KPI highlights out of sync.
- Sticky Commerce table headers no longer overlap summary cards.

### Security & Privacy

- Commerce synchronization remains read-only.
- No market or contract actions were added.
- No new ESI scopes were added.
- No database or migration changes were made.
- Diagnostics remain unchanged.

### Known Limitations

- A completed Market Order activity inbox is not implemented.
- Corporation Commerce is not supported.
- Monetary summary cards without a direct filter remain informational.
- Wallet journal and wallet transaction synchronization are not supported.

## [0.1.5] — 2026-07-19

### RenderNorth Industrial Open Beta 0.1.5

Inventory is now cleaner, faster, and easier to use.

### Added

- Collapsed Asset Source Details for verification and troubleshooting.

### Improved

- Inventory Search & Market Valuation is now the primary Inventory experience.
- Personal, Corporation, and Both now work as asset-source filters.
- Raw synchronized ESI assets moved into a collapsed Asset Source Details section.
- Reduced duplicate inventory presentation.
- Improved Inventory performance with large asset counts.
- Preserved market filters, source selection, sorting, and cached pricing during asset refreshes.
- Cleaner workflow for ownership, valuation, and troubleshooting.

### Fixed

- Inventory Market Valuation no longer disappears after synchronized assets finish loading.
- Personal, Corporation, and Both no longer behave like page-level selectors.
- Asset loading can no longer hide or replace the Market Valuation module.
- Inventory state is now separated correctly between synchronized assets and valuation data.

### Security & Privacy

- No authentication or permission changes.
- No new ESI scopes.
- No credential or diagnostic exposure.
- Corporation assets remain read-only.
- Corporation assets are still not used by Production, Quartermaster, or Procurement.

### Known Limitations

- Corporation asset synchronization still requires an actual EVE Director role.
- Corporation assets are displayed and valued but are not yet available as Production sources.
- Corporation Blueprint synchronization remains planned.
- Raw Asset Source Details are intended for verification and troubleshooting, not the primary workflow.

## [0.1.4] — 2026-07-18

### RenderNorth Industrial Open Beta 0.1.4

Commerce has arrived.

### Added

- Commerce Dashboard.
- Personal Market Order synchronization.
- Personal Contract synchronization.
- Multi-character Market Order and Contract views.
- Buy and Sell order tracking.
- Remaining quantity and filled percentage.
- Order expiration tracking.
- Contract status and direction tracking.
- Item Exchange, Courier, and Auction contract support.
- Offered and requested contract-item details.
- Contract auction bid details.
- Per-character Commerce synchronization states.
- Commerce filters, sorting, and search.
- Total Commerce Exposure.

### Improved

- Compact K/M/B/T ISK formatting.
- Exact ISK values available on hover and keyboard focus.
- Separate Market Order and Contract synchronization status.
- Sticky table headers.
- Responsive Commerce layout.
- Improved filter organization.
- Clearer financial summary cards.
- Open Contract Value wording.
- Commerce Exposure explanation tooltip.
- Sanitized Commerce diagnostics.

### Fixed

- Finished and terminal contracts no longer show a misleading Open action or active countdown.
- Contract details open in a visible dialog instead of rendering below a long contract table.

### Security & Privacy

- Market Orders and Contracts are read-only.
- No order modification or cancellation is available.
- No contract creation, acceptance, rejection, deletion, or bidding is available.
- Diagnostics exclude exact orders, contract contents, counterparties, amounts, locations, tokens, credentials, and PKCE values.
- Personal Commerce data remains isolated by character.

### Known Limitations

- Personal Market Orders and Contracts only.
- Corporation Market Orders and Corporation Contracts are not yet supported.
- Wallet journal and wallet transaction integration are not yet implemented.
- Completed or sold-out Market Order history is not available from the active-order endpoint.
- Contract history is limited by CCP's returned history window.
- Contract counterparties currently display as identifiers where names have not been resolved.
- Corporation Blueprint synchronization is authorized but not yet implemented.
- Corporation Assets still require an actual EVE Director role for live synchronization.

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
