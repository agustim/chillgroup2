CREATE TABLE user_streaming_usage_monthly (
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year_month  CHAR(7)     NOT NULL,
    streaming_seconds BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, year_month)
);

CREATE INDEX idx_streaming_usage_user ON user_streaming_usage_monthly(user_id);
