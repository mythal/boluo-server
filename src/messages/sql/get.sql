SELECT CASE
           WHEN msg.whisper_to_users IS NOT NULL AND sm.is_master IS NOT true AND $2 <> ALL (msg.whisper_to_users)
               THEN msg.hide
           ELSE msg END
FROM messages msg
         INNER JOIN channels ch ON msg.channel_id = ch.id
         LEFT JOIN space_members sm ON ch.space_id = sm.space_id AND sm.user_id = $2
WHERE msg.id = $1
  AND msg.deleted = false
LIMIT 1;
