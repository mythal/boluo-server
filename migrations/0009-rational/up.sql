CREATE EXTENSION IF NOT EXISTS "pg_rational";

ALTER TABLE messages ADD COLUMN "pos" float NOT NULL DEFAULT 42.0;

WITH messages_order_table AS (
    SELECT msg.id AS id, row_number() over(partition by msg.channel_id order by order_date, order_offset) AS pos
    FROM messages msg
    ORDER BY pos
)
UPDATE messages
SET pos = messages_order_table.pos
FROM messages_order_table
WHERE messages.id = messages_order_table.id;

ALTER TABLE messages
    ADD CONSTRAINT pos_unique UNIQUE (channel_id, pos) DEFERRABLE INITIALLY IMMEDIATE;
