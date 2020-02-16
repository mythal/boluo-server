SELECT msg.hide
FROM messages msg
WHERE msg.channel_id = $1
  AND msg.deleted = false
  AND ($2 IS NULL OR msg.created < to_timestamp($2 / 1000))
ORDER BY msg.created ASC
LIMIT COALESCE($3, 256);

