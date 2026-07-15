# Quartermaster

RNI-154 introduces a deterministic doctrine domain. Doctrine metadata, EFT
fits, desired fleet quantities, original EFT text, and normalized fit items are
owned by Quartermaster. Fleet coverage, missing quantities, build/buy action,
direct-purchase cost/volume, manufacturing-input cost/volume, complete-fit
readiness, and completion totals are calculated live and are never stored as
authoritative snapshots.

Analysis consumes existing services:

- enabled ESI character assets and manual inventory for owned quantities;
- active non-demo reservations where available;
- synchronized character blueprints and CCP blueprint products for Build
  capability;
- the existing recursive Production Engine for manufacturing runs, owned ME,
  deterministic blueprint selection, and leaf-material expansion;
- RNI-152 cached bulk market quotes for Buy lines;
- RNI-153 shopping summary and Multi-Buy, Discord, and CSV export generation;
- CCP SDE type names, categories, groups, and volume.

Usable inventory is allocated to finished doctrine requirements first. The
remaining doctrine-wide pool is then allocated once across aggregated Build
inputs, preventing shared inventory from being counted for more than one
finished fit or manufacturing path. Direct Buy shortages and missing Build
inputs have separate procurement summaries and exports; the combined export
contains each purchase type once and never includes both a Build output and
its expanded inputs. Totals retain known subtotals but are labeled partial
whenever a required line is unresolved, unpriced, depth-limited, or missing
static volume.

The module does not create corporation, wallet, job, alliance-market,
recommendation, automation, or AI data. Those future sources can contribute to
the existing owned/capability/pricing boundaries without changing doctrine
tables.
