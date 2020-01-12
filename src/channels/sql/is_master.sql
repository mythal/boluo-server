SELECT is_master
FROM channel_members cm
    INNER JOIN channels ch on cm.channel_id = ch.id
    INNER JOIN space_members sm on sm.space_id = ch.space_id AND sm.user_id = cm.user_id
WHERE cm.user_id = $1 AND cm.channel_id = $2
LIMIT 1;
