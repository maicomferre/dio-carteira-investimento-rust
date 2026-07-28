use crate::{
    application::error::AppError,
    domain::instrument::{InstrumentError, InstrumentSearchQuery, InstrumentSearchResult},
    infrastructure::instrument_provider::CachedInstrumentProvider,
};

pub async fn search_instruments(
    provider: &CachedInstrumentProvider,
    query: &str,
) -> Result<InstrumentSearchResult, AppError> {
    let query = InstrumentSearchQuery::parse(query).map_err(map_instrument_error)?;

    provider.search(&query).await.map_err(map_instrument_error)
}

fn map_instrument_error(error: InstrumentError) -> AppError {
    match error {
        InstrumentError::InvalidQuery | InstrumentError::InvalidProviderData => {
            AppError::Validation("busca de instrumento inválida")
        }
        InstrumentError::ProviderUnavailable => AppError::Unavailable,
    }
}
