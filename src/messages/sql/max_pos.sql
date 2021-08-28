SELECT max(pos)
FROM messages
WHERE channel_id = $1;