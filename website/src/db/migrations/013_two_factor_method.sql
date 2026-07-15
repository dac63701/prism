ALTER TABLE users ADD COLUMN IF NOT EXISTS two_factor_method TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_2fa_code TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_2fa_code_expires_at TIMESTAMPTZ;
UPDATE users SET two_factor_method = 'totp' WHERE totp_enabled = true;
