SELECT spaces
FROM spaces
WHERE deleted = false AND name LIKE ALL ($1)
LIMIT 1024;