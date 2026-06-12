-- Add position field for custom channel ordering
ALTER TABLE channels ADD COLUMN position INTEGER DEFAULT 0;

-- Set positions based on existing creation order within each server
-- group by server_id and assign position
UPDATE channels
SET position = (
    SELECT ROW_NUMBER() OVER (PARTITION BY c.server_id ORDER BY c.created_at ASC)
    FROM channels c
    WHERE c.id = channels.id
);
