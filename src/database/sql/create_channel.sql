INSERT INTO channels (space_id, name, is_public)
VALUES ($1, $2, $3)
ON CONFLICT DO NOTHING
RETURNING channels;