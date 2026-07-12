ALTER TABLE clips
    ALTER COLUMN share_id
    SET DEFAULT substr(replace(gen_random_uuid()::text, '-', ''), 1, 12);
