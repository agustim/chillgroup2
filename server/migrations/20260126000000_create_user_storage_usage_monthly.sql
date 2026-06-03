CREATE TABLE user_storage_usage_monthly (
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year_month  CHAR(7)     NOT NULL,  -- format: '2026-06'
    stored_bytes    BIGINT  NOT NULL DEFAULT 0,
    transfer_bytes  BIGINT  NOT NULL DEFAULT 0,
    warning_sent_at_80  TIMESTAMPTZ,
    warning_sent_at_90  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, year_month)
);

CREATE INDEX idx_storage_usage_user ON user_storage_usage_monthly(user_id);
