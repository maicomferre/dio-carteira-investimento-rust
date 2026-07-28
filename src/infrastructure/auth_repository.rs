use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthRepository {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct CreateUserInput {
    pub id: Uuid,
    pub username: String,
    pub username_normalized: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_id_hash: String,
    pub expires_at: OffsetDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRecord {
    pub id: Uuid,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_user(&self, input: CreateUserInput) -> Result<UserRecord, AppError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            INSERT INTO users (id, username, username_normalized, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, password_hash
            "#,
        )
        .bind(input.id)
        .bind(input.username)
        .bind(input.username_normalized)
        .bind(input.password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn find_active_user_by_username(
        &self,
        username_normalized: &str,
    ) -> Result<Option<UserRecord>, AppError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, username, password_hash
            FROM users
            WHERE username_normalized = $1
              AND is_active = TRUE
            "#,
        )
        .bind(username_normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn find_active_user_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserRecord>, AppError> {
        sqlx::query_as::<_, UserRecord>(
            r#"
            SELECT id, username, password_hash
            FROM users
            WHERE id = $1
              AND is_active = TRUE
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn create_session(&self, input: CreateSessionInput) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO auth_sessions (id, user_id, token_id_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.token_id_hash)
        .bind(input.expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    pub async fn find_active_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        token_id_hash: &str,
    ) -> Result<Option<SessionRecord>, AppError> {
        sqlx::query_as::<_, SessionRecord>(
            r#"
            SELECT id
            FROM auth_sessions
            WHERE id = $1
              AND user_id = $2
              AND token_id_hash = $3
              AND revoked_at IS NULL
              AND expires_at > now()
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .bind(token_id_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn revoke_session(&self, session_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET revoked_at = now()
            WHERE id = $1
              AND user_id = $2
              AND revoked_at IS NULL
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_sqlx_error)
    }

    pub async fn delete_expired_sessions_before(
        &self,
        cutoff: OffsetDateTime,
    ) -> Result<u64, AppError> {
        sqlx::query(
            r#"
            DELETE FROM auth_sessions
            WHERE expires_at < $1
            "#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(map_sqlx_error)
    }
}

fn map_sqlx_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.is_unique_violation()
    {
        return AppError::Conflict("registro já existe");
    }

    tracing::error!(%error, "erro de banco de dados");
    AppError::Internal
}
