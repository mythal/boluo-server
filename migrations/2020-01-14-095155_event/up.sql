CREATE TYPE event_type AS ENUM (
    'Joined',
    'Left',
    'NewMaster',
    'NewAdmin'
);

CREATE TABLE events
(
    "id" uuid NOT NULL PRIMARY KEY,
    "type" event_type NOT NULL,
    "channel_id" uuid DEFAULT NULL CONSTRAINT event_channel REFERENCES channels (id) ON DELETE CASCADE,
    "space_id" uuid DEFAULT NULL CONSTRAINT event_space REFERENCES spaces (id) ON DELETE CASCADE,
    "receiver_id" uuid CONSTRAINT event_receiver REFERENCES users (id) ON DELETE CASCADE,
    "payload" jsonb NOT NULL DEFAULT '{}',
    "created" timestamp NOT NULL default now()
);
