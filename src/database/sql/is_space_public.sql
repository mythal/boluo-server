SELECT is_public
FROM spaces
WHERE id = $1
LIMIT 1;