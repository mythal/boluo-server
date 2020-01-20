INSERT INTO channels (space_id, name, is_public)
VALUES ($1, $2, $3)
RETURNING channels;
