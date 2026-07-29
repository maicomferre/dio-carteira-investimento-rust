use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::user::{NormalizedUsername, validate_password},
    infrastructure::{
        auth_repository::{AuthRepository, CreateSessionInput, CreateUserInput, UserRecord},
        config::AuthConfig,
    },
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct LoginSession {
    pub user: PublicUser,
    pub token: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    sid: String,
    jti: String,
    iss: String,
    aud: String,
    exp: i64,
    nbf: i64,
    iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
}

pub async fn register_user(
    repository: &AuthRepository,
    username: String,
    password: String,
) -> Result<PublicUser, AppError> {
    let normalized = NormalizedUsername::parse(&username)
        .map_err(|_| AppError::Validation("username inválido"))?;
    validate_password(&password).map_err(|_| AppError::Validation("senha inválida"))?;

    let password_hash = hash_password(&password)?;
    let user = repository
        .create_user(CreateUserInput {
            id: Uuid::new_v4(),
            username: username.trim().to_owned(),
            username_normalized: normalized.as_str().to_owned(),
            password_hash,
        })
        .await?;

    Ok(public_user(user))
}

pub async fn login_user(
    repository: &AuthRepository,
    config: &AuthConfig,
    username: String,
    password: String,
) -> Result<LoginSession, AppError> {
    let normalized =
        NormalizedUsername::parse(&username).map_err(|_| AppError::InvalidCredentials)?;

    let Some(user) = repository
        .find_active_user_by_username(normalized.as_str())
        .await?
    else {
        return Err(AppError::InvalidCredentials);
    };

    verify_password(&password, &user.password_hash)?;

    let now = OffsetDateTime::now_utc();
    let expires_at = now + Duration::seconds(config.session_ttl_seconds as i64);
    let session_id = Uuid::new_v4();
    let token_id = Uuid::new_v4();
    let token_id_hash = hmac_hex(config.session_hash_key.as_bytes(), token_id.as_bytes())?;

    repository
        .create_session(CreateSessionInput {
            id: session_id,
            user_id: user.id,
            token_id_hash,
            expires_at,
        })
        .await?;

    let claims = Claims {
        sub: user.id.to_string(),
        sid: session_id.to_string(),
        jti: token_id.to_string(),
        iss: config.jwt_issuer.clone(),
        aud: config.jwt_audience.clone(),
        exp: expires_at.unix_timestamp(),
        nbf: now.unix_timestamp(),
        iat: now.unix_timestamp(),
    };

    let token = encode_claims(config, &claims)?;

    Ok(LoginSession {
        user: public_user(user),
        token,
        expires_at,
    })
}

pub async fn authenticate_token(
    repository: &AuthRepository,
    config: &AuthConfig,
    token: &str,
) -> Result<PublicUser, AppError> {
    let claims = decode_claims(config, token, true)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let session_id = Uuid::parse_str(&claims.sid).map_err(|_| AppError::Unauthorized)?;
    let token_id = Uuid::parse_str(&claims.jti).map_err(|_| AppError::Unauthorized)?;
    let token_id_hash = hmac_hex(config.session_hash_key.as_bytes(), token_id.as_bytes())?;

    repository
        .find_active_session(session_id, user_id, &token_id_hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let user = repository
        .find_active_user_by_id(user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(public_user(user))
}

pub async fn logout_user(
    repository: &AuthRepository,
    config: &AuthConfig,
    token: &str,
) -> Result<(), AppError> {
    let claims = decode_claims(config, token, false)?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
    let session_id = Uuid::parse_str(&claims.sid).map_err(|_| AppError::Unauthorized)?;

    repository.revoke_session(session_id, user_id).await
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(%error, "falha ao gerar hash de senha");
            AppError::Internal
        })
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|error| {
        tracing::error!(%error, "hash de senha armazenado é inválido");
        AppError::Internal
    })?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::InvalidCredentials)
}

fn hmac_hex(key: &[u8], value: &[u8]) -> Result<String, AppError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|error| {
        tracing::error!(%error, "chave hmac inválida");
        AppError::Internal
    })?;
    mac.update(value);

    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn encode_claims(config: &AuthConfig, claims: &Claims) -> Result<String, AppError> {
    let header = JwtHeader {
        alg: "HS256".to_owned(),
        typ: "JWT".to_owned(),
    };
    let header = encode_json_segment(&header)?;
    let payload = encode_json_segment(claims)?;
    let signing_input = format!("{header}.{payload}");
    let signature = hmac_bytes(config.jwt_secret.as_bytes(), signing_input.as_bytes())?;

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature)
    ))
}

fn decode_claims(config: &AuthConfig, token: &str, validate_exp: bool) -> Result<Claims, AppError> {
    let parts = token.split('.').collect::<Vec<_>>();
    let [header, payload, signature] = parts.as_slice() else {
        return Err(AppError::Unauthorized);
    };

    let decoded_header: JwtHeader = decode_json_segment(header)?;
    if decoded_header.alg != "HS256" || decoded_header.typ != "JWT" {
        return Err(AppError::Unauthorized);
    }

    let signing_input = format!("{header}.{payload}");
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| AppError::Unauthorized)?;
    verify_hmac(
        config.jwt_secret.as_bytes(),
        signing_input.as_bytes(),
        &signature,
    )?;

    let claims: Claims = decode_json_segment(payload)?;
    validate_claims(config, &claims, validate_exp)?;

    Ok(claims)
}

