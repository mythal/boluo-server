WITH add(channel_members) AS (
    INSERT INTO channel_members (user_id, channel_id, character_name)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        RETURNING channel_members
)
SELECT true AS created, channel_members FROM add
UNION ALL
SELECT false AS created, channel_members FROM channel_members
WHERE user_id = $1 AND channel_id = $2
LIMIT 1;
