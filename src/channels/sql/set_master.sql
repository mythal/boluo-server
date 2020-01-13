UPDATE channel_members
SET is_master = COALESCE($3, is_master)
WHERE user_id = $1
  AND channel_id = $2
RETURNING channel_members;
