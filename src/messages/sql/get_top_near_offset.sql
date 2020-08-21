SELECT order_offset
FROM messages
WHERE channel_id = $1 AND order_date = $2 AND order_offset < $3
ORDER BY order_offset DESC
LIMIT 1;

