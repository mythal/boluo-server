DELETE
FROM channel_members m
USING spaces s
WHERE m.user_id = $1
  AND s.id = $2;
