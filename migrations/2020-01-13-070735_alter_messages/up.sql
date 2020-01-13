ALTER TABLE messages
    DROP COLUMN metadata;
ALTER TABLE messages
    DROP COLUMN is_system_message;
ALTER TABLE messages
    DROP COLUMN reaction;
ALTER TABLE messages
    RENAME COLUMN crossed_off TO folded;
CREATE INDEX message_channel ON messages USING btree (channel_id);

CREATE FUNCTION hide(messages) RETURNS messages AS
$$
SELECT CASE
           WHEN $1.whisper_to_users IS NULL THEN $1
           ELSE ROW (
               $1.id,
               $1.sender_id,
               $1.channel_id,
               $1.parent_message_id,
               $1.name,
               $1.media_id,
               E'\\x00000000',
               $1.deleted,
               $1.in_game,
               $1.is_action,
               $1.is_master,
               $1.pinned,
               $1.tags,
               $1.folded,
               '',
               $1.whisper_to_users,
               '[]',
               $1.created,
               $1.modified,
               $1.order_date,
               $1.order_offset
               )::messages END AS result;
$$ LANGUAGE SQL;
