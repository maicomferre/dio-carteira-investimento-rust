use std::net::IpAddr;

use sha2::{Digest, Sha256};

pub const AUTH_LOGIN_FAILED: &str = "auth.login_failed";
pub const AUTH_LOGIN_RATE_LIMITED: &str = "auth.login_rate_limited";
pub const HTTP_RATE_LIMITED: &str = "http.rate_limited";
pub const HTTP_CONCURRENCY_SATURATED: &str = "http.concurrency_saturated";
pub const HTTP_SERVER_ERROR: &str = "http.server_error";
pub const DB_READINESS_FAILED: &str = "db.readiness_failed";

pub fn username_fingerprint(username_normalized: &str) -> String {
    let digest = Sha256::digest(username_normalized.as_bytes());
    hex::encode(&digest[..8])
}

pub fn log_login_failed(client_ip: IpAddr, username_normalized: &str) {
    tracing::warn!(
        event = AUTH_LOGIN_FAILED,
        client_ip = %client_ip,
        username_fingerprint = %username_fingerprint(username_normalized),
        "falha de login registrada"
    );
}

pub fn log_login_rate_limited(client_ip: IpAddr, username_normalized: &str) {
    tracing::warn!(
        event = AUTH_LOGIN_RATE_LIMITED,
        client_ip = %client_ip,
        username_fingerprint = %username_fingerprint(username_normalized),
        "login bloqueado por rate limit"
    );
}

pub fn log_http_rate_limited(client_ip: IpAddr, scope: &'static str, path: &str) {
    let path = if path.is_empty() {
        "<not-captured>"
    } else {
        path
    };

    tracing::warn!(
        event = HTTP_RATE_LIMITED,
        client_ip = %client_ip,
        scope,
        path,
        "requisição bloqueada por rate limit"
    );
}

pub fn log_concurrency_saturated(client_ip: IpAddr, path: &str) {
    tracing::warn!(
        event = HTTP_CONCURRENCY_SATURATED,
        client_ip = %client_ip,
        path,
        "requisição bloqueada por limite de concorrência"
    );
}

pub fn log_db_readiness_failed(error: &sqlx::Error) {
    tracing::error!(
        event = DB_READINESS_FAILED,
        error = %error,
        "readiness falhou ao consultar o banco"
    );
}

pub fn log_server_error(status: u16, code: &'static str, correlation_id: &str) {
    tracing::error!(
        event = HTTP_SERVER_ERROR,
        status,
        code,
        correlation_id,
        "resposta de erro 5xx emitida"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_fingerprint_is_stable_and_does_not_expose_username() {
        let fingerprint = username_fingerprint("maicom");

        assert_eq!(fingerprint, username_fingerprint("maicom"));
        assert_ne!(fingerprint, username_fingerprint("outro"));
        assert_eq!(fingerprint.len(), 16);
        assert!(!fingerprint.contains("maicom"));
    }
}
