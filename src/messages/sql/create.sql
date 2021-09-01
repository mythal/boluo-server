WITH last AS (
    (
        SELECT floor(m.pos + 1.0) AS pos
        FROM messages m
        WHERE m.channel_id = $3
        ORDER BY m.pos DESC
        LIMIT 1
    ) UNION ALL (SELECT 42.0 AS pos)
)
INSERT INTO messages (
    id,
    sender_id,
    channel_id,
    name,
    text,
    entities,
    in_game,
    is_action,
    is_master,
    whisper_to_users,
    media_id,
    pos
)
SELECT 
    COALESCE($1, uuid_generate_v1mc()) AS id,
    $2 AS sender_id,
    $3 AS channel_id,
    $4 AS name,
    $5 AS text,
    $6 AS entities,
    $7 AS in_game,
    $8 AS is_action,
    $9 AS is_master,
    $10 AS whisper_to_users,
    $11 AS media_id,
    COALESCE($12, pos)
FROM last
LIMIT 1
RETURNING messages;
