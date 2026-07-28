use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use rust_decimal::Decimal;
use tokio::sync::RwLock;

use crate::domain::instrument::{
    InstrumentCacheStatus, InstrumentError, InstrumentSearchQuery, InstrumentSearchResult,
    InstrumentSuggestion, InstrumentSuggestionInput,
};

#[derive(Debug, Clone)]
pub struct InstrumentProviderConfig {
    pub timeout: Duration,
    pub cache_ttl: Duration,
    pub stale_ttl: Duration,
    pub max_results: usize,
}

#[derive(Debug, Clone)]
pub struct CachedInstrumentProvider {
    source: LocalInstrumentProvider,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    config: InstrumentProviderConfig,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    result: Vec<InstrumentSuggestion>,
    stored_at: Instant,
}

impl CachedInstrumentProvider {
    pub fn new(config: InstrumentProviderConfig) -> Self {
        Self {
            source: LocalInstrumentProvider::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub async fn search(
        &self,
        query: &InstrumentSearchQuery,
    ) -> Result<InstrumentSearchResult, InstrumentError> {
        let cached = self.cache.read().await.get(query.as_str()).cloned();
        if let Some(entry) = cached.as_ref()
            && entry.stored_at.elapsed() <= self.config.cache_ttl
        {
            return Ok(InstrumentSearchResult {
                items: entry.result.clone(),
                cache: InstrumentCacheStatus::Hit,
            });
        }

        let provider_result =
            tokio::time::timeout(self.config.timeout, self.source.search(query)).await;

        match provider_result {
            Ok(Ok(items)) => {
                let items = items
                    .into_iter()
                    .take(self.config.max_results)
                    .collect::<Vec<_>>();
                self.cache.write().await.insert(
                    query.as_str().to_owned(),
                    CacheEntry {
                        result: items.clone(),
                        stored_at: Instant::now(),
                    },
                );

                Ok(InstrumentSearchResult {
                    items,
                    cache: InstrumentCacheStatus::Fresh,
                })
            }
            Ok(Err(error)) => stale_or_error(cached, self.config.stale_ttl, error),
            Err(_) => stale_or_error(
                cached,
                self.config.stale_ttl,
                InstrumentError::ProviderUnavailable,
            ),
        }
    }
}

fn stale_or_error(
    cached: Option<CacheEntry>,
    stale_ttl: Duration,
    error: InstrumentError,
) -> Result<InstrumentSearchResult, InstrumentError> {
    if let Some(entry) = cached
        && entry.stored_at.elapsed() <= stale_ttl
    {
        return Ok(InstrumentSearchResult {
            items: entry.result,
            cache: InstrumentCacheStatus::Stale,
        });
    }

    Err(error)
}

#[derive(Debug, Clone)]
struct LocalInstrumentProvider {
    instruments: Vec<InstrumentSuggestion>,
}

impl LocalInstrumentProvider {
    fn new() -> Self {
        let instruments = [
            ("PETR4", "Petrobras PN", "B3", "STOCK", "BRL", cents(3_842)),
            ("CMIG4", "Cemig PN", "B3", "STOCK", "BRL", cents(1_130)),
            ("ASAI3", "Assaí ON", "B3", "STOCK", "BRL", cents(920)),
            ("KLBN4", "Klabin PN", "B3", "STOCK", "BRL", cents(415)),
            (
                "AAPL",
                "Apple Inc.",
                "NASDAQ",
                "STOCK",
                "USD",
                cents(21_400),
            ),
            (
                "MSFT",
                "Microsoft Corp.",
                "NASDAQ",
                "STOCK",
                "USD",
                cents(50_500),
            ),
        ]
        .into_iter()
        .map(|(symbol, name, market, category, currency, price)| {
            InstrumentSuggestion::new(InstrumentSuggestionInput {
                symbol,
                name,
                market,
                category,
                currency,
                indicative_price: price,
                source: "local-fixture",
                source_instrument_id: symbol,
            })
            .expect("instrumento local válido")
        })
        .collect();

        Self { instruments }
    }

    async fn search(
        &self,
        query: &InstrumentSearchQuery,
    ) -> Result<Vec<InstrumentSuggestion>, InstrumentError> {
        let query = query.as_str();
        Ok(self
            .instruments
            .iter()
            .filter(|instrument| {
                instrument.symbol.starts_with(query)
                    || instrument.name.to_ascii_uppercase().contains(query)
            })
            .cloned()
            .collect())
    }
}

fn cents(value: i64) -> Decimal {
    Decimal::new(value, 2)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn provider() -> CachedInstrumentProvider {
        CachedInstrumentProvider::new(InstrumentProviderConfig {
            timeout: Duration::from_millis(50),
            cache_ttl: Duration::from_secs(60),
            stale_ttl: Duration::from_secs(300),
            max_results: 5,
        })
    }

    #[tokio::test]
    async fn finds_b3_instrument_and_normalizes_metadata() {
        let query = InstrumentSearchQuery::parse("petr4").unwrap();
        let result = provider().search(&query).await.unwrap();

        assert_eq!(result.cache, InstrumentCacheStatus::Fresh);
        assert_eq!(result.items[0].symbol, "PETR4");
        assert_eq!(result.items[0].currency, "BRL");
        assert_eq!(result.items[0].category, "STOCK");
    }

    #[tokio::test]
    async fn returns_cache_hit_after_first_lookup() {
        let provider = provider();
        let query = InstrumentSearchQuery::parse("PETR4").unwrap();

        let first = provider.search(&query).await.unwrap();
        let second = provider.search(&query).await.unwrap();

        assert_eq!(first.cache, InstrumentCacheStatus::Fresh);
        assert_eq!(second.cache, InstrumentCacheStatus::Hit);
    }

    #[tokio::test]
    async fn returns_empty_items_for_unknown_symbol() {
        let query = InstrumentSearchQuery::parse("ZZZZ3").unwrap();
        let result = provider().search(&query).await.unwrap();

        assert!(result.items.is_empty());
    }
}
