ALTER TABLE media ALTER COLUMN "created" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE users ALTER COLUMN "joined" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE spaces ALTER COLUMN "created" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE spaces ALTER COLUMN "modified" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE space_members ALTER COLUMN "join_date" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE channels ALTER COLUMN "created" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE channel_members ALTER COLUMN "join_date" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE messages ALTER COLUMN "created" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE messages ALTER COLUMN "modified" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE messages ALTER COLUMN "order_date" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE restrained_members ALTER COLUMN "restrained_date" SET DEFAULT (now() at time zone 'utc');
ALTER TABLE events ALTER COLUMN "created" SET DEFAULT (now() at time zone 'utc');
