UPDATE spaces
SET name = COALESCE($2, name), description = COALESCE($3, description), default_dice_type = COALESCE($4, default_dice_type), explorable = COALESCE($5, explorable)
WHERE id = $1
RETURNING spaces;
