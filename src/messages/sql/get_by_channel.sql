SELECT msg.hide
FROM messages msg
WHERE msg.channel_id = $1
  AND msg.deleted = false
ORDER BY msg.order_date DESC, msg.order_offset;

