ALTER TABLE invitations
ADD COLUMN IF NOT EXISTS server_id UUID REFERENCES servers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_invitations_server_id ON invitations(server_id);
