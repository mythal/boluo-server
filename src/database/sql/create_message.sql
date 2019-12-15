INSERT INTO messages (sender_id, channel_id, name, text)
VALUES ($1, $2, $3, $4)
RETURNING messages;