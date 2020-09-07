UPDATE channels
SET deleted = true
WHERE id = $1 AND deleted = false;
