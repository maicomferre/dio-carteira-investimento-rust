CREATE TABLE brokers (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name VARCHAR(120) NOT NULL,
    name_normalized VARCHAR(120) NOT NULL,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at TIMESTAMPTZ,
    CONSTRAINT brokers_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT brokers_name_normalized_not_blank CHECK (btrim(name_normalized) <> ''),
    CONSTRAINT brokers_version_positive CHECK (version > 0),
    CONSTRAINT brokers_archived_at_consistent CHECK (
        (is_archived = FALSE AND archived_at IS NULL)
        OR (is_archived = TRUE AND archived_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX brokers_user_active_name_unique
    ON brokers (user_id, name_normalized)
    WHERE is_archived = FALSE;

CREATE INDEX brokers_user_id_idx
    ON brokers (user_id);
