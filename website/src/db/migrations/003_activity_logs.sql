CREATE TYPE log_level AS ENUM ('info', 'warn', 'error');

CREATE TYPE log_action AS ENUM (
    'user_registered', 'user_logged_in', 'user_deleted',
    'clip_uploaded', 'clip_deleted', 'clip_viewed',
    'api_key_created', 'api_key_revoked',
    'admin_user_banned', 'admin_user_unbanned', 'admin_role_changed'
);

CREATE TABLE activity_logs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    action      log_action NOT NULL,
    level       log_level NOT NULL DEFAULT 'info',
    ip_address  TEXT,
    details     JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_logs_created_at ON activity_logs(created_at DESC);
CREATE INDEX idx_logs_user_id ON activity_logs(user_id);
CREATE INDEX idx_logs_action ON activity_logs(action);
