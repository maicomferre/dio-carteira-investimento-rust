CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_id_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ,
    user_agent_hash TEXT,
    ip_hash TEXT,
    CONSTRAINT auth_sessions_token_id_hash_not_blank CHECK (btrim(token_id_hash) <> ''),
    CONSTRAINT auth_sessions_expiration_after_creation CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX auth_sessions_token_id_hash_unique
    ON auth_sessions (token_id_hash);

CREATE INDEX auth_sessions_user_id_idx
    ON auth_sessions (user_id);

CREATE INDEX auth_sessions_active_idx
    ON auth_sessions (user_id, expires_at)
    WHERE revoked_at IS NULL;
