UPDATE spaces
SET name = COALESCE($2, name), description = COALESCE($3, description)
WHERE id = $1
RETURNING spaces;
