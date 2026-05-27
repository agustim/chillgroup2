CREATE TABLE plans (
    id UUID PRIMARY KEY,
    name VARCHAR(32) UNIQUE NOT NULL,
    display_name VARCHAR(64) NOT NULL,
    description TEXT,
    max_servers INT NOT NULL,
    max_channels_text_per_server INT NOT NULL,
    max_channels_voice_per_server INT NOT NULL,
    max_members_per_server INT NOT NULL,
    api_calls_per_minute INT NOT NULL,
    messages_per_day INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_plans_name ON plans(name);
