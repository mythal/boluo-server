UPDATE messages
SET
    text = COALESCE($2, text),
    entities = COALESCE($3, entities),
    in_game = COALESCE($4, in_game),
    is_action = COALESCE($5, is_action),
    modified = now()
WHERE id = $1
RETURNING messages;
