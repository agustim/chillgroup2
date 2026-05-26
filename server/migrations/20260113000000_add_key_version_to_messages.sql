-- Afegir versió de clau per missatge per resoldre decrypt segons clau activa.
ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS key_version INTEGER;
