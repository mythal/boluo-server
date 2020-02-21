ALTER TABLE channel_members ADD COLUMN text_color text DEFAULT NULL;
ALTER TABLE channel_members ADD COLUMN is_joined boolean NOT NULL DEFAULT true;
