INSERT INTO media (mime_type, uploader_id, filename, original_filename, hash, size)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING media;
