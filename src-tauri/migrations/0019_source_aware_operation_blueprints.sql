-- RenderNorth Industrial — migration 0019
-- BUG-001: source-aware selected blueprints for Production operations.
--
-- Keep owned_blueprint_id intact for backward compatibility with existing
-- manual-blueprint operations. New operations use the explicit source and
-- identity columns below. ESI snapshot rows are deliberately validated in
-- application code rather than referenced by a foreign key: blueprint sync
-- replaces a character's snapshot transactionally with DELETE + INSERT, and
-- a foreign key would either block refresh or silently destroy a still-valid
-- saved selection during that replacement window.

ALTER TABLE operation_build_targets ADD COLUMN selected_blueprint_source TEXT;
ALTER TABLE operation_build_targets ADD COLUMN manual_blueprint_id INTEGER REFERENCES blueprints(blueprint_id);
ALTER TABLE operation_build_targets ADD COLUMN character_blueprint_character_id INTEGER;
ALTER TABLE operation_build_targets ADD COLUMN character_blueprint_item_id INTEGER;

UPDATE operation_build_targets
SET selected_blueprint_source = 'manual',
    manual_blueprint_id = owned_blueprint_id
WHERE blueprint_mode = 'owned'
  AND owned_blueprint_id IS NOT NULL;

CREATE INDEX idx_operation_build_target_character_blueprint
ON operation_build_targets(character_blueprint_character_id, character_blueprint_item_id);
