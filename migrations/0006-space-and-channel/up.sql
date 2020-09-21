ALTER TABLE channels ADD COLUMN "default_roll_command" text NOT NULL DEFAULT 'd';
ALTER TABLE spaces ADD COLUMN "invite_token" uuid NOT NULL DEFAULT gen_random_uuid();
ALTER TABLE spaces ADD COLUMN "allow_spectator" boolean NOT NULL DEFAULT true;
