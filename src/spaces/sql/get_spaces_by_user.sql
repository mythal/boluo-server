SELECT s, sm
FROM space_members sm
    INNER JOIN spaces s ON sm.space_id = s.id
WHERE sm.user_id = $1;
