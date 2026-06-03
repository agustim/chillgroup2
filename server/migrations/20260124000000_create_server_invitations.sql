-- Invitacions de servidor amb flux d'acceptació (pendent/acceptada/declinada).
-- Permet que un usuari accepti o declini explícitament una invitació a un servidor.

CREATE TABLE server_invitations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    inviter_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invitee_id  UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      VARCHAR(10) NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending', 'accepted', 'declined')),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_server_invitations_unique_pending
    ON server_invitations(server_id, invitee_id)
    WHERE status = 'pending';

CREATE INDEX idx_server_invitations_invitee
    ON server_invitations(invitee_id, status);

CREATE INDEX idx_server_invitations_server
    ON server_invitations(server_id, status);
