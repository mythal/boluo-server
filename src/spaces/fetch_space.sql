SELECT s, u
FROM spaces s
         LEFT JOIN users u on CASE WHEN $3 THEN s.owner_id = u.id END
WHERE deleted = false
  AND CASE WHEN $1 IS NOT NULL THEN s.id = $1 WHEN $2 IS NOT NULL THEN s.name = $2 END
LIMIT 1;
