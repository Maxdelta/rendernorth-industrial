-- CCP SDE type volume, expressed in cubic metres. Nullable because some
-- imported or self-healed placeholder types may not provide a volume.
ALTER TABLE eve_types ADD COLUMN volume_m3 REAL;
