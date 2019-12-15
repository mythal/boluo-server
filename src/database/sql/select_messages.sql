SELECT messages
FROM messages
WHERE channel_id = $1
ORDER BY order_date DESC, order_offset