ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS sender_username TEXT NOT NULL DEFAULT '';

UPDATE messages m
SET sender_username = u.username
FROM users u
WHERE m.sender_user_id = u.id
  AND m.sender_username = '';