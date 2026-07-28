CREATE TABLE assets (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    symbol VARCHAR(32) NOT NULL,
    symbol_normalized VARCHAR(32) NOT NULL,
    name VARCHAR(160) NOT NULL,
    market VARCHAR(16) NOT NULL,
    category VARCHAR(24) NOT NULL,
    currency CHAR(3) NOT NULL,
    current_price NUMERIC(20, 8) NOT NULL,
    external_id TEXT,
    metadata_source VARCHAR(64),
    metadata_fetched_at TIMESTAMPTZ,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT assets_symbol_not_blank CHECK (btrim(symbol) <> ''),
    CONSTRAINT assets_symbol_normalized_not_blank CHECK (btrim(symbol_normalized) <> ''),
    CONSTRAINT assets_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT assets_market_allowed CHECK (market IN ('B3', 'NASDAQ', 'NYSE', 'CRYPTO', 'OTHER')),
    CONSTRAINT assets_category_allowed CHECK (category IN ('STOCK', 'FII', 'ETF', 'BDR', 'CRYPTO', 'OTHER')),
    CONSTRAINT assets_currency_allowed CHECK (currency IN ('BRL', 'USD')),
    CONSTRAINT assets_current_price_non_negative CHECK (current_price >= 0),
    CONSTRAINT assets_version_positive CHECK (version > 0)
);

CREATE UNIQUE INDEX assets_user_symbol_market_unique
    ON assets (user_id, symbol_normalized, market);

CREATE INDEX assets_user_id_idx
    ON assets (user_id);
