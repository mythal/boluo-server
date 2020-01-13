SELECT CASE
           WHEN msg.whisper_to_users IS NOT NULL AND cm.is_master IS NOT true AND $2 <> ALL (msg.whisper_to_users)
               THEN msg.hide
           ELSE msg END
FROM messages msg
         LEFT JOIN channel_members cm ON cm.channel_id = msg.channel_id AND cm.user_id = $2
WHERE msg.id = $1
  AND msg.deleted = false
LIMIT 1;
