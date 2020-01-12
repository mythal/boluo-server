SELECT (password = crypt($3, password)), users
FROM users
WHERE CASE
          WHEN $1 IS NOT NULL THEN email = $1
          WHEN $2 IS NOT NULL THEN username = $2 END
  AND deactivated = false
LIMIT 1;
