SELECT sm
FROM channel_members cm
    INNER JOIN channels c ON cm.channel_id = c.id
    INNER JOIN space_members sm ON sm.space_id = c.space_id AND sm.user_id = cm.user_id
WHERE cm.user_id = $1 AND cm.channel_id = $2;
