CREATE TABLE users_extension
(
    "user_id"   uuid    NOT NULL PRIMARY KEY
        CONSTRAINT "extension_user" REFERENCES users (id) ON DELETE CASCADE,
    "settings"  jsonb   NOT NULL DEFAULT '{}'
);
