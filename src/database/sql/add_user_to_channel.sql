INSERT INTO channel_members (user_id, channel_id, character_name)
VALUES ($1, $2, $3)
ON CONFLICT DO NOTHING
RETURNING channel_members;