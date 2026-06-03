ALTER TABLE plans
    ADD COLUMN max_streaming_hours_monthly INT NOT NULL DEFAULT -1;

UPDATE plans SET max_streaming_hours_monthly = 10  WHERE name = 'free';
UPDATE plans SET max_streaming_hours_monthly = 50  WHERE name = 'pro';
UPDATE plans SET max_streaming_hours_monthly = -1  WHERE name = 'enterprise';
