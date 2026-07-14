use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub mod clips;
pub mod config;
pub mod tags;
pub mod users;

async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    if let Err(error) = sqlx::migrate!("src/db/migrations").run(pool).await {
        tracing::warn!(
            error = %error,
            "migration failed; attempting known legacy checksum recovery"
        );

        sqlx::query(
            r#"DO $$
               DECLARE changed INTEGER;
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
                       RAISE EXCEPTION 'No known legacy migration checksum found';
                   END IF;
               END $$;"#,
        )
        .execute(pool)
        .await?;

        sqlx::migrate!("src/db/migrations").run(pool).await?;
    }

    Ok(())
}

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    run_migrations(&pool).await?;

    sqlx::query(
        r#"DO $$
           BEGIN
               IF (SELECT data_type FROM information_schema.columns
                   WHERE table_name = 'clips' AND column_name = 'duration_secs') = 'real' THEN
                   ALTER TABLE clips ALTER COLUMN duration_secs TYPE DOUBLE PRECISION;
               END IF;
           END $$;"#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
