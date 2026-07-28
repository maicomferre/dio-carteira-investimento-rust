use rust_decimal::Decimal;
use serde::Serialize;
use time::OffsetDateTime;

use crate::domain::portfolio::{AssetCategory, AssetSymbol, Currency, Market};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentSearchQuery(String);

impl InstrumentSearchQuery {
    pub fn parse(value: &str) -> Result<Self, InstrumentError> {
        let normalized = value.trim().to_ascii_uppercase();
        if normalized.len() < 2 {
            return Err(InstrumentError::InvalidQuery);
        }
        if normalized.len() > 32 {
            return Err(InstrumentError::InvalidQuery);
        }
        if !normalized
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '-' | '_'))
        {
            return Err(InstrumentError::InvalidQuery);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InstrumentSuggestion {
    pub symbol: String,
    pub name: String,
    pub market: String,
    pub category: String,
    pub currency: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub indicative_price: Decimal,
    pub source: String,
    pub source_instrument_id: String,
    pub as_of_unix: i64,
}

impl InstrumentSuggestion {
    pub fn new(input: InstrumentSuggestionInput<'_>) -> Result<Self, InstrumentError> {
        let symbol =
            AssetSymbol::parse(input.symbol).map_err(|_| InstrumentError::InvalidProviderData)?;
        let market =
            Market::parse(input.market).map_err(|_| InstrumentError::InvalidProviderData)?;
        let category = AssetCategory::parse(input.category)
            .map_err(|_| InstrumentError::InvalidProviderData)?;
        let currency =
            Currency::parse(input.currency).map_err(|_| InstrumentError::InvalidProviderData)?;

        let name = input.name.trim();
        let source = input.source.trim();
        let source_instrument_id = input.source_instrument_id.trim();
        if name.is_empty()
            || name.len() > 160
            || source.is_empty()
            || source.len() > 64
            || source_instrument_id.is_empty()
            || source_instrument_id.len() > 160
            || input.indicative_price.is_sign_negative()
        {
            return Err(InstrumentError::InvalidProviderData);
        }

        Ok(Self {
            symbol: symbol.as_str().to_owned(),
            name: name.to_owned(),
            market: market.as_str().to_owned(),
            category: category.as_str().to_owned(),
            currency: currency.as_str().to_owned(),
            indicative_price: input.indicative_price,
            source: source.to_owned(),
            source_instrument_id: source_instrument_id.to_owned(),
            as_of_unix: OffsetDateTime::now_utc().unix_timestamp(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InstrumentSuggestionInput<'a> {
    pub symbol: &'a str,
    pub name: &'a str,
    pub market: &'a str,
    pub category: &'a str,
    pub currency: &'a str,
    pub indicative_price: Decimal,
    pub source: &'a str,
    pub source_instrument_id: &'a str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentCacheStatus {
    Fresh,
    Hit,
    Stale,
    Miss,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct InstrumentSearchResult {
    pub items: Vec<InstrumentSuggestion>,
    pub cache: InstrumentCacheStatus,
}

#[derive(Debug, thiserror::Error)]
pub enum InstrumentError {
    #[error("consulta de instrumento inválida")]
    InvalidQuery,
    #[error("provider indisponível")]
    ProviderUnavailable,
    #[error("resposta inválida do provider")]
    InvalidProviderData,
}

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn normalizes_query() {
        let query = InstrumentSearchQuery::parse(" petr4 ").unwrap();

        assert_eq!(query.as_str(), "PETR4");
    }

    #[test]
    fn rejects_invalid_query() {
        assert!(InstrumentSearchQuery::parse("p").is_err());
        assert!(InstrumentSearchQuery::parse("PETR4;DROP").is_err());
    }

    #[test]
    fn rejects_invalid_provider_payload() {
        assert!(
            InstrumentSuggestion::new(InstrumentSuggestionInput {
                symbol: "PETR4",
                name: "Petrobras PN",
                market: "B3",
                category: "STOCK",
                currency: "BRL",
                indicative_price: dec!(-1),
                source: "local",
                source_instrument_id: "PETR4",
            })
            .is_err()
        );
    }
}
