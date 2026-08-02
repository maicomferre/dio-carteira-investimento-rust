use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::infrastructure::config::DatabaseConfig;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn create_pool(config: &DatabaseConfig) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.url)
        .await
        .map_err(Into::into)
}

pub async fn run_migrations(config: &DatabaseConfig) -> anyhow::Result<()> {
    let pool = create_pool(config).await?;
    MIGRATOR.run(&pool).await?;
    pool.close().await;

    Ok(())
}
