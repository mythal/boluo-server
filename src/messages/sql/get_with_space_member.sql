SELECT msg, sm
FROM messages msg
    LEFT JOIN channels ch on msg.channel_id = ch.id
    LEFT JOIN channel_members cm on msg.channel_id = cm.channel_id AND cm.user_id = msg.sender_id
    LEFT JOIN space_members sm on sm.space_id = ch.space_id AND sm.user_id = msg.sender_id
WHERE msg.id = $1 AND msg.deleted = false
LIMIT 1;
