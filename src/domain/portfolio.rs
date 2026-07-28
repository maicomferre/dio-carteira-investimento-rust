use std::collections::BTreeMap;

use rust_decimal::Decimal;
use thiserror::Error;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PortfolioError {
    #[error("símbolo inválido")]
    InvalidSymbol,
    #[error("moeda inválida")]
    InvalidCurrency,
    #[error("mercado inválido")]
    InvalidMarket,
    #[error("categoria inválida")]
    InvalidCategory,
    #[error("decimal inválido")]
    InvalidDecimal,
    #[error("venda excede posição disponível")]
    InsufficientPosition,
    #[error("nome inválido")]
    InvalidName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerName {
    display: String,
    normalized: String,
}

impl BrokerName {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        let display = value.trim().to_owned();
        let normalized = display.to_ascii_lowercase();

        if display.is_empty() || display.len() > 120 {
            return Err(PortfolioError::InvalidName);
        }

        Ok(Self {
            display,
            normalized,
        })
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn normalized(&self) -> &str {
        &self.normalized
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetSymbol(String);

impl AssetSymbol {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        let normalized = value.trim().to_uppercase();

        if normalized.is_empty()
            || normalized.len() > 32
            || normalized
                .chars()
                .any(|char| !(char.is_ascii_alphanumeric() || matches!(char, '.' | '-' | '_')))
        {
            return Err(PortfolioError::InvalidSymbol);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Currency {
    Brl,
    Usd,
}

impl Currency {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        match value.trim().to_uppercase().as_str() {
            "BRL" => Ok(Self::Brl),
            "USD" => Ok(Self::Usd),
            _ => Err(PortfolioError::InvalidCurrency),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brl => "BRL",
            Self::Usd => "USD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Market {
    B3,
    Nasdaq,
    Nyse,
    Crypto,
    Other,
}

impl Market {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        match value.trim().to_uppercase().as_str() {
            "B3" => Ok(Self::B3),
            "NASDAQ" => Ok(Self::Nasdaq),
            "NYSE" => Ok(Self::Nyse),
            "CRYPTO" => Ok(Self::Crypto),
            "OTHER" => Ok(Self::Other),
            _ => Err(PortfolioError::InvalidMarket),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::B3 => "B3",
            Self::Nasdaq => "NASDAQ",
            Self::Nyse => "NYSE",
            Self::Crypto => "CRYPTO",
            Self::Other => "OTHER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetCategory {
    Stock,
    Fii,
    Etf,
    Bdr,
    Crypto,
    Other,
}

impl AssetCategory {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        match value.trim().to_uppercase().as_str() {
            "STOCK" => Ok(Self::Stock),
            "FII" => Ok(Self::Fii),
            "ETF" => Ok(Self::Etf),
            "BDR" => Ok(Self::Bdr),
            "CRYPTO" => Ok(Self::Crypto),
            "OTHER" => Ok(Self::Other),
            _ => Err(PortfolioError::InvalidCategory),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stock => "STOCK",
            Self::Fii => "FII",
            Self::Etf => "ETF",
            Self::Bdr => "BDR",
            Self::Crypto => "CRYPTO",
            Self::Other => "OTHER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity(Decimal);

impl Quantity {
    pub fn parse(value: Decimal) -> Result<Self, PortfolioError> {
        if value <= Decimal::ZERO {
            return Err(PortfolioError::InvalidDecimal);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> Decimal {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonNegativeDecimal(Decimal);

impl NonNegativeDecimal {
    pub fn parse(value: Decimal) -> Result<Self, PortfolioError> {
        if value < Decimal::ZERO {
            return Err(PortfolioError::InvalidDecimal);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> Decimal {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    Buy,
    Sell,
}

impl TransactionType {
    pub fn parse(value: &str) -> Result<Self, PortfolioError> {
        match value.trim().to_uppercase().as_str() {
            "BUY" => Ok(Self::Buy),
            "SELL" => Ok(Self::Sell),
            _ => Err(PortfolioError::InvalidCategory),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortfolioTransaction {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub currency: Currency,
    pub category: AssetCategory,
    pub transaction_type: TransactionType,
    pub quantity: Quantity,
    pub unit_price: NonNegativeDecimal,
    pub fees: NonNegativeDecimal,
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub asset_id: Uuid,
    pub broker_id: Uuid,
    pub currency: Currency,
    pub category: AssetCategory,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
}

impl Position {
    pub fn average_cost(&self) -> Decimal {
        if self.quantity <= Decimal::ZERO {
            Decimal::ZERO
        } else {
            self.cost_basis / self.quantity
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyCashFlow {
    pub purchases: Decimal,
    pub sales: Decimal,
    pub fees: Decimal,
    pub net_flow: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioSummary {
    pub positions: Vec<Position>,
    pub totals_by_currency: BTreeMap<Currency, Decimal>,
    pub allocation_by_category: BTreeMap<(Currency, AssetCategory), Decimal>,
    pub allocation_by_broker: BTreeMap<(Currency, Uuid), Decimal>,
    pub daily_cash_flow: BTreeMap<Date, DailyCashFlow>,
}

pub fn calculate_summary(
    transactions: &[PortfolioTransaction],
) -> Result<PortfolioSummary, PortfolioError> {
    let mut ordered_transactions = transactions.to_vec();
    ordered_transactions.sort_by_key(|transaction| (transaction.occurred_at, transaction.id));

    let mut positions = BTreeMap::<(Uuid, Uuid), Position>::new();
    let mut daily_cash_flow = BTreeMap::<Date, DailyCashFlow>::new();

    for transaction in ordered_transactions {
        let quantity = transaction.quantity.value();
        let unit_price = transaction.unit_price.value();
        let fees = transaction.fees.value();
        let gross_amount = quantity * unit_price;
        let day = transaction.occurred_at.date();
        let daily = daily_cash_flow.entry(day).or_insert(DailyCashFlow {
            purchases: Decimal::ZERO,
            sales: Decimal::ZERO,
            fees: Decimal::ZERO,
            net_flow: Decimal::ZERO,
        });

        let position = positions
            .entry((transaction.asset_id, transaction.broker_id))
            .or_insert(Position {
                asset_id: transaction.asset_id,
                broker_id: transaction.broker_id,
                currency: transaction.currency,
                category: transaction.category,
                quantity: Decimal::ZERO,
                cost_basis: Decimal::ZERO,
            });

        match transaction.transaction_type {
            TransactionType::Buy => {
                position.quantity += quantity;
                position.cost_basis += gross_amount + fees;
                daily.purchases += gross_amount;
                daily.fees += fees;
                daily.net_flow -= gross_amount + fees;
            }
            TransactionType::Sell => {
                if position.quantity < quantity {
                    return Err(PortfolioError::InsufficientPosition);
                }

                let average_cost = position.average_cost();
                position.quantity -= quantity;
                position.cost_basis -= average_cost * quantity;
                daily.sales += gross_amount;
                daily.fees += fees;
                daily.net_flow += gross_amount - fees;
            }
        }
    }

    let positions = positions
        .into_values()
        .filter(|position| position.quantity > Decimal::ZERO)
        .collect::<Vec<_>>();
    let mut totals_by_currency = BTreeMap::<Currency, Decimal>::new();
    let mut allocation_by_category = BTreeMap::<(Currency, AssetCategory), Decimal>::new();
    let mut allocation_by_broker = BTreeMap::<(Currency, Uuid), Decimal>::new();

    for position in &positions {
        *totals_by_currency.entry(position.currency).or_default() += position.cost_basis;
        *allocation_by_category
            .entry((position.currency, position.category))
            .or_default() += position.cost_basis;
        *allocation_by_broker
            .entry((position.currency, position.broker_id))
            .or_default() += position.cost_basis;
    }

    Ok(PortfolioSummary {
        positions,
        totals_by_currency,
        allocation_by_category,
        allocation_by_broker,
        daily_cash_flow,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::datetime;

    struct TransactionInput {
        id: Uuid,
        asset_id: Uuid,
        broker_id: Uuid,
        transaction_type: TransactionType,
        quantity: Decimal,
        unit_price: Decimal,
        fees: Decimal,
        occurred_at: OffsetDateTime,
    }

    fn transaction(input: TransactionInput) -> PortfolioTransaction {
        PortfolioTransaction {
            id: input.id,
            asset_id: input.asset_id,
            broker_id: input.broker_id,
            currency: Currency::Brl,
            category: AssetCategory::Stock,
            transaction_type: input.transaction_type,
            quantity: Quantity::parse(input.quantity).unwrap(),
            unit_price: NonNegativeDecimal::parse(input.unit_price).unwrap(),
            fees: NonNegativeDecimal::parse(input.fees).unwrap(),
            occurred_at: input.occurred_at,
        }
    }

    #[test]
    fn normalizes_asset_symbol() {
        let symbol = AssetSymbol::parse(" petr4 ").unwrap();

        assert_eq!(symbol.as_str(), "PETR4");
    }

    #[test]
    fn rejects_negative_money_and_zero_quantity() {
        assert_eq!(
            NonNegativeDecimal::parse(dec!(-0.01)),
            Err(PortfolioError::InvalidDecimal)
        );
        assert_eq!(
            Quantity::parse(Decimal::ZERO),
            Err(PortfolioError::InvalidDecimal)
        );
    }

    #[test]
    fn normalizes_broker_name() {
        let name = BrokerName::parse(" Nubank ").unwrap();

        assert_eq!(name.display(), "Nubank");
        assert_eq!(name.normalized(), "nubank");
    }

    #[test]
    fn calculates_weighted_average_after_partial_sale() {
        let asset_id = Uuid::new_v4();
        let broker_id = Uuid::new_v4();
        let transactions = vec![
            transaction(TransactionInput {
                id: Uuid::from_u128(1),
                asset_id,
                broker_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(100),
                unit_price: dec!(10),
                fees: dec!(2),
                occurred_at: datetime!(2026-07-01 10:00 UTC),
            }),
            transaction(TransactionInput {
                id: Uuid::from_u128(2),
                asset_id,
                broker_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(50),
                unit_price: dec!(20),
                fees: dec!(3),
                occurred_at: datetime!(2026-07-02 10:00 UTC),
            }),
            transaction(TransactionInput {
                id: Uuid::from_u128(3),
                asset_id,
                broker_id,
                transaction_type: TransactionType::Sell,
                quantity: dec!(40),
                unit_price: dec!(30),
                fees: dec!(1),
                occurred_at: datetime!(2026-07-03 10:00 UTC),
            }),
        ];

        let summary = calculate_summary(&transactions).unwrap();
        let position = summary.positions.first().unwrap();

        assert_eq!(position.quantity, dec!(110));
        assert_eq!(position.cost_basis, dec!(1470.3333333333333333333333333));
        assert_eq!(
            position.average_cost(),
            dec!(13.366666666666666666666666666)
        );
        assert_eq!(
            summary.totals_by_currency.get(&Currency::Brl),
            Some(&position.cost_basis)
        );
    }

    #[test]
    fn rejects_sale_above_position_for_same_broker() {
        let asset_id = Uuid::new_v4();
        let nubank_id = Uuid::new_v4();
        let xp_id = Uuid::new_v4();
        let transactions = vec![
            transaction(TransactionInput {
                id: Uuid::from_u128(1),
                asset_id,
                broker_id: nubank_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(100),
                unit_price: dec!(10),
                fees: Decimal::ZERO,
                occurred_at: datetime!(2026-07-01 10:00 UTC),
            }),
            transaction(TransactionInput {
                id: Uuid::from_u128(2),
                asset_id,
                broker_id: xp_id,
                transaction_type: TransactionType::Sell,
                quantity: dec!(1),
                unit_price: dec!(10),
                fees: Decimal::ZERO,
                occurred_at: datetime!(2026-07-02 10:00 UTC),
            }),
        ];

        assert_eq!(
            calculate_summary(&transactions),
            Err(PortfolioError::InsufficientPosition)
        );
    }

    #[test]
    fn separates_same_asset_by_broker_and_groups_allocations() {
        let asset_id = Uuid::new_v4();
        let nubank_id = Uuid::new_v4();
        let xp_id = Uuid::new_v4();
        let transactions = vec![
            transaction(TransactionInput {
                id: Uuid::from_u128(1),
                asset_id,
                broker_id: nubank_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(100),
                unit_price: dec!(10),
                fees: Decimal::ZERO,
                occurred_at: datetime!(2026-07-01 10:00 UTC),
            }),
            transaction(TransactionInput {
                id: Uuid::from_u128(2),
                asset_id,
                broker_id: xp_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(200),
                unit_price: dec!(20),
                fees: Decimal::ZERO,
                occurred_at: datetime!(2026-07-01 11:00 UTC),
            }),
        ];

        let summary = calculate_summary(&transactions).unwrap();

        assert_eq!(summary.positions.len(), 2);
        assert_eq!(
            summary
                .allocation_by_broker
                .get(&(Currency::Brl, nubank_id)),
            Some(&dec!(1000))
        );
        assert_eq!(
            summary.allocation_by_broker.get(&(Currency::Brl, xp_id)),
            Some(&dec!(4000))
        );
        assert_eq!(
            summary
                .allocation_by_category
                .get(&(Currency::Brl, AssetCategory::Stock)),
            Some(&dec!(5000))
        );
    }

    #[test]
    fn calculates_daily_cash_flow_without_calling_it_profit() {
        let asset_id = Uuid::new_v4();
        let broker_id = Uuid::new_v4();
        let transactions = vec![
            transaction(TransactionInput {
                id: Uuid::from_u128(1),
                asset_id,
                broker_id,
                transaction_type: TransactionType::Buy,
                quantity: dec!(10),
                unit_price: dec!(20),
                fees: dec!(2),
                occurred_at: datetime!(2026-07-01 10:00 UTC),
            }),
            transaction(TransactionInput {
                id: Uuid::from_u128(2),
                asset_id,
                broker_id,
                transaction_type: TransactionType::Sell,
                quantity: dec!(4),
                unit_price: dec!(30),
                fees: dec!(1),
                occurred_at: datetime!(2026-07-01 11:00 UTC),
            }),
        ];

        let summary = calculate_summary(&transactions).unwrap();
        let cash_flow = summary
            .daily_cash_flow
            .get(&datetime!(2026-07-01 00:00 UTC).date())
            .unwrap();

        assert_eq!(cash_flow.purchases, dec!(200));
        assert_eq!(cash_flow.sales, dec!(120));
        assert_eq!(cash_flow.fees, dec!(3));
        assert_eq!(cash_flow.net_flow, dec!(-83));
    }
}
