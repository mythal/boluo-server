DELETE
FROM channel_members cm
USING channels ch
WHERE cm.user_id = $1 AND ch.space_id = $2 AND cm.channel_id = ch.id;
