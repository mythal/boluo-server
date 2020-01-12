DELETE
FROM channel_members
WHERE user_id = $1
  AND channel_id = $2;
