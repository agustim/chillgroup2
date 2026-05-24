-- Preparar schema per DM 1:1 sobre channels (fase incremental, sense tallar flux legacy).
-- Aquesta migració només afegeix metadades DM i índexs.

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS scope VARCHAR(10) NOT NULL DEFAULT 'server';

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS dm_user_a_id UUID REFERENCES users(id);

ALTER TABLE channels
    ADD COLUMN IF NOT EXISTS dm_user_b_id UUID REFERENCES users(id);

ALTER TABLE channels
    ALTER COLUMN server_id DROP NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_channels_scope'
    ) THEN
        ALTER TABLE channels
            ADD CONSTRAINT chk_channels_scope CHECK (scope IN ('server', 'dm'));
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_channels_dm_pair'
    ) THEN
        ALTER TABLE channels
            ADD CONSTRAINT chk_channels_dm_pair CHECK (
                (scope = 'server' AND dm_user_a_id IS NULL AND dm_user_b_id IS NULL)
                OR
                (scope = 'dm' AND dm_user_a_id IS NOT NULL AND dm_user_b_id IS NOT NULL AND dm_user_a_id <> dm_user_b_id)
            );
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_channels_scope ON channels(scope);

CREATE UNIQUE INDEX IF NOT EXISTS idx_channels_dm_pair
ON channels (
    LEAST(dm_user_a_id, dm_user_b_id),
    GREATEST(dm_user_a_id, dm_user_b_id)
)
WHERE scope = 'dm';
