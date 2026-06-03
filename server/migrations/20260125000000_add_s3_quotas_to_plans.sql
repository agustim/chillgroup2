ALTER TABLE plans
    ADD COLUMN max_storage_bytes BIGINT NOT NULL DEFAULT -1,
    ADD COLUMN max_transfer_bytes_monthly BIGINT NOT NULL DEFAULT -1;

UPDATE plans SET max_storage_bytes = 10737418240,   max_transfer_bytes_monthly = 107374182400 WHERE name = 'free';
UPDATE plans SET max_storage_bytes = 53687091200,   max_transfer_bytes_monthly = 536870912000 WHERE name = 'pro';
UPDATE plans SET max_storage_bytes = -1,            max_transfer_bytes_monthly = -1           WHERE name = 'enterprise';
