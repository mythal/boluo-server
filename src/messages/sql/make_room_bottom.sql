UPDATE messages
SET order_offset = order_offset + 16
WHERE channel_id = $1 AND order_date = $2 AND order_offset > $3
RETURNING id, order_date, order_offset;