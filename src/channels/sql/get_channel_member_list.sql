SELECT m
FROM channel_members m
WHERE channel_id = $1 AND is_joined;
