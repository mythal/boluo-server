UPDATE messages m
SET order_offset = m.order_offset + 1
WHERE m.order_date = $2 AND m.channel_id = $1 AND m.order_offset > $3
RETURNING m;