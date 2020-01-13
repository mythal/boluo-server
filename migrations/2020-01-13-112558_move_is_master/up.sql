ALTER TABLE space_members DROP COLUMN is_master;
ALTER TABLE channel_members ADD COLUMN is_master bool NOT NULL DEFAULT false;
