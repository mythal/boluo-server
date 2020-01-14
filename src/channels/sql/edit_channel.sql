UPDATE channels
SET name = COALESCE($2, name)
WHERE id = $1
RETURNING channels;
