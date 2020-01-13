ALTER TABLE channel_members DROP COLUMN is_master;
ALTER TABLE space_members ADD COLUMN is_master bool NOT NULL DEFAULT false;
