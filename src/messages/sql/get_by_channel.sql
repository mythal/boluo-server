SELECT msg.hide
FROM messages msg
WHERE msg.channel_id = $1
  AND msg.deleted = false
  AND msg.order_date >= to_timestamp($2 / 1000)  -- [after,
  AND ($3 IS NULL OR msg.order_date < to_timestamp($3 / 1000)) -- before)
ORDER BY msg.order_date DESC, msg.order_offset;

