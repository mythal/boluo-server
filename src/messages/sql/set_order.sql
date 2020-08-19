UPDATE messages
SET order_date = $2, order_offset = $3
WHERE id = $1
RETURNING messages;