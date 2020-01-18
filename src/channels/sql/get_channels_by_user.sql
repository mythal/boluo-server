SELECT c, cm
FROM channel_members cm INNER JOIN channels c ON cm.channel_id = c.id
WHERE cm.user_id = $1;
