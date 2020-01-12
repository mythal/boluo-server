SELECT member
FROM channel_members member
where user_id = $1 AND channel_id = $2
LIMIT 1;
