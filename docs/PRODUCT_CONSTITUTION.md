# RenderNorth Industrial — Product Constitution

Version 1.0 — Sprint 001
Status: Ratified. Changes require an explicit amendment entry at the bottom of this file.

## 1. Mission

RenderNorth Industrial is a desktop-first EVE Online industrial command center for capital-scale production. It supports building **any selected ship, structure, or manufacturable item** in EVE industry data — Revelation, Navy Revelation, Apostle, carriers, FAX, dreadnoughts, supercarriers, Titans, structures, components, anything with a blueprint. It answers, from real data, the seven operator questions:

1. What do I own?
2. Where is it?
3. What am I missing?
4. What can I build today?
5. What should I build next?
6. What is blocking my selected build target?
7. What should I buy, mine, manufacture, or recover from asset safety?

## 2. Identity: Intelligence, not Automation

RenderNorth Industrial is **not a bot** and will never become one.

**Hard prohibitions (never, in any release):**
- No gameplay automation of any kind.
- No clicking, keystroke injection, or control of the EVE client.
- No screen scraping, memory reading, or log-file exploitation of the EVE client.
- No unofficial gameplay automation frameworks or gray-market APIs.
- No cache scraping of the EVE client.

**The only data channels are:**
- CCP's official ESI API, authenticated with the official OAuth 2.0 (PKCE) browser flow.
- The official EVE Static Data Export (SDE).
- Optional, explicitly user-enabled third-party *read-only pricing* services (e.g. Janice, Fuzzworks) in later sprints.

The app analyzes; the pilot acts. Every output is a report, a plan, or a recommendation — never an action in the game world.

## 2a. Product Rule: No Ship Is Special-Cased

The core system is a **generic Build Target Engine**. The Avatar Titan is a validation scenario, not the architecture. No type ID, hull, or structure may be hardcoded into engine logic, schema, or UI flow. Every production workflow begins with the same pipeline:

**Select Build Target → Load Blueprint Requirements → Calculate Materials → Compare Inventory → Identify Missing Inputs → Recommend Next Action**

Requirements are always calculated dynamically from industry data (SDE), owned blueprints (ME/TE), owned assets, market prices, and user inventory. A feature that only works for one hull is a defect.

## 3. Scope Fences (do not build)

- No cloud sync. All state lives in a local SQLite database on the user's machine.
- No billing, licensing servers, or telemetry.
- No multi-user accounts. One local install, one operator (who may own many EVE characters).
- No AI chat, LLM features, or nondeterministic advice.
- No mobile or web deployment for MVP. Windows desktop first.

## 4. Determinism Doctrine

Every recommendation must be **deterministic and explainable**:

- Recommendations are produced by rule pipelines over the local database. Same data in, same recommendation out.
- Every recommendation record must (eventually) carry a `reason` payload: the rule that fired, the inputs it read, and the thresholds it compared against. "Start Capital Construction Parts" must be traceable to "component coverage 63% < target, 3 idle component slots, all input minerals at 100%."
- No black boxes. If a rule cannot be explained in one paragraph, it is too complicated to ship.

## 5. Data Stewardship

- ESI tokens are stored locally, encrypted at rest (OS keychain where available), never transmitted anywhere except to CCP's OAuth endpoints.
- The app requests the **minimum ESI scopes** required by enabled modules, and lists every scope and why it is needed in ESI_INTEGRATION.md.
- ESI cache timers and error-limit headers are respected without exception. We are a polite API citizen.
- The user can wipe all local data (including tokens) from inside the app.

## 6. Engineering Values

- **Architecture first.** Layers stay separated: Presentation → Business Logic → Data → Synchronization. UI never talks to ESI directly; engines never render.
- **Maintainable over clever.** Boring, typed, tested code beats smart code.
- **Do not overbuild.** Each sprint ships the thinnest slice that is correct, then hardens it.
- **Local-first.** The app must be useful offline with last-synced data.
- **EVE-inspired, not EVE-copied.** The visual identity evokes a command deck; it does not reproduce CCP art, icons, or trademarks.

## 7. Compliance Posture

The app is designed to comply with CCP's Developer License Agreement and EULA: official APIs only, no gameplay automation, no real-money trading facilitation. If a planned feature is ever in tension with CCP's terms, the feature loses.

## 8. Definition of "MVP Done"

MVP is done when a Windows user can: authenticate one or more characters via ESI, sync assets/blueprints/jobs/wallet, **select any manufacturable build target** (validated end-to-end with an Avatar Titan scenario), and see — from live data — ownership, location, gaps, buildable-today, blocking items, and a deterministic shopping/build plan.

## Amendments

- **A-001 (Sprint 001):** Generalized the MVP target from Avatar-specific production to a generic Build Target Engine. No ship is special-cased; Avatar is the first validation scenario only.
