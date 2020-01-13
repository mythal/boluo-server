SELECT is_master
FROM channel_members cm
WHERE cm.user_id = $1 AND cm.channel_id = $2
LIMIT 1;
