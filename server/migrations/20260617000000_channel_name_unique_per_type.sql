-- Els noms de canal són únics dins de cada tipus (text/veu) per separat.
-- Un canal de text i un de veu poden compartir nom; dos de text (o dos de veu) no.
-- Abans la unicitat era només (server_id, name), que bloquejava text+veu amb el mateix nom.
DROP INDEX IF EXISTS idx_channels_server_name;
CREATE UNIQUE INDEX idx_channels_server_type_name ON channels(server_id, channel_type, name);
