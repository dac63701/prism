-- One-time recovery for databases initialized before migration 001 changed.
-- Run only after taking a tested backup. This deliberately accepts exactly the
-- known legacy migration checksum and refuses every other database state.
BEGIN;

LOCK TABLE _sqlx_migrations IN ACCESS EXCLUSIVE MODE;

DO $$
DECLARE
    changed INTEGER;
BEGIN
    UPDATE _sqlx_migrations
    SET checksum = decode(
        'c080de1038ff40c2e202a9b8c3cd91d9c74541a5854f17bdb182c763afccf1959d4fd9ce654956ad0495cd4b25c25b5f',
        'hex'
    )
    WHERE version = 1
      AND success = TRUE
      AND checksum = decode(
        'e9b7ba69023760e6ec193df709e812e3b0573622407cbf03d549d68c079216a61346e8565877100f5758c9eb4873e1c2',
        'hex'
      );

    GET DIAGNOSTICS changed = ROW_COUNT;
    IF changed <> 1 THEN
        RAISE EXCEPTION
            'Expected exactly one successful migration 001 with the known legacy checksum; changed % rows',
            changed;
    END IF;
END $$;

COMMIT;
