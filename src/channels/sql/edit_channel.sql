UPDATE channels
SET name = COALESCE($2, name), topic = COALESCE($3, topic), default_dice_type = COALESCE($4, default_dice_type)
WHERE id = $1
RETURNING channels;
