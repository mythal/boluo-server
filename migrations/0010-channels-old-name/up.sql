ALTER TABLE channels ADD COLUMN old_name text NOT NULL DEFAULT '';

UPDATE channels
SET old_name = name, name = uuid_generate_v4()::text
WHERE deleted = true;
