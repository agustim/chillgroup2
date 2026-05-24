-- Crear taula friendships
CREATE TABLE friendships (
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    friend_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_user_id, friend_user_id)
);

CREATE INDEX idx_friendships_owner_user_id ON friendships(owner_user_id);
CREATE INDEX idx_friendships_friend_user_id ON friendships(friend_user_id);