CREATE TYPE user_role AS ENUM ('user', 'admin');
CREATE TYPE clip_visibility AS ENUM ('public', 'private', 'unlisted');

CREATE TABLE users (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email              TEXT NOT NULL UNIQUE,
    password_hash      TEXT NOT NULL,
    display_name       TEXT NOT NULL DEFAULT '',
    role               user_role NOT NULL DEFAULT 'user',
    storage_used_bytes BIGINT NOT NULL DEFAULT 0,
    max_storage_bytes  BIGINT NOT NULL DEFAULT 10737418240,
    is_banned          BOOLEAN NOT NULL DEFAULT false,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE clips (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_filename TEXT NOT NULL,
    storage_path      TEXT NOT NULL,
    thumbnail_path    TEXT,
    share_id          TEXT NOT NULL UNIQUE DEFAULT substr(replace(gen_random_uuid()::text, '-', ''), 1, 12),
    title             TEXT NOT NULL DEFAULT '',
    game              TEXT NOT NULL DEFAULT '',
    duration_secs     REAL NOT NULL DEFAULT 0,
    size_bytes        BIGINT NOT NULL DEFAULT 0,
    width             INT NOT NULL DEFAULT 0,
    height            INT NOT NULL DEFAULT 0,
    codec             TEXT NOT NULL DEFAULT 'h264',
    visibility        clip_visibility NOT NULL DEFAULT 'unlisted',
    download_count    INT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE clip_tags (
    clip_id UUID NOT NULL REFERENCES clips(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    PRIMARY KEY (clip_id, tag)
);

CREATE INDEX idx_clips_user_id ON clips(user_id);
CREATE INDEX idx_clips_share_id ON clips(share_id);
CREATE INDEX idx_clips_visibility ON clips(visibility) WHERE visibility != 'private';
CREATE INDEX idx_clips_game ON clips(game);
CREATE INDEX idx_clips_created_at ON clips(created_at DESC);
