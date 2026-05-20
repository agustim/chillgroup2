-- Crear taula channel_read_state
CREATE TABLE IF NOT EXISTS channel_read_state (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    last_read_message_id UUID,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, channel_id)
);

CREATE INDEX IF NOT EXISTS idx_channel_read_state_user ON channel_read_state(user_id);
CREATE INDEX IF NOT EXISTS idx_channel_read_state_channel ON channel_read_state(channel_id);
