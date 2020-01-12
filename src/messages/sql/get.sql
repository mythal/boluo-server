SELECT messages
FROM messages
WHERE id = $1 AND deleted = false
LIMIT 1;
