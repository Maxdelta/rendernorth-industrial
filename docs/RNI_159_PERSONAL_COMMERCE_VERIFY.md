# RNI-159 — Personal Commerce Dashboard Phase 1

Status: Implemented — live verification completed.

## Scope

RNI-159 adds a read-only Commerce destination for personal character market
orders and contracts. It does not add corporation commerce, market actions,
contract actions, notifications, history analytics, or integration with
Production, Quartermaster, Procurement, Inventory ownership, or reservations.

Requested scopes:

- `esi-markets.read_character_orders.v1`
- `esi-contracts.read_character_contracts.v1`

Existing scopes remain requested during reauthorization.

## Deterministic verification

- Migration 0021 applies to a fresh database and additively upgrades the prior
  schema without changing existing personal inventory.
- Market-order snapshots remain separated by character and buy/sell side.
- Filled quantity, fill percentage, remaining sell value, buy commitment, and
  escrow are derived deterministically.
- Corporation orders returned by the character endpoint are excluded.
- Contract status, type, availability, direction, dates, locations, and exact
  decimal amounts persist per character.
- Contract list pagination follows `X-Pages`; items and auction bids load only
  when the selected contract detail is opened.
- Successful empty results are not errors. Failed syncs preserve prior rows and
  surface the error through sync state.
- Diagnostics expose only capability, count, page, timestamp, status, and
  error-presence metadata—never order rows, contract contents, counterparties,
  amounts, locations, tokens, credentials, authorization codes, client secrets,
  or PKCE verifier values.

## Commerce presentation and exposure

Commerce monetary values retain their exact decimal strings. Compact K/M/B/T
labels are presentation-only, and the exact grouped value remains available by
hovering or focusing the displayed amount.

Phase 1 Total Commerce Exposure is defined as:

`Remaining Sell Value + Remaining Buy Commitment + Open Contract Value + Collateral Exposure`

Only outstanding or in-progress contracts contribute contract value and
collateral. Finished contracts, realized revenue, wallet balances, and rewards
are excluded. Market escrow is also excluded because it is already part of the
full remaining buy commitment and including it again would double count that
exposure.

For multi-character views, each synchronization status reports the oldest
relevant successful timestamp. A mixture of successful, never-synchronized, or
failed characters is reported as Partial rather than implying that all selected
characters are current.

Character portraits are deferred. The application has no existing reusable
portrait-loading component, and adding remote image lifecycle behavior is not
required for this presentation pass.

## Live verification

Status: COMPLETED

1. Open Settings → Characters & ESI Sync.
2. Reauthorize each test character and confirm CCP consent includes both new
   read-only scopes.
3. Confirm each character shows Market Orders and Contracts capability state.
4. Run Sync Market Orders for one character, then Sync All Market Orders.
5. Run Sync Contracts for one character, then Sync All Contracts.
6. Confirm disabled characters are excluded from both Sync All operations.
7. Open Commerce → Market Orders and verify character, side, item, location,
   price, volumes, filled quantity, fill percentage, issue/expiry, range,
   escrow, source, and last-sync values against the EVE client.
8. Exercise character, buy/sell, expiring-soon, item, and location filters and
   every order sort option.
9. Open Commerce → Contracts and verify direction, status, type, availability,
   dates, start/end locations, price, reward, collateral, and buyout.
10. Open an item-exchange or courier contract and confirm offered/requested
    items load. Open an auction and confirm bids load.
11. Disconnect the network, attempt both sync types, and confirm the prior
    successful rows remain visible with a precise sync error.
12. Confirm a character with no active orders or returned contracts shows a
    successful empty state rather than a failure.
13. Export diagnostics and confirm no commerce rows, contract details,
    counterparties, amounts, raw errors, or authentication secrets appear.
14. Confirm large contract and summary values use compact K/M/B/T labels without
    leaving their cards, then hover and keyboard-focus each value to confirm the
    exact ISK value remains available.
15. Add the four documented exposure components and confirm the Total Commerce
    Exposure value matches exactly; confirm escrow, rewards, and finished
    contracts do not increase it.
16. Confirm Expiring Soon uses a warning label/glyph/accent and Collateral
    Exposure uses a distinct risk label/glyph/accent, so neither depends on
    color alone.
17. Switch Market Orders and Contracts by mouse and arrow key. Confirm the active
    module is unmistakable and each module retains its own advanced filters.
18. Expand Advanced Filters in each module and verify the primary character,
    search, expiration, side/direction, and status filters remain visible.
19. Scroll hundreds of rows inside each table and confirm the header remains
    visible while horizontal scrolling still works.
20. Select All Characters and confirm the sync panel uses the oldest relevant
    success and reports Partial or Error when any enabled character is not
    current. Select one character and confirm its exact timestamp is shown.
21. Review the page at standard desktop, narrower laptop, and high-density wide
    desktop widths. Confirm cards wrap, tabs and filters remain reachable, and
    long names and ISK values remain contained.
22. Compare several market-order and contract values to the pre-polish build and
    EVE client to confirm the underlying exact values did not change.
