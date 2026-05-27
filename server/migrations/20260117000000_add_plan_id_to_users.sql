ALTER TABLE users
ADD COLUMN plan_id UUID REFERENCES plans(id);

CREATE INDEX idx_users_plan_id ON users(plan_id);
