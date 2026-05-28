ALTER TABLE channel_members
    ADD COLUMN IF NOT EXISTS permission_level INTEGER NOT NULL DEFAULT 2;

UPDATE channel_members
SET permission_level = 2
WHERE permission_level IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_channel_members_permission_level'
    ) THEN
        ALTER TABLE channel_members
            ADD CONSTRAINT chk_channel_members_permission_level
            CHECK (permission_level BETWEEN 1 AND 3);
    END IF;
END $$;