fn encode_json_segment<T: Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_vec(value)
        .map(|json| URL_SAFE_NO_PAD.encode(json))
        .map_err(|error| {
            tracing::error!(%error, "falha ao serializar jwt");
            AppError::Internal
        })
}

fn decode_json_segment<T: for<'de> Deserialize<'de>>(segment: &str) -> Result<T, AppError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| AppError::Unauthorized)?;

    serde_json::from_slice(&decoded).map_err(|_| AppError::Unauthorized)
}

fn hmac_bytes(key: &[u8], value: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|error| {
        tracing::error!(%error, "chave hmac inválida");
        AppError::Internal
    })?;
    mac.update(value);

    Ok(mac.finalize().into_bytes().to_vec())
}

fn verify_hmac(key: &[u8], value: &[u8], signature: &[u8]) -> Result<(), AppError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|error| {
        tracing::error!(%error, "chave hmac inválida");
        AppError::Internal
    })?;
    mac.update(value);

    mac.verify_slice(signature)
        .map_err(|_| AppError::Unauthorized)
}

fn validate_claims(
    config: &AuthConfig,
    claims: &Claims,
    validate_exp: bool,
) -> Result<(), AppError> {
    if claims.iss != config.jwt_issuer || claims.aud != config.jwt_audience {
        return Err(AppError::Unauthorized);
    }

    let now = OffsetDateTime::now_utc().unix_timestamp();
    if validate_exp && claims.exp <= now {
        return Err(AppError::Unauthorized);
    }
    if claims.nbf > now || claims.iat > now {
        return Err(AppError::Unauthorized);
    }

    Ok(())
}

fn public_user(user: UserRecord) -> PublicUser {
    PublicUser {
        id: user.id,
        username: user.username,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth_config() -> AuthConfig {
        AuthConfig {
            jwt_secret: "test-jwt-secret-with-at-least-32-characters".to_owned(),
            jwt_issuer: "carteira-de-investimentos-maicaosa".to_owned(),
            jwt_audience: "carteira-web".to_owned(),
            session_hash_key: "test-session-hash-key-with-at-least-32-chars".to_owned(),
            session_ttl_seconds: 3_600,
            cookie_secure: false,
            allowed_origins: vec!["http://127.0.0.1:3000".to_owned()],
            global_rate_limit_max_requests: 300,
            global_rate_limit_window_seconds: 60,
            global_rate_limit_block_seconds: 300,
            login_rate_limit_max_attempts: 5,
            login_rate_limit_window_seconds: 300,
            login_rate_limit_block_seconds: 900,
            register_rate_limit_max_requests: 3,
            register_rate_limit_window_seconds: 300,
            register_rate_limit_block_seconds: 900,
            mutation_rate_limit_max_requests: 60,
            mutation_rate_limit_window_seconds: 60,
            mutation_rate_limit_block_seconds: 300,
            expired_session_retention_days: 7,
        }
    }

    fn claims(config: &AuthConfig, expires_at: OffsetDateTime) -> Claims {
        let now = OffsetDateTime::now_utc();

        Claims {
            sub: Uuid::new_v4().to_string(),
            sid: Uuid::new_v4().to_string(),
            jti: Uuid::new_v4().to_string(),
            iss: config.jwt_issuer.clone(),
            aud: config.jwt_audience.clone(),
            exp: expires_at.unix_timestamp(),
            nbf: now.unix_timestamp(),
            iat: now.unix_timestamp(),
        }
    }

    #[test]
    fn rejects_tampered_jwt() {
        let config = auth_config();
        let token = encode_claims(
            &config,
            &claims(&config, OffsetDateTime::now_utc() + Duration::hours(1)),
        )
        .unwrap();
        let mut tampered = token;
        tampered.push('x');

        assert!(matches!(
            decode_claims(&config, &tampered, true),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn rejects_expired_jwt() {
        let config = auth_config();
        let token = encode_claims(
            &config,
            &claims(&config, OffsetDateTime::now_utc() - Duration::hours(1)),
        )
        .unwrap();

        assert!(matches!(
            decode_claims(&config, &token, true),
            Err(AppError::Unauthorized)
        ));
    }

    #[test]
    fn rejects_wrong_audience() {
        let config = auth_config();
        let mut wrong_claims = claims(&config, OffsetDateTime::now_utc() + Duration::hours(1));
        wrong_claims.aud = "outra-aplicacao".to_owned();
        let token = encode_claims(&config, &wrong_claims).unwrap();

        assert!(matches!(
            decode_claims(&config, &token, true),
            Err(AppError::Unauthorized)
        ));
    }
}
