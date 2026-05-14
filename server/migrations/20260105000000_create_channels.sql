-- Crear taula channels
CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    channel_type VARCHAR(10) NOT NULL CHECK (channel_type IN ('text', 'voice')),
    encryption_type VARCHAR(10) NOT NULL DEFAULT 'none' CHECK (encryption_type IN ('none', 'symmetric', 'asymmetric')),
    message_ttl INTEGER,
    is_private BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_channels_server ON channels(server_id);
CREATE UNIQUE INDEX idx_channels_server_name ON channels(server_id, name);