use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

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

    Ok(pool)
}
