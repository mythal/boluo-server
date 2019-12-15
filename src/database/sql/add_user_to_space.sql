INSERT INTO space_members (user_id, space_id, is_admin, is_master)
VALUES ($1, $2, false, false)
ON CONFLICT DO NOTHING
RETURNING space_members;