use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub mod api_keys;
pub mod clips;
pub mod config;
pub mod tags;
pub mod users;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;

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

    // Ensures the columns exist even if migration 002 was corrupted
    // in a stale Docker build. DO block swallows "duplicate_column"
    // so these are safe to run every startup.
    sqlx::query(
        r#"DO $$ BEGIN
               ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;
           EXCEPTION WHEN duplicate_column THEN NULL;
           END $$;"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"DO $$ BEGIN
               ALTER TABLE users ADD COLUMN verification_token TEXT;
           EXCEPTION WHEN duplicate_column THEN NULL;
           END $$;"#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
