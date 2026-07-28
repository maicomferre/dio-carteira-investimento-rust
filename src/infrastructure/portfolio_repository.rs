use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::error::AppError;

#[derive(Debug, Clone)]
pub struct PortfolioRepository {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct CreateBrokerInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub name_normalized: String,
}

#[derive(Debug, Clone)]
pub struct UpdateBrokerInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub name_normalized: String,
    pub version: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BrokerRecord {
    pub id: Uuid,
    pub name: String,
    pub is_archived: bool,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct CreateAssetInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub symbol_normalized: String,
    pub name: String,
    pub market: String,
    pub category: String,
    pub currency: String,
    pub current_price: Decimal,
}

#[derive(Debug, Clone)]
pub struct UpdateAssetInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub symbol: Option<String>,
    pub symbol_normalized: Option<String>,
    pub name: Option<String>,
    pub market: Option<String>,
    pub category: Option<String>,
    pub currency: Option<String>,
    pub current_price: Option<Decimal>,
    pub version: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AssetRecord {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub market: String,
    pub category: String,
    pub currency: String,
    pub current_price: Decimal,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct CreateTransactionInput {
    pub id: Uuid,
    pub user_id: Uuid,
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub transaction_type: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub fees: Decimal,
    pub occurred_at: OffsetDateTime,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TransactionRecord {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub transaction_type: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub fees: Decimal,
    pub occurred_at: OffsetDateTime,
    pub notes: Option<String>,
    pub currency: String,
    pub category: String,
}

impl PortfolioRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_broker(&self, input: CreateBrokerInput) -> Result<BrokerRecord, AppError> {
        sqlx::query_as::<_, BrokerRecord>(
            r#"
            INSERT INTO brokers (id, user_id, name, name_normalized)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, is_archived, version
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.name)
        .bind(input.name_normalized)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn list_brokers(&self, user_id: Uuid) -> Result<Vec<BrokerRecord>, AppError> {
        sqlx::query_as::<_, BrokerRecord>(
            r#"
            SELECT id, name, is_archived, version
            FROM brokers
            WHERE user_id = $1
            ORDER BY is_archived ASC, name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn update_broker(&self, input: UpdateBrokerInput) -> Result<BrokerRecord, AppError> {
        sqlx::query_as::<_, BrokerRecord>(
            r#"
            UPDATE brokers
            SET name = $3,
                name_normalized = $4,
                version = version + 1,
                updated_at = now()
            WHERE id = $1
              AND user_id = $2
              AND version = $5
              AND is_archived = FALSE
            RETURNING id, name, is_archived, version
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.name)
        .bind(input.name_normalized)
        .bind(input.version)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(AppError::Conflict(
            "corretora não encontrada ou versão desatualizada",
        ))
    }

    pub async fn archive_broker(&self, user_id: Uuid, broker_id: Uuid) -> Result<(), AppError> {
        let has_position: Option<i32> = sqlx::query_scalar(
            r#"
            SELECT 1
            FROM transactions
            WHERE user_id = $1
              AND broker_id = $2
            GROUP BY asset_id, broker_id
            HAVING SUM(
                CASE
                    WHEN transaction_type = 'BUY' THEN quantity
                    ELSE -quantity
                END
            ) > 0
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(broker_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if has_position.is_some() {
            return Err(AppError::Conflict("corretora possui posição aberta"));
        }

        let rows = sqlx::query(
            r#"
            UPDATE brokers
            SET is_archived = TRUE,
                archived_at = now(),
                updated_at = now(),
                version = version + 1
            WHERE id = $1
              AND user_id = $2
              AND is_archived = FALSE
            "#,
        )
        .bind(broker_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .rows_affected();

        (rows == 1)
            .then_some(())
            .ok_or(AppError::Conflict("corretora não encontrada"))
    }

    pub async fn create_asset(&self, input: CreateAssetInput) -> Result<AssetRecord, AppError> {
        sqlx::query_as::<_, AssetRecord>(
            r#"
            INSERT INTO assets (
                id, user_id, symbol, symbol_normalized, name,
                market, category, currency, current_price
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, symbol, name, market, category, currency, current_price, version
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.symbol)
        .bind(input.symbol_normalized)
        .bind(input.name)
        .bind(input.market)
        .bind(input.category)
        .bind(input.currency)
        .bind(input.current_price)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn list_assets(&self, user_id: Uuid) -> Result<Vec<AssetRecord>, AppError> {
        sqlx::query_as::<_, AssetRecord>(
            r#"
            SELECT id, symbol, name, market, category, currency, current_price, version
            FROM assets
            WHERE user_id = $1
            ORDER BY symbol ASC, market ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn get_asset(
        &self,
        user_id: Uuid,
        asset_id: Uuid,
    ) -> Result<Option<AssetRecord>, AppError> {
        sqlx::query_as::<_, AssetRecord>(
            r#"
            SELECT id, symbol, name, market, category, currency, current_price, version
            FROM assets
            WHERE id = $1
              AND user_id = $2
            "#,
        )
        .bind(asset_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn update_asset(&self, input: UpdateAssetInput) -> Result<AssetRecord, AppError> {
        sqlx::query_as::<_, AssetRecord>(
            r#"
            UPDATE assets
            SET symbol = COALESCE($3, symbol),
                symbol_normalized = COALESCE($4, symbol_normalized),
                name = COALESCE($5, name),
                market = COALESCE($6, market),
                category = COALESCE($7, category),
                currency = COALESCE($8, currency),
                current_price = COALESCE($9, current_price),
                version = version + 1,
                updated_at = now()
            WHERE id = $1
              AND user_id = $2
              AND version = $10
            RETURNING id, symbol, name, market, category, currency, current_price, version
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.symbol)
        .bind(input.symbol_normalized)
        .bind(input.name)
        .bind(input.market)
        .bind(input.category)
        .bind(input.currency)
        .bind(input.current_price)
        .bind(input.version)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(AppError::Conflict(
            "ativo não encontrado ou versão desatualizada",
        ))
    }

    pub async fn create_transaction(
        &self,
        input: CreateTransactionInput,
    ) -> Result<TransactionRecord, AppError> {
        self.insert_transaction(&self.pool, input).await
    }

    pub async fn create_transaction_with_position_validation<F>(
        &self,
        input: CreateTransactionInput,
        validate: F,
    ) -> Result<TransactionRecord, AppError>
    where
        F: FnOnce(Vec<TransactionRecord>, AssetRecord) -> Result<(), AppError>,
    {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;

        sqlx::query(
            r#"
            SELECT pg_advisory_xact_lock(
                hashtextextended($1::text || ':' || $2::text || ':' || $3::text, 0)
            )
            "#,
        )
        .bind(input.user_id)
        .bind(input.asset_id)
        .bind(input.broker_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let asset = sqlx::query_as::<_, AssetRecord>(
            r#"
            SELECT id, symbol, name, market, category, currency, current_price, version
            FROM assets
            WHERE id = $1
              AND user_id = $2
            "#,
        )
        .bind(input.asset_id)
        .bind(input.user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(AppError::Validation("ativo inválido"))?;

        let broker_exists: Option<i32> = sqlx::query_scalar(
            r#"
            SELECT 1
            FROM brokers
            WHERE id = $1
              AND user_id = $2
              AND is_archived = FALSE
            "#,
        )
        .bind(input.broker_id)
        .bind(input.user_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        if broker_exists.is_none() {
            return Err(AppError::Validation("corretora inválida"));
        }

        let existing = Self::list_transactions_query_on_executor(
            &mut *transaction,
            input.user_id,
            Some(input.asset_id),
            Some(input.broker_id),
        )
        .await?;

        validate(existing, asset)?;

        let record = self.insert_transaction(&mut *transaction, input).await?;

        transaction.commit().await.map_err(map_sqlx_error)?;

        Ok(record)
    }

    async fn insert_transaction<'e, E>(
        &self,
        executor: E,
        input: CreateTransactionInput,
    ) -> Result<TransactionRecord, AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query_as::<_, TransactionRecord>(
            r#"
            INSERT INTO transactions (
                id, user_id, asset_id, broker_id, transaction_type,
                quantity, unit_price, fees, occurred_at, notes
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
            WHERE EXISTS (
                SELECT 1
                FROM assets
                WHERE id = $3
                  AND user_id = $2
            )
              AND EXISTS (
                SELECT 1
                FROM brokers
                WHERE id = $4
                  AND user_id = $2
                  AND is_archived = FALSE
            )
            RETURNING
                transactions.id,
                transactions.asset_id,
                transactions.broker_id,
                transactions.transaction_type,
                transactions.quantity,
                transactions.unit_price,
                transactions.fees,
                transactions.occurred_at,
                transactions.notes,
                (SELECT currency FROM assets WHERE assets.id = transactions.asset_id) AS currency,
                (SELECT category FROM assets WHERE assets.id = transactions.asset_id) AS category
            "#,
        )
        .bind(input.id)
        .bind(input.user_id)
        .bind(input.asset_id)
        .bind(input.broker_id)
        .bind(input.transaction_type)
        .bind(input.quantity)
        .bind(input.unit_price)
        .bind(input.fees)
        .bind(input.occurred_at)
        .bind(input.notes)
        .fetch_one(executor)
        .await
        .map_err(map_sqlx_error)
    }

    pub async fn list_transactions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        self.list_transactions_query(user_id, None, None).await
    }

    pub async fn list_transactions_for_position(
        &self,
        user_id: Uuid,
        asset_id: Uuid,
        broker_id: Uuid,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        self.list_transactions_query(user_id, Some(asset_id), Some(broker_id))
            .await
    }

    pub async fn list_transactions_with_asset_metadata(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        self.list_transactions(user_id).await
    }

    async fn list_transactions_query(
        &self,
        user_id: Uuid,
        asset_id: Option<Uuid>,
        broker_id: Option<Uuid>,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        Self::list_transactions_query_on_executor(&self.pool, user_id, asset_id, broker_id).await
    }

    async fn list_transactions_query_on_executor<'e, E>(
        executor: E,
        user_id: Uuid,
        asset_id: Option<Uuid>,
        broker_id: Option<Uuid>,
    ) -> Result<Vec<TransactionRecord>, AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query_as::<_, TransactionRecord>(
            r#"
            SELECT
                transactions.id,
                transactions.asset_id,
                transactions.broker_id,
                transactions.transaction_type,
                transactions.quantity,
                transactions.unit_price,
                transactions.fees,
                transactions.occurred_at,
                transactions.notes,
                assets.currency,
                assets.category
            FROM transactions
            INNER JOIN assets ON assets.id = transactions.asset_id
            WHERE transactions.user_id = $1
              AND ($2::uuid IS NULL OR transactions.asset_id = $2)
              AND ($3::uuid IS NULL OR transactions.broker_id = $3)
            ORDER BY transactions.occurred_at DESC, transactions.id DESC
            "#,
        )
        .bind(user_id)
        .bind(asset_id)
        .bind(broker_id)
        .fetch_all(executor)
        .await
        .map_err(map_sqlx_error)
    }
}

fn map_sqlx_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.is_unique_violation()
    {
        return AppError::Conflict("registro já existe");
    }

    if matches!(error, sqlx::Error::RowNotFound) {
        return AppError::Validation("recurso não encontrado");
    }

    tracing::error!(%error, "erro de banco de dados");
    AppError::Internal
}
