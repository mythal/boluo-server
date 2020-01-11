SELECT messages
FROM messages
WHERE channel_id = $1 AND deleted = false
ORDER BY order_date DESC, order_offset;
