UPDATE channels
SET name = channels.old_name
WHERE deleted = true;

ALTER TABLE channels DROP COLUMN old_name;
