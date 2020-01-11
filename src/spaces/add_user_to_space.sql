WITH add(space_members) AS (
    INSERT INTO space_members (user_id, space_id, is_admin, is_master)
        VALUES ($1, $2, $3, false)
        ON CONFLICT DO NOTHING
        RETURNING space_members
)
SELECT true, space_members FROM add
UNION ALL
SELECT false, space_members FROM space_members
WHERE user_id = $1 AND space_id = $2
LIMIT 1;
