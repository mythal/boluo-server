SELECT msg
FROM messages msg
WHERE msg.channel_id = $1
  AND msg.deleted = false
ORDER BY msg.order_date, msg.order_offset;

