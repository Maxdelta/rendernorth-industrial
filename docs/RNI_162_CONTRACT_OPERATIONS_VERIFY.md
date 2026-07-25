# RNI-162 — Contract Operations Verification

Status: Implemented — live desktop verification pending

## Architecture

- Reuses the existing personal-contract synchronization request and the shared Commerce activity read/seen helpers.
- Makes no additional ESI request for Activity, KPI filters, unread state, or Mission Control metrics.
- Preserves terminal contract rows locally after CCP's rolling response window no longer returns them.
- Keeps raw CCP contract-item rows unchanged; identical items are grouped only in the detail presentation.
- Stores read/seen state locally. Mark Read and Mark All Read never write to CCP.

## Contract views

- Active: outstanding and in-progress contracts.
- Activity: completed, cancelled-family, and inferred-expired activity.
- Activity filters: Completed, Cancelled, Expired, New Activity, and Seen Activity.
- Character, direction, status, type, availability, activity date, location,
  title, character, and synchronized item-name filters run client-side.
- All nine operational KPI cards are clickable, reflect equivalent manual
  filters, and clear when clicked again.
- Human-readable status retains the authoritative raw CCP status in storage.

## Timeline

- Issued uses `date_issued`.
- Accepted is shown only when `date_accepted` exists.
- Completed is shown only when `date_completed` exists.
- Expiry uses CCP's `date_expired`.
- CCP does not provide a cancellation timestamp, so none is invented.

## Mission Control-ready metrics

- Outstanding Contracts
- Completed Today
- Cancelled Today
- Expired Today
- Contracts Awaiting Acceptance
- New Contract Activity

Completed Today uses CCP's completion timestamp. Cancelled Today and Expired Today use the date RenderNorth first observed the terminal activity because CCP does not supply those terminal timestamps.

## Automated verification

- Migration 0023 is additive and preserves existing contracts.
- Seen state persists across synchronization.
- Terminal activity remains available after it falls out of a later ESI response.
- Grouping preserves raw item records.
- Timeline rendering does not invent timestamps.
- KPI and combined client-side filters are deterministic.
- A 5,000-row contract dataset filters client-side.
- Existing Commerce UI and backend suites remain in the normal verification run.

## Live verification

Pending:

1. Synchronize Contracts for multiple enabled characters.
2. Verify Active and Activity counts against known EVE contracts.
3. Verify New badges, Mark Read, Mark All Read, restart persistence, and resynchronization persistence.
4. Verify Completed, Cancelled, Expired, New Activity, and Seen Activity filters.
5. Verify the New Activity KPI, date filter, item-name search, and click-again clearing.
6. Verify grouped items and Show Individual Items preserve totals.
7. Verify timeline events and terminal-state notes against CCP-supplied contract dates.
8. Verify smooth filtering and scrolling with a large synchronized history.
9. Confirm Market Orders, Contract Details, and all other completed modules remain unchanged.
