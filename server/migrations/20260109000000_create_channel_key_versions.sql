-- Nivell 1: versions de clau de canal simètrica (xifrada amb master key del servidor)
CREATE TABLE channel_key_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    encrypted_key TEXT NOT NULL,
    nonce TEXT NOT NULL,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deprecated_at TIMESTAMPTZ,
    UNIQUE (channel_id, version)
);

CREATE INDEX idx_channel_key_versions_channel ON channel_key_versions(channel_id);
CREATE INDEX idx_channel_key_versions_channel_version ON channel_key_versions(channel_id, version DESC);
