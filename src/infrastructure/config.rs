use std::{env, net::SocketAddr, str::FromStr, time::Duration};

use anyhow::{Context, bail};

use crate::infrastructure::instrument_provider::InstrumentProviderConfig;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app: GeneralConfig,
    pub http: HttpConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub instrument_provider: InstrumentProviderConfig,
}

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub environment: String,
    pub log_format: LogFormat,
    pub log_level: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Compact,
    Json,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub bind_addr: SocketAddr,
    pub request_timeout: Duration,
    pub max_body_bytes: usize,
    pub max_concurrent_requests: usize,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub session_hash_key: String,
    pub session_ttl_seconds: u64,
    pub cookie_secure: bool,
    pub allowed_origins: Vec<String>,
    pub global_rate_limit_max_requests: u32,
    pub global_rate_limit_window_seconds: u64,
    pub global_rate_limit_block_seconds: u64,
    pub login_rate_limit_max_attempts: u32,
    pub login_rate_limit_window_seconds: u64,
    pub login_rate_limit_block_seconds: u64,
    pub register_rate_limit_max_requests: u32,
    pub register_rate_limit_window_seconds: u64,
    pub register_rate_limit_block_seconds: u64,
    pub mutation_rate_limit_max_requests: u32,
    pub mutation_rate_limit_window_seconds: u64,
    pub mutation_rate_limit_block_seconds: u64,
    pub expired_session_retention_days: i64,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment = read_env("APP_ENV").unwrap_or_else(|| "development".to_owned());
        let log_level = read_env("RUST_LOG").unwrap_or_else(|| {
            "carteira_de_investimentos_maicaosa=info,tower_http=info".to_owned()
        });
        let log_format = match read_env("APP_LOG_FORMAT")
            .unwrap_or_else(|| "compact".to_owned())
            .as_str()
        {
            "compact" => LogFormat::Compact,
            "json" => LogFormat::Json,
            other => bail!("APP_LOG_FORMAT inválido: {other}"),
        };

        let bind_addr = read_env("APP_BIND_ADDR")
            .unwrap_or_else(|| "127.0.0.1:3000".to_owned())
            .parse()
            .context("APP_BIND_ADDR inválido")?;
        let request_timeout_seconds = read_env_parse("APP_REQUEST_TIMEOUT_SECONDS", 10)?;
        if !(1..=60).contains(&request_timeout_seconds) {
            bail!("APP_REQUEST_TIMEOUT_SECONDS deve ficar entre 1 e 60");
        }
        let max_body_bytes = read_env_parse("APP_MAX_BODY_BYTES", 1_048_576)?;
        if !(16_384..=10_485_760).contains(&max_body_bytes) {
            bail!("APP_MAX_BODY_BYTES deve ficar entre 16384 e 10485760");
        }
        let max_concurrent_requests = read_env_parse("APP_MAX_CONCURRENT_REQUESTS", 128)?;
        if !(1..=2_048).contains(&max_concurrent_requests) {
            bail!("APP_MAX_CONCURRENT_REQUESTS deve ficar entre 1 e 2048");
        }

        let url = read_env("DATABASE_URL").context("DATABASE_URL é obrigatório")?;
        if url.trim().is_empty() {
            bail!("DATABASE_URL não pode ser vazio");
        }

        let max_connections = read_env_parse("DATABASE_MAX_CONNECTIONS", 5)?;
        if max_connections == 0 || max_connections > 50 {
            bail!("DATABASE_MAX_CONNECTIONS deve ficar entre 1 e 50");
        }

        let acquire_timeout_seconds = read_env_parse("DATABASE_ACQUIRE_TIMEOUT_SECONDS", 3)?;
        if acquire_timeout_seconds == 0 || acquire_timeout_seconds > 30 {
            bail!("DATABASE_ACQUIRE_TIMEOUT_SECONDS deve ficar entre 1 e 30");
        }

        let jwt_secret = read_required_secret("AUTH_JWT_SECRET")?;
        let session_hash_key = read_required_secret("AUTH_SESSION_HASH_KEY")?;
        let jwt_issuer = read_env("AUTH_JWT_ISSUER")
            .unwrap_or_else(|| "carteira-de-investimentos-maicaosa".to_owned());
        let jwt_audience =
            read_env("AUTH_JWT_AUDIENCE").unwrap_or_else(|| "carteira-web".to_owned());
        let session_ttl_seconds = read_env_parse("AUTH_SESSION_TTL_SECONDS", 3_600)?;
        if !(300..=86_400).contains(&session_ttl_seconds) {
            bail!("AUTH_SESSION_TTL_SECONDS deve ficar entre 300 e 86400");
        }
        let cookie_secure = read_env_parse("AUTH_COOKIE_SECURE", environment != "development")?;
        let allowed_origins = read_csv_env("AUTH_ALLOWED_ORIGINS")
            .unwrap_or_else(|| default_allowed_origins(&environment));
        if environment != "development" && allowed_origins.is_empty() {
            bail!("AUTH_ALLOWED_ORIGINS é obrigatório fora do ambiente development");
        }

        let global_rate_limit_max_requests =
            read_env_parse("AUTH_GLOBAL_RATE_LIMIT_MAX_REQUESTS", 300)?;
        if !(10..=10_000).contains(&global_rate_limit_max_requests) {
            bail!("AUTH_GLOBAL_RATE_LIMIT_MAX_REQUESTS deve ficar entre 10 e 10000");
        }
        let global_rate_limit_window_seconds =
            read_env_parse("AUTH_GLOBAL_RATE_LIMIT_WINDOW_SECONDS", 60)?;
        if !(10..=3_600).contains(&global_rate_limit_window_seconds) {
            bail!("AUTH_GLOBAL_RATE_LIMIT_WINDOW_SECONDS deve ficar entre 10 e 3600");
        }
        let global_rate_limit_block_seconds =
            read_env_parse("AUTH_GLOBAL_RATE_LIMIT_BLOCK_SECONDS", 300)?;
        if !(30..=86_400).contains(&global_rate_limit_block_seconds) {
            bail!("AUTH_GLOBAL_RATE_LIMIT_BLOCK_SECONDS deve ficar entre 30 e 86400");
        }
        let login_rate_limit_max_attempts =
            read_env_parse("AUTH_LOGIN_RATE_LIMIT_MAX_ATTEMPTS", 5)?;
        if !(1..=20).contains(&login_rate_limit_max_attempts) {
            bail!("AUTH_LOGIN_RATE_LIMIT_MAX_ATTEMPTS deve ficar entre 1 e 20");
        }
        let login_rate_limit_window_seconds =
            read_env_parse("AUTH_LOGIN_RATE_LIMIT_WINDOW_SECONDS", 300)?;
        if !(30..=3_600).contains(&login_rate_limit_window_seconds) {
            bail!("AUTH_LOGIN_RATE_LIMIT_WINDOW_SECONDS deve ficar entre 30 e 3600");
        }
        let login_rate_limit_block_seconds =
            read_env_parse("AUTH_LOGIN_RATE_LIMIT_BLOCK_SECONDS", 900)?;
        if !(30..=86_400).contains(&login_rate_limit_block_seconds) {
            bail!("AUTH_LOGIN_RATE_LIMIT_BLOCK_SECONDS deve ficar entre 30 e 86400");
        }
        let register_rate_limit_max_requests =
            read_env_parse("AUTH_REGISTER_RATE_LIMIT_MAX_REQUESTS", 3)?;
        if !(1..=20).contains(&register_rate_limit_max_requests) {
            bail!("AUTH_REGISTER_RATE_LIMIT_MAX_REQUESTS deve ficar entre 1 e 20");
        }
        let register_rate_limit_window_seconds =
            read_env_parse("AUTH_REGISTER_RATE_LIMIT_WINDOW_SECONDS", 300)?;
        if !(30..=3_600).contains(&register_rate_limit_window_seconds) {
            bail!("AUTH_REGISTER_RATE_LIMIT_WINDOW_SECONDS deve ficar entre 30 e 3600");
        }
        let register_rate_limit_block_seconds =
            read_env_parse("AUTH_REGISTER_RATE_LIMIT_BLOCK_SECONDS", 900)?;
        if !(30..=86_400).contains(&register_rate_limit_block_seconds) {
            bail!("AUTH_REGISTER_RATE_LIMIT_BLOCK_SECONDS deve ficar entre 30 e 86400");
        }
        let mutation_rate_limit_max_requests =
            read_env_parse("AUTH_MUTATION_RATE_LIMIT_MAX_REQUESTS", 60)?;
        if !(1..=600).contains(&mutation_rate_limit_max_requests) {
            bail!("AUTH_MUTATION_RATE_LIMIT_MAX_REQUESTS deve ficar entre 1 e 600");
        }
        let mutation_rate_limit_window_seconds =
            read_env_parse("AUTH_MUTATION_RATE_LIMIT_WINDOW_SECONDS", 60)?;
        if !(10..=3_600).contains(&mutation_rate_limit_window_seconds) {
            bail!("AUTH_MUTATION_RATE_LIMIT_WINDOW_SECONDS deve ficar entre 10 e 3600");
        }
        let mutation_rate_limit_block_seconds =
            read_env_parse("AUTH_MUTATION_RATE_LIMIT_BLOCK_SECONDS", 300)?;
        if !(30..=86_400).contains(&mutation_rate_limit_block_seconds) {
            bail!("AUTH_MUTATION_RATE_LIMIT_BLOCK_SECONDS deve ficar entre 30 e 86400");
        }
        let expired_session_retention_days =
            read_env_parse("AUTH_EXPIRED_SESSION_RETENTION_DAYS", 7)?;
        if !(1..=365).contains(&expired_session_retention_days) {
            bail!("AUTH_EXPIRED_SESSION_RETENTION_DAYS deve ficar entre 1 e 365");
        }

        let instrument_provider_timeout_seconds =
            read_env_parse("INSTRUMENT_PROVIDER_TIMEOUT_SECONDS", 2)?;
        if !(1..=10).contains(&instrument_provider_timeout_seconds) {
            bail!("INSTRUMENT_PROVIDER_TIMEOUT_SECONDS deve ficar entre 1 e 10");
        }
        let instrument_provider_cache_ttl_seconds =
            read_env_parse("INSTRUMENT_PROVIDER_CACHE_TTL_SECONDS", 3_600)?;
        if !(60..=86_400).contains(&instrument_provider_cache_ttl_seconds) {
            bail!("INSTRUMENT_PROVIDER_CACHE_TTL_SECONDS deve ficar entre 60 e 86400");
        }
        let instrument_provider_stale_ttl_seconds =
            read_env_parse("INSTRUMENT_PROVIDER_STALE_TTL_SECONDS", 86_400)?;
        if instrument_provider_stale_ttl_seconds < instrument_provider_cache_ttl_seconds
            || instrument_provider_stale_ttl_seconds > 604_800
        {
            bail!(
                "INSTRUMENT_PROVIDER_STALE_TTL_SECONDS deve ser maior ou igual ao TTL fresco e no máximo 604800"
            );
        }
        let instrument_provider_max_results = read_env_parse("INSTRUMENT_PROVIDER_MAX_RESULTS", 8)?;
        if !(1..=20).contains(&instrument_provider_max_results) {
            bail!("INSTRUMENT_PROVIDER_MAX_RESULTS deve ficar entre 1 e 20");
        }

        Ok(Self {
            app: GeneralConfig {
                environment,
                log_format,
                log_level,
            },
            http: HttpConfig {
                bind_addr,
                request_timeout: Duration::from_secs(request_timeout_seconds),
                max_body_bytes,
                max_concurrent_requests,
            },
            database: DatabaseConfig {
                url,
                max_connections,
                acquire_timeout: Duration::from_secs(acquire_timeout_seconds),
            },
            auth: AuthConfig {
                jwt_secret,
                jwt_issuer,
                jwt_audience,
                session_hash_key,
                session_ttl_seconds,
                cookie_secure,
                allowed_origins,
                global_rate_limit_max_requests,
                global_rate_limit_window_seconds,
                global_rate_limit_block_seconds,
                login_rate_limit_max_attempts,
                login_rate_limit_window_seconds,
                login_rate_limit_block_seconds,
                register_rate_limit_max_requests,
                register_rate_limit_window_seconds,
                register_rate_limit_block_seconds,
                mutation_rate_limit_max_requests,
                mutation_rate_limit_window_seconds,
                mutation_rate_limit_block_seconds,
                expired_session_retention_days,
            },
            instrument_provider: InstrumentProviderConfig {
                timeout: Duration::from_secs(instrument_provider_timeout_seconds),
                cache_ttl: Duration::from_secs(instrument_provider_cache_ttl_seconds),
                stale_ttl: Duration::from_secs(instrument_provider_stale_ttl_seconds),
                max_results: instrument_provider_max_results,
            },
        })
    }
}

fn read_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn read_required_secret(key: &str) -> anyhow::Result<String> {
    let secret = read_env(key).with_context(|| format!("{key} é obrigatório"))?;
    if secret.len() < 32 {
        bail!("{key} deve ter pelo menos 32 caracteres");
    }

    Ok(secret)
}

fn read_csv_env(key: &str) -> Option<Vec<String>> {
    read_env(key).map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect()
    })
}

fn default_allowed_origins(environment: &str) -> Vec<String> {
    if environment == "development" {
        vec![
            "http://127.0.0.1:3000".to_owned(),
            "http://localhost:3000".to_owned(),
        ]
    } else {
        Vec::new()
    }
}

fn read_env_parse<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match read_env(key) {
        Some(value) => value
            .parse()
            .with_context(|| format!("{key} possui valor inválido")),
        None => Ok(default),
    }
}
