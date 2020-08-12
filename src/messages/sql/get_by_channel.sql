SELECT msg
FROM messages msg
WHERE msg.channel_id = $1
  AND msg.deleted = false
  AND ($2 IS NULL OR msg.order_date < to_timestamp($2 / 1000.0)) -- before
ORDER BY msg.order_date, msg.order_offset DESC
LIMIT $3;

