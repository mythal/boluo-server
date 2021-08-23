CREATE EXTENSION IF NOT EXISTS "pg_rational";

ALTER TABLE channels ADD COLUMN "serial" integer NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN "pos" float NOT NULL DEFAULT 0.0;

WITH messages_order_table AS (
    SELECT a.id AS id, count(b.id) AS pos
    FROM messages a
        LEFT JOIN messages b
            ON a.channel_id = b.channel_id AND (a.order_date > b.order_date OR (a.order_date = b.order_date AND a.order_offset > b.order_offset))
         GROUP BY a.id
         ORDER BY pos
)
UPDATE messages
SET pos = messages_order_table.pos::float
FROM messages_order_table
WHERE messages.id = messages_order_table.id;

WITH channels_message_count AS (
    SELECT ch.id AS id, count(m.id) AS messages_count
    FROM channels ch LEFT JOIN messages m on ch.id = m.channel_id
    GROUP BY ch.id
)
UPDATE channels ch
SET serial = channels_message_count.messages_count
FROM channels_message_count
WHERE ch.id = channels_message_count.id;

ALTER TABLE messages
    ADD CONSTRAINT pos_unique UNIQUE (channel_id, pos) DEFERRABLE INITIALLY IMMEDIATE;
