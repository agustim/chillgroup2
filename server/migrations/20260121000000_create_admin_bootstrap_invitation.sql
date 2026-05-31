CREATE TABLE IF NOT EXISTS admin_bootstrap_invitation (
    slot SMALLINT PRIMARY KEY,
    code_hash TEXT NOT NULL,
    consumed_by_user_id UUID,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_admin_bootstrap_invitation_code_hash
    ON admin_bootstrap_invitation(code_hash);
