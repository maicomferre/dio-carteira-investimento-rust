use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::portfolio::{
        ASSET_NAME_MAX_CHARS, AssetCategory, AssetSymbol, BrokerName, Currency, Market,
        NonNegativeDecimal, PortfolioError, PortfolioSummary, PortfolioTransaction, Quantity,
        TransactionType, calculate_summary,
    },
    infrastructure::portfolio_repository::{
        AssetRecord, BrokerRecord, CreateAssetInput, CreateBrokerInput, CreateTransactionInput,
        PortfolioRepository, TransactionRecord, UpdateAssetInput, UpdateBrokerInput,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct PublicBroker {
    pub id: Uuid,
    pub name: String,
    pub is_archived: bool,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicAsset {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub market: String,
    pub category: String,
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub current_price: Decimal,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicTransaction {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub transaction_type: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub unit_price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees: Decimal,
    pub occurred_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AssetDraft {
    pub symbol: String,
    pub name: String,
    pub market: String,
    pub category: String,
    pub currency: String,
    pub current_price: Decimal,
}

#[derive(Debug, Clone)]
pub struct AssetPatch {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub market: Option<String>,
    pub category: Option<String>,
    pub currency: Option<String>,
    pub current_price: Option<Decimal>,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct BrokerPatch {
    pub name: String,
    pub version: i64,
}

#[derive(Debug, Clone)]
pub struct TransactionDraft {
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub transaction_type: TransactionType,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub fees: Decimal,
    pub occurred_at: OffsetDateTime,
    pub notes: Option<String>,
}

pub async fn create_broker(
    repository: &PortfolioRepository,
    user_id: Uuid,
    name: String,
) -> Result<PublicBroker, AppError> {
    let name = BrokerName::parse(&name).map_err(map_portfolio_error)?;

    repository
        .create_broker(CreateBrokerInput {
            id: Uuid::new_v4(),
            user_id,
            name: name.display().to_owned(),
            name_normalized: name.normalized().to_owned(),
        })
        .await
        .map(public_broker)
}

pub async fn list_brokers(
    repository: &PortfolioRepository,
    user_id: Uuid,
) -> Result<Vec<PublicBroker>, AppError> {
    repository
        .list_brokers(user_id)
        .await
        .map(|brokers| brokers.into_iter().map(public_broker).collect())
}

pub async fn update_broker(
    repository: &PortfolioRepository,
    user_id: Uuid,
    broker_id: Uuid,
    patch: BrokerPatch,
) -> Result<PublicBroker, AppError> {
    let name = BrokerName::parse(&patch.name).map_err(map_portfolio_error)?;

    repository
        .update_broker(UpdateBrokerInput {
            id: broker_id,
            user_id,
            name: name.display().to_owned(),
            name_normalized: name.normalized().to_owned(),
            version: patch.version,
        })
        .await
        .map(public_broker)
}

pub async fn archive_broker(
    repository: &PortfolioRepository,
    user_id: Uuid,
    broker_id: Uuid,
) -> Result<(), AppError> {
    repository.archive_broker(user_id, broker_id).await
}

pub async fn create_asset(
    repository: &PortfolioRepository,
    user_id: Uuid,
    draft: AssetDraft,
) -> Result<PublicAsset, AppError> {
    let symbol = AssetSymbol::parse(&draft.symbol).map_err(map_portfolio_error)?;
    let market = Market::parse(&draft.market).map_err(map_portfolio_error)?;
    let category = AssetCategory::parse(&draft.category).map_err(map_portfolio_error)?;
    let currency = Currency::parse(&draft.currency).map_err(map_portfolio_error)?;
    let current_price =
        NonNegativeDecimal::parse(draft.current_price).map_err(map_portfolio_error)?;
    let name = validate_asset_name(draft.name)?;

    repository
        .create_asset(CreateAssetInput {
            id: Uuid::new_v4(),
            user_id,
            symbol: symbol.as_str().to_owned(),
            symbol_normalized: symbol.as_str().to_owned(),
            name,
            market: market.as_str().to_owned(),
            category: category.as_str().to_owned(),
            currency: currency.as_str().to_owned(),
            current_price: current_price.value(),
        })
        .await
        .map(public_asset)
}

pub async fn list_assets(
    repository: &PortfolioRepository,
    user_id: Uuid,
) -> Result<Vec<PublicAsset>, AppError> {
    repository
        .list_assets(user_id)
        .await
        .map(|assets| assets.into_iter().map(public_asset).collect())
}

pub async fn update_asset(
    repository: &PortfolioRepository,
    user_id: Uuid,
    asset_id: Uuid,
    patch: AssetPatch,
) -> Result<PublicAsset, AppError> {
    let symbol = patch
        .symbol
        .as_deref()
        .map(AssetSymbol::parse)
        .transpose()
        .map_err(map_portfolio_error)?;
    let market = patch
        .market
        .as_deref()
        .map(Market::parse)
        .transpose()
        .map_err(map_portfolio_error)?;
    let category = patch
        .category
        .as_deref()
        .map(AssetCategory::parse)
        .transpose()
        .map_err(map_portfolio_error)?;
    let currency = patch
        .currency
        .as_deref()
        .map(Currency::parse)
        .transpose()
        .map_err(map_portfolio_error)?;
    let current_price = patch
        .current_price
        .map(NonNegativeDecimal::parse)
        .transpose()
        .map_err(map_portfolio_error)?;
    let name = patch.name.map(validate_asset_name).transpose()?;
    let symbol = symbol.map(|symbol| symbol.as_str().to_owned());

    repository
        .update_asset(UpdateAssetInput {
            id: asset_id,
            user_id,
            symbol: symbol.clone(),
            symbol_normalized: symbol,
            name,
            market: market.map(|market| market.as_str().to_owned()),
            category: category.map(|category| category.as_str().to_owned()),
            currency: currency.map(|currency| currency.as_str().to_owned()),
            current_price: current_price.map(|price| price.value()),
            version: patch.version,
        })
        .await
        .map(public_asset)
}

pub async fn record_transaction(
    repository: &PortfolioRepository,
    user_id: Uuid,
    draft: TransactionDraft,
) -> Result<PublicTransaction, AppError> {
    let quantity = Quantity::parse(draft.quantity).map_err(map_portfolio_error)?;
    let unit_price = NonNegativeDecimal::parse(draft.unit_price).map_err(map_portfolio_error)?;
    let fees = NonNegativeDecimal::parse(draft.fees).map_err(map_portfolio_error)?;
    let notes = validate_notes(draft.notes)?;
    let transaction_id = Uuid::new_v4();
    let transaction_type = draft.transaction_type;
    let candidate = CreateTransactionInput {
        id: transaction_id,
        user_id,
        asset_id: draft.asset_id,
        broker_id: draft.broker_id,
        transaction_type: transaction_type.as_str().to_owned(),
        quantity: quantity.value(),
        unit_price: unit_price.value(),
        fees: fees.value(),
        occurred_at: draft.occurred_at,
        notes,
    };

    repository
        .create_transaction_with_position_validation(candidate, |existing, asset| {
            if !matches!(transaction_type, TransactionType::Sell) {
                return Ok(());
            }

            let mut transactions = existing
                .into_iter()
                .map(portfolio_transaction)
                .collect::<Result<Vec<_>, _>>()?;
            transactions.push(PortfolioTransaction {
                id: transaction_id,
                asset_id: draft.asset_id,
                broker_id: draft.broker_id,
                currency: Currency::parse(&asset.currency).map_err(map_portfolio_error)?,
                category: AssetCategory::parse(&asset.category).map_err(map_portfolio_error)?,
                transaction_type: TransactionType::Sell,
                quantity,
                unit_price,
                fees,
                occurred_at: draft.occurred_at,
            });
            calculate_summary(&transactions)
                .map(|_| ())
                .map_err(map_portfolio_error)
        })
        .await
        .map(public_transaction)
}

pub async fn list_transactions(
    repository: &PortfolioRepository,
    user_id: Uuid,
) -> Result<Vec<PublicTransaction>, AppError> {
    repository
        .list_transactions(user_id)
        .await
        .map(|transactions| transactions.into_iter().map(public_transaction).collect())
}

pub async fn portfolio_summary(
    repository: &PortfolioRepository,
    user_id: Uuid,
) -> Result<PortfolioSummary, AppError> {
    let transactions = repository
        .list_transactions_with_asset_metadata(user_id)
        .await?
        .into_iter()
        .map(portfolio_transaction)
        .collect::<Result<Vec<_>, _>>()?;

    calculate_summary(&transactions).map_err(map_portfolio_error)
}

fn validate_asset_name(value: String) -> Result<String, AppError> {
    let name = value.trim().to_owned();

    if name.is_empty() || name.chars().count() > ASSET_NAME_MAX_CHARS {
        return Err(AppError::Validation("nome do ativo inválido"));
    }

    Ok(name)
}

fn validate_notes(value: Option<String>) -> Result<Option<String>, AppError> {
    value
        .map(|notes| {
            let trimmed = notes.trim().to_owned();

            if trimmed.is_empty() || trimmed.len() > 500 {
                Err(AppError::Validation("observação inválida"))
            } else {
                Ok(trimmed)
            }
        })
        .transpose()
}

fn portfolio_transaction(record: TransactionRecord) -> Result<PortfolioTransaction, AppError> {
    Ok(PortfolioTransaction {
        id: record.id,
        asset_id: record.asset_id,
        broker_id: record.broker_id,
        currency: Currency::parse(&record.currency).map_err(map_portfolio_error)?,
        category: AssetCategory::parse(&record.category).map_err(map_portfolio_error)?,
        transaction_type: TransactionType::parse(&record.transaction_type)
            .map_err(map_portfolio_error)?,
        quantity: Quantity::parse(record.quantity).map_err(map_portfolio_error)?,
        unit_price: NonNegativeDecimal::parse(record.unit_price).map_err(map_portfolio_error)?,
        fees: NonNegativeDecimal::parse(record.fees).map_err(map_portfolio_error)?,
        occurred_at: record.occurred_at,
    })
}

fn public_broker(record: BrokerRecord) -> PublicBroker {
    PublicBroker {
        id: record.id,
        name: record.name,
        is_archived: record.is_archived,
        version: record.version,
    }
}

fn public_asset(record: AssetRecord) -> PublicAsset {
    PublicAsset {
        id: record.id,
        symbol: record.symbol,
        name: record.name,
        market: record.market,
        category: record.category,
        currency: record.currency,
        current_price: record.current_price,
        version: record.version,
    }
}

fn public_transaction(record: TransactionRecord) -> PublicTransaction {
    PublicTransaction {
        id: record.id,
        asset_id: record.asset_id,
        broker_id: record.broker_id,
        transaction_type: record.transaction_type,
        quantity: record.quantity,
        unit_price: record.unit_price,
        fees: record.fees,
        occurred_at: record.occurred_at.to_string(),
        notes: record.notes,
    }
}

fn map_portfolio_error(error: PortfolioError) -> AppError {
    match error {
        PortfolioError::InsufficientPosition => AppError::Conflict("posição insuficiente"),
        PortfolioError::InvalidSymbol => AppError::Validation("símbolo inválido"),
        PortfolioError::InvalidCurrency => AppError::Validation("moeda inválida"),
        PortfolioError::InvalidMarket => AppError::Validation("mercado inválido"),
        PortfolioError::InvalidCategory => AppError::Validation("categoria inválida"),
        PortfolioError::InvalidDecimal => AppError::Validation("valor numérico inválido"),
        PortfolioError::InvalidName => AppError::Validation("nome inválido"),
    }
}

#[cfg(test)]
mod tests {
    use crate::{application::error::AppError, domain::portfolio::ASSET_NAME_MAX_CHARS};

    use super::validate_asset_name;

    #[test]
    fn validates_asset_name_limit_by_chars() {
        let valid = "Á".repeat(ASSET_NAME_MAX_CHARS);
        let invalid = "A".repeat(ASSET_NAME_MAX_CHARS + 1);

        assert!(
            matches!(validate_asset_name(valid), Ok(name) if name.chars().count() == ASSET_NAME_MAX_CHARS)
        );
        assert!(matches!(
            validate_asset_name(invalid),
            Err(AppError::Validation("nome do ativo inválido"))
        ));
    }
}
