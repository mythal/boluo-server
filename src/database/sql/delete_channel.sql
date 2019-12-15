DELETE
FROM channels
WHERE id = $1
RETURNING channels;