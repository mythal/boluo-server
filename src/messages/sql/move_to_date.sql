WITH max_offset AS (
    SELECT COALESCE(max(order_offset), -1) AS order_offset
    FROM messages
    WHERE channel_id = $2 AND order_date = $3
)
UPDATE messages
SET order_offset = max_offset.order_offset + 1, order_date = $3
FROM max_offset
WHERE id = $1 AND channel_id = $2
RETURNING messages;