-- RenderNorth Industrial — migration 0010
-- Sprint 009 audit finding: "Mechanical Parts" was hand-authored into the
-- demo fixture (migrations 0003 and 0007) under category_key
-- 'reaction_materials'. That's wrong — in EVE Online, Mechanical Parts is
-- a manufactured construction component (built from minerals, consumed
-- as an input to capital components), not a reaction product. Reaction
-- materials are a structurally different category of item entirely
-- (e.g. Platinum Technite, Titanium Diborite) that this app's demo data
-- never included in the first place, so nothing else in the fixture is
-- affected.
--
-- This is a DATA correction only — no schema change, no new table, no
-- new column. It exists as a migration (rather than being fixed in place
-- in 0003/0007) because those files are frozen; the project's standing
-- rule is that past migrations are never edited, only added to.
--
-- Scope and honesty note: the REAL, CCP-import-derived production
-- calculation (see production/repository.rs::group_and_category_for) has
-- never had any hardcoded category mapping — it reads eve_groups/
-- eve_categories, populated entirely from imported static data. This
-- fix affects only the illustrative demo fixture (production_requirements
-- and inventory_items), which existed to make the app usable before any
-- real import; it has no effect on any real operation's calculated plan.
-- 'components' (not 'capital_components') was chosen because Mechanical
-- Parts is itself a base-tier manufactured item that gets consumed BY
-- capital component blueprints, not a capital-tier item in its own
-- right — the same tier as Capital Capacitor Batteries' own inputs. This
-- reflects general EVE Online domain knowledge and was not verified
-- against a live CCP SDE export in the environment this was written in.

UPDATE production_requirements
SET category_key = 'components'
WHERE type_name = 'Mechanical Parts' AND category_key = 'reaction_materials';

UPDATE inventory_items
SET category_key = 'components'
WHERE type_name = 'Mechanical Parts' AND category_key = 'reaction_materials';

-- production_requirement_groups declares which categories exist for an
-- operation's demo requirement scope (structural metadata only — see
-- migration 0007's own doc comment). Operation 4's reaction_materials
-- group entry existed solely to display Mechanical Parts under that
-- category; with the row above corrected, that group declaration is
-- stale and would show an empty "Reaction Materials" bucket for
-- operation 4. Replace it with a components declaration instead — but
-- only if operation 4 doesn't already have one, to stay idempotent.
INSERT INTO production_requirement_groups (operation_id, category_key)
SELECT 4, 'components'
WHERE NOT EXISTS (
    SELECT 1 FROM production_requirement_groups WHERE operation_id = 4 AND category_key = 'components'
);

DELETE FROM production_requirement_groups
WHERE operation_id = 4
  AND category_key = 'reaction_materials'
  AND NOT EXISTS (
      SELECT 1 FROM production_requirements
      WHERE operation_id = 4 AND category_key = 'reaction_materials'
  );
