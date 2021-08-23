ALTER TABLE messages DROP CONSTRAINT pos_unique;
ALTER TABLE channels DROP COLUMN "serial";
ALTER TABLE messages DROP COLUMN "pos";
