ALTER TABLE devices
    ADD COLUMN kem_public_key TEXT NOT NULL DEFAULT '';

ALTER TABLE devices
    ADD COLUMN dsa_public_key TEXT NOT NULL DEFAULT '';

UPDATE devices
SET kem_public_key = COALESCE(public_key, '')
WHERE kem_public_key = '';

CREATE TABLE channel_key_device_bundles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_version_id UUID NOT NULL REFERENCES channel_key_versions(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_key TEXT NOT NULL,
    kem_ciphertext TEXT NOT NULL,
    signature TEXT,
    signed_by_device_id UUID REFERENCES devices(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (key_version_id, device_id)
);

CREATE INDEX idx_channel_key_device_bundles_key_version ON channel_key_device_bundles(key_version_id);
CREATE INDEX idx_channel_key_device_bundles_device ON channel_key_device_bundles(device_id);

INSERT INTO channel_key_device_bundles (id, key_version_id, device_id, encrypted_key, kem_ciphertext, created_at)
SELECT gen_random_uuid(), ckv.id, ck.device_id, ck.encrypted_key, COALESCE(ck.kem_ciphertext, ''), ck.created_at
FROM channel_keys ck
JOIN channel_key_versions ckv ON ckv.channel_id = ck.channel_id AND ckv.deprecated_at IS NULL;

DROP TABLE channel_keys;