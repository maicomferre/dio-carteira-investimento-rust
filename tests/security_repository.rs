use carteira_de_investimentos_maicaosa::{
    application::{
        error::AppError,
        portfolio::{self, AssetDraft, AssetPatch, TransactionDraft},
    },
    domain::portfolio::TransactionType,
    infrastructure::portfolio_repository::{
        CreateAssetInput, CreateTransactionInput, PortfolioRepository,
    },
};
use rust_decimal_macros::dec;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::OffsetDateTime;
use uuid::Uuid;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

async fn pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://carteira_runtime:carteira_runtime_dev_password@127.0.0.1:5433/carteira_dev"
            .to_owned()
    });
    let migration_database_url = std::env::var("DATABASE_MIGRATION_URL").unwrap_or_else(|_| {
        "postgres://carteira_migrator:carteira_migrator_dev_password@127.0.0.1:5433/carteira_dev"
            .to_owned()
    });
    let migration_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&migration_database_url)
        .await
        .expect("migration database connection for integration tests");

    MIGRATOR
        .run(&migration_pool)
        .await
        .expect("database migrations for integration tests");

    migration_pool.close().await;

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("runtime database connection for integration tests")
}

async fn create_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let username = format!("user-{}", id.simple());

    sqlx::query(
        r#"
        INSERT INTO users (id, username, username_normalized, password_hash)
        VALUES ($1, $2, $2, 'integration-test-hash')
        "#,
    )
    .bind(id)
    .bind(username)
    .execute(pool)
    .await
    .expect("insert integration test user");

    id
}

fn assert_validation(error: AppError) {
    assert!(matches!(error, AppError::Validation(_)), "{error:?}");
}

#[tokio::test]
async fn runtime_database_user_cannot_create_schema_objects() {
    let pool = pool().await;
    let table_name = format!("runtime_privilege_probe_{}", Uuid::new_v4().simple());
    let create_sql = format!("CREATE TABLE public.{table_name} (id INTEGER)");

    // Audited: `table_name` is generated internally from a UUID and never
    // contains user input. The dynamic DDL exists only to prove least privilege.
    if sqlx::query(sqlx::AssertSqlSafe(create_sql))
        .execute(&pool)
        .await
        .is_ok()
    {
        let drop_sql = format!("DROP TABLE public.{table_name}");
        let _ = sqlx::query(sqlx::AssertSqlSafe(drop_sql))
            .execute(&pool)
            .await;
        panic!("runtime database user must not have DDL privileges");
    }
}

#[tokio::test]
async fn asset_listing_is_scoped_by_authenticated_user() {
    let pool = pool().await;
    let repository = PortfolioRepository::new(pool.clone());
    let first_user = create_user(&pool).await;
    let second_user = create_user(&pool).await;

    portfolio::create_asset(
        &repository,
        first_user,
        AssetDraft {
            symbol: "PETR4".to_owned(),
            name: "Petrobras PN".to_owned(),
            market: "B3".to_owned(),
            category: "STOCK".to_owned(),
            currency: "BRL".to_owned(),
            current_price: dec!(38.42),
        },
    )
    .await
    .expect("create first user asset");
    portfolio::create_asset(
        &repository,
        second_user,
        AssetDraft {
            symbol: "AAPL".to_owned(),
            name: "Apple Inc.".to_owned(),
            market: "NASDAQ".to_owned(),
            category: "STOCK".to_owned(),
            currency: "USD".to_owned(),
            current_price: dec!(214.00),
        },
    )
    .await
    .expect("create second user asset");

    let first_assets = portfolio::list_assets(&repository, first_user)
        .await
        .expect("list first user assets");

    assert_eq!(first_assets.len(), 1);
    assert_eq!(first_assets[0].symbol, "PETR4");
}

