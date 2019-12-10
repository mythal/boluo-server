INSERT INTO spaces (name, owner_id, password)
VALUES ($1, $2, COALESCE($3, ''))
ON CONFLICT DO NOTHING
RETURNING spaces;