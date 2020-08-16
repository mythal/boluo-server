UPDATE messages
SET name         = COALESCE($2, name),
    text         = COALESCE($3, text),
    entities     = COALESCE($4, entities),
    in_game      = COALESCE($5, in_game),
    is_action    = COALESCE($6, is_action),
    folded       = COALESCE($7, folded),
    order_date   = COALESCE($8, order_date),
    order_offset = COALESCE($9, order_offset),
    modified     = now()
WHERE id = $1
RETURNING messages;
