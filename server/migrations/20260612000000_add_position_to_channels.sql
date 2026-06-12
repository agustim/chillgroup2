-- Add position field for custom channel ordering
ALTER TABLE channels ADD COLUMN position INTEGER DEFAULT 0;

-- Set positions based on existing creation order within each server
UPDATE channels SET position = (
    SELECT COUNT(*) FROM channels c2
    WHERE c2.server_id = channels.server_id
    AND c2.created_at <= channels.created_at
) - 1;
