CREATE EXTENSION IF NOT EXISTS "pg_rational";

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

ALTER TABLE messages
    ADD CONSTRAINT pos_unique UNIQUE (channel_id, pos) DEFERRABLE INITIALLY IMMEDIATE;
