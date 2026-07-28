use carteira_de_investimentos_maicaosa::{
    infrastructure::{
        auth_repository::AuthRepository,
        config::AppConfig,
        database::create_pool,
        telemetry::{init_tracing, shutdown_signal},
    },
    presentation::http::build_router,
};
use time::{Duration, OffsetDateTime};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::from_path(".env").ok();

    let config = AppConfig::from_env()?;
    init_tracing(&config)?;

    let pool = create_pool(&config.database).await?;
    let auth_repository = AuthRepository::new(pool.clone());
    let expired_session_cutoff =
        OffsetDateTime::now_utc() - Duration::days(config.auth.expired_session_retention_days);
    let deleted_expired_sessions = auth_repository
        .delete_expired_sessions_before(expired_session_cutoff)
        .await?;
    tracing::info!(
        deleted_expired_sessions,
        retention_days = config.auth.expired_session_retention_days,
        "limpeza inicial de sessões expiradas concluída"
    );

    let app = build_router(
        pool,
        config.auth.clone(),
        config.instrument_provider.clone(),
    );
    let listener = tokio::net::TcpListener::bind(config.http.bind_addr).await?;

    tracing::info!(
        address = %listener.local_addr()?,
        environment = %config.app.environment,
        "servidor iniciado"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}
