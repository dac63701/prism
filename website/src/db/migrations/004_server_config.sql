CREATE TABLE IF NOT EXISTS server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO server_config (key, value) VALUES
    ('max_upload_size_mb', '500'),
    ('default_max_storage_gb', '10'),
    ('rate_limit_per_min', '100'),
    ('signups_allowed', 'true')
ON CONFLICT (key) DO NOTHING;
