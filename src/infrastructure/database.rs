use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::infrastructure::config::DatabaseConfig;

pub async fn create_pool(config: &DatabaseConfig) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(&config.url)
        .await
        .map_err(Into::into)
}
