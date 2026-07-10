use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub mod api_keys;
pub mod clips;
pub mod users;

pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;

    Ok(pool)
}
