DELETE
FROM spaces
WHERE id = $1
RETURNING spaces;
