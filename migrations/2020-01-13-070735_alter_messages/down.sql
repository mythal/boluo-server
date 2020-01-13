ALTER TABLE messages
    RENAME COLUMN folded TO crossed_off;
ALTER TABLE messages
    ADD COLUMN metadata jsonb default NULL;
ALTER TABLE messages
    ADD COLUMN reaction hstore NOT NULL DEFAULT '';
ALTER TABLE messages
    ADD COLUMN is_system_message boolean NOT NULL DEFAULT false;
DROP INDEX message_channel;
DROP FUNCTION hide;
