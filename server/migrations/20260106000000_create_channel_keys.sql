-- Crear taula channel_keys
CREATE TABLE channel_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_key TEXT NOT NULL,
    kem_ciphertext TEXT NOT NULL,
    encryption_type VARCHAR(10) NOT NULL DEFAULT 'none',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(channel_id, device_id)
);

CREATE INDEX idx_channel_keys_channel ON channel_keys(channel_id);
CREATE INDEX idx_channel_keys_device ON channel_keys(device_id);