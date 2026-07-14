# Market Pricing Service

RNI-152 uses CCP's public `GET /markets/{region_id}/orders` operation. A
market profile identifies a region and concrete station/structure. Refreshing
downloads every paginated regional order page once, transactionally replaces
the normalized local cache, and retains the prior successful snapshot if any
request or database write fails. `Cache-Control: max-age`, `X-Pages`, the
project User-Agent, and `X-Compatibility-Date: 2026-07-14` are honored.

Acquisition price is the volume-weighted fill of ascending sell orders at the
profile location. Liquidation price is the volume-weighted fill of descending
buy orders usable at that location. A partial fill reports its filled and
available volume and is never presented as a complete quote. No orders is
represented by `None`, never zero ISK.

The reusable backend interfaces are independent of Production:

- `quote(connection, type_id, quantity)` — one local cached quote.
- `bulk_quote(connection, &[(type_id, quantity)])` — deduplicatable bulk
  contract intended for RNI-153 doctrine demand.
- `weighted_sell_fill(order_levels, quantity)` — acquisition depth.
- `weighted_buy_fill(order_levels, quantity)` — liquidation depth.
- `selected_profile(connection)` — market identity, freshness, and errors.
- `refresh(connection)` — controlled regional snapshot refresh.

Inventory search and operation economics consume these services but do not
own them. No frontend render performs an ESI request.

Operation economics loads persisted, operation-specific cost assumptions and
accepts an explicit `immediate` or `listed` revenue basis. Immediate and listed
output valuations remain independent; if the requested basis is unavailable,
the service returns the other available basis plus a precise fallback message.
Cost assumptions are read, saved, and reset through separate commands so merely
editing a field never mutates persisted economics.

## CCP static volume

RNI-152 treats packaged item volume as a separate, reusable industrial metric.
The static-data importer stores CCP's `invTypes`/type `volume` value as
`eve_types.volume_m3`; inventory stacks, market requests, production
requirements, shortages, and shopping exports multiply that single imported
unit value by their relevant quantities through the shared `volume` module.
No UI or pricing path invents or maintains a second volume value. If CCP's
static record has no usable volume, unit and aggregate volume remain unknown
instead of being reported as zero.

Volume is informational in RNI-152. It does not select routes, ships, cargo
fits, or hauling recommendations.
