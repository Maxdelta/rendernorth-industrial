# RNI-161 — Commerce Activity Center Verification

## Official CCP behavior verified

- Endpoint: `GET /characters/{character_id}/orders/history`
- Scope: `esi-markets.read_character_orders.v1`
- Pagination: `page` request parameter and `X-Pages` response header
- Cache age: 3,600 seconds
- History window: cancelled and expired orders up to 90 days in the past
- Response states: `cancelled`, `expired`
- Quantity fields: `volume_total` and `volume_remain`

Sources reviewed on 2026-07-25:

- `https://esi.evetech.net/meta/openapi.json` with
  `X-Compatibility-Date: 2026-07-18`
- `https://developers.eveonline.com/docs/services/esi/overview/`
- `https://developers.eveonline.com/docs/services/esi/best-practices/`

CCP does not return a close/completion timestamp and does not expose a `sold`
history state. RenderNorth therefore:

- preserves CCP's state without reinterpretation;
- derives completed quantity as `volume_total - volume_remain`;
- uses Buy/Sell side to present Bought/Sold wording when completed quantity is
  positive;
- uses `Completed` when no positive fill can be derived;
- uses local first-observed time for New, Today, and This Week activity.

## Architecture and isolation

- Migration 0022 adds a per-character history sync state and an append/update
  local activity ledger.
- First observation creates an unread record. Repeat synchronization updates
  CCP fields while preserving first-seen and seen state.
- Mark Read and Mark All Read are local SQLite updates only.
- Historical activity is loaded once and filtered with memoized client-side
  selectors, so filter changes do not call ESI or rerun backend queries.
- Failed synchronization preserves all prior activity.
- Disabled and demo characters are excluded from dashboard reads and bulk
  mark-read behavior.
- No corporation market orders, wallet data, transactions, journal data,
  fees/taxes, notifications, or Production integration are added.
- The dashboard query is reusable by a future Mission Control widget; the
  current ticket does not add that widget.

## Deterministic verification

- multi-page history response and page-count persistence;
- first-observed records become New;
- seen state survives repeat synchronization;
- Mark Read and Mark All Read;
- failed synchronization preserves prior activity;
- authoritative cancelled/expired states;
- Sold/Bought/Completed wording;
- active quantity shown as remaining / original;
- character, side, derived activity, state, date, item, and location filters;
- deterministic client filtering across 2,500 records;
- migration 0022 fresh install and upgrade preservation;
- diagnostics exclude order rows, prices, locations, tokens, and credentials.

## Live verification

1. Rebuild and launch the desktop application.
2. Open Commerce and click **Sync Market Orders** or **Sync Commerce**.
3. Confirm **Market Orders** contains **Active Orders** and **Completed
   Orders**.
4. Confirm the Completed Orders badge appears only when unread activity exists.
5. Open Completed Orders and verify item, character, location, side, completed
   quantity, CCP state, issued time, expected expiry, first observed, and last
   observed.
6. Confirm cancelled and expired are the only displayed CCP states.
7. Exercise Character, Buy, Sell, Sold, Bought, Cancelled, Expired, date, item,
   and location filters without new network activity or interface stutter.
8. Click **Mark Read**, then **Mark All Read**, and confirm highlight and badge
   counts update.
9. Restart the application and confirm seen state persists.
10. Confirm Active Orders quantity reads `remaining / original`.
11. Disable a character and confirm its completed activity no longer appears.
12. Disconnect the network, sync again, and confirm prior completed activity
    remains available with a precise sync error.

## Known limitations

- Initial history is limited to CCP's available 90-day window.
- RenderNorth retains records locally after first observation, so subsequent
  local history can extend beyond CCP's current response window.
- "Completed Today" and "This Week" mean first observed by RenderNorth because
  CCP provides no close timestamp.
- A positive completed quantity means the order was partially or fully
  transacted before it was cancelled/expired; it does not change CCP's
  authoritative state.
