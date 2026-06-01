CREATE TABLE IF NOT EXISTS attachments (
    id UUID PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    uploader_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    uploader_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    object_key TEXT NOT NULL,
    status TEXT NOT NULL,
    upload_id TEXT NOT NULL,
    chunk_size_bytes BIGINT NOT NULL,
    chunk_count INTEGER NOT NULL,
    algorithm TEXT,
    file_iv TEXT,
    wrapped_file_key TEXT,
    key_version_id UUID REFERENCES channel_key_versions(id),
    key_version INTEGER,
    ciphertext_sha256 TEXT,
    completed_at TIMESTAMPTZ,
    thumbnail_attachment_id UUID REFERENCES attachments(id)
);

CREATE INDEX IF NOT EXISTS idx_attachments_channel_id ON attachments(channel_id);
CREATE INDEX IF NOT EXISTS idx_attachments_status ON attachments(status);
CREATE INDEX IF NOT EXISTS idx_attachments_key_version_id ON attachments(key_version_id);

CREATE TABLE IF NOT EXISTS message_attachments (
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    attachment_id UUID NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    PRIMARY KEY (message_id, attachment_id)
);

CREATE INDEX IF NOT EXISTS idx_message_attachments_attachment_id ON message_attachments(attachment_id);
