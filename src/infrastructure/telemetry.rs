use crate::infrastructure::config::{AppConfig, LogFormat};

pub fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_new(&config.app.log_level)?;

    match config.app.log_format {
        LogFormat::Compact => tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .compact()
            .init(),
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .init(),
    }

    Ok(())
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("falha ao instalar handler de Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("falha ao instalar handler SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("sinal de encerramento recebido");
}