#[tokio::test]
async fn asset_update_rejects_idor_against_another_user_asset() {
    let pool = pool().await;
    let repository = PortfolioRepository::new(pool.clone());
    let owner = create_user(&pool).await;
    let attacker = create_user(&pool).await;
    let asset = portfolio::create_asset(
        &repository,
        owner,
        AssetDraft {
            symbol: "CMIG4".to_owned(),
            name: "Cemig PN".to_owned(),
            market: "B3".to_owned(),
            category: "STOCK".to_owned(),
            currency: "BRL".to_owned(),
            current_price: dec!(11.30),
        },
    )
    .await
    .expect("create owner asset");

    let error = portfolio::update_asset(
        &repository,
        attacker,
        asset.id,
        AssetPatch {
            symbol: None,
            name: Some("Ativo invadido".to_owned()),
            market: None,
            category: None,
            currency: None,
            current_price: None,
            version: asset.version,
        },
    )
    .await
    .expect_err("attacker must not update another user's asset");

    assert!(matches!(error, AppError::Conflict(_)), "{error:?}");
    let unchanged = repository
        .get_asset(owner, asset.id)
        .await
        .expect("load owner asset")
        .expect("owner asset still exists");
    assert_eq!(unchanged.name, "Cemig PN");
}

#[tokio::test]
async fn repository_uses_parameters_for_sql_injection_like_asset_values() {
    let pool = pool().await;
    let repository = PortfolioRepository::new(pool.clone());
    let user_id = create_user(&pool).await;
    let injected_symbol = "PETR4');--";

    let record = repository
        .create_asset(CreateAssetInput {
            id: Uuid::new_v4(),
            user_id,
            symbol: injected_symbol.to_owned(),
            symbol_normalized: injected_symbol.to_owned(),
            name: "Valor com aspas".to_owned(),
            market: "B3".to_owned(),
            category: "STOCK".to_owned(),
            currency: "BRL".to_owned(),
            current_price: dec!(38.42),
        })
        .await
        .expect("parameterized insert must store the literal value");

    assert_eq!(record.symbol, injected_symbol);
    let assets_table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('public.assets') IS NOT NULL")
            .fetch_one(&pool)
            .await
            .expect("check assets table");
    assert!(assets_table_exists);
}

#[tokio::test]
async fn transaction_rejects_broker_owned_by_another_user() {
    let pool = pool().await;
    let repository = PortfolioRepository::new(pool.clone());
    let asset_owner = create_user(&pool).await;
    let broker_owner = create_user(&pool).await;
    let asset = portfolio::create_asset(
        &repository,
        asset_owner,
        AssetDraft {
            symbol: "ASAI3".to_owned(),
            name: "Assai ON".to_owned(),
            market: "B3".to_owned(),
            category: "STOCK".to_owned(),
            currency: "BRL".to_owned(),
            current_price: dec!(9.20),
        },
    )
    .await
    .expect("create asset");
    let broker = portfolio::create_broker(&repository, broker_owner, "XP Investimentos".to_owned())
        .await
        .expect("create broker for another user");

    let error = portfolio::record_transaction(
        &repository,
        asset_owner,
        TransactionDraft {
            asset_id: asset.id,
            broker_id: broker.id,
            transaction_type: TransactionType::Buy,
            quantity: dec!(10),
            unit_price: dec!(9.20),
            fees: dec!(0),
            occurred_at: OffsetDateTime::now_utc(),
            notes: None,
        },
    )
    .await
    .expect_err("transaction must reject another user's broker");

    assert_validation(error);
    assert!(
        portfolio::list_transactions(&repository, asset_owner)
            .await
            .expect("list asset owner transactions")
            .is_empty()
    );
}

#[tokio::test]
async fn direct_transaction_insert_still_enforces_same_user_ownership() {
    let pool = pool().await;
    let repository = PortfolioRepository::new(pool.clone());
    let first_user = create_user(&pool).await;
    let second_user = create_user(&pool).await;
    let asset = portfolio::create_asset(
        &repository,
        first_user,
        AssetDraft {
            symbol: "KLBN4".to_owned(),
            name: "Klabin PN".to_owned(),
            market: "B3".to_owned(),
            category: "STOCK".to_owned(),
            currency: "BRL".to_owned(),
            current_price: dec!(4.15),
        },
    )
    .await
    .expect("create asset");
    let broker = portfolio::create_broker(&repository, second_user, "Nubank".to_owned())
        .await
        .expect("create broker for another user");

    let error = repository
        .create_transaction(CreateTransactionInput {
            id: Uuid::new_v4(),
            user_id: first_user,
            asset_id: asset.id,
            broker_id: broker.id,
            transaction_type: "BUY".to_owned(),
            quantity: dec!(1),
            unit_price: dec!(4.15),
            fees: dec!(0),
            occurred_at: OffsetDateTime::now_utc(),
            notes: None,
        })
        .await
        .expect_err("repository must reject mixed ownership");

    assert_validation(error);
}
