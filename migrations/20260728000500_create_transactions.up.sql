CREATE TABLE transactions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    asset_id UUID NOT NULL REFERENCES assets (id) ON DELETE RESTRICT,
    broker_id UUID NOT NULL REFERENCES brokers (id) ON DELETE RESTRICT,
    transaction_type VARCHAR(8) NOT NULL,
    quantity NUMERIC(28, 10) NOT NULL,
    unit_price NUMERIC(20, 8) NOT NULL,
    fees NUMERIC(20, 8) NOT NULL DEFAULT 0,
    occurred_at TIMESTAMPTZ NOT NULL,
    notes VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT transactions_type_allowed CHECK (transaction_type IN ('BUY', 'SELL')),
    CONSTRAINT transactions_quantity_positive CHECK (quantity > 0),
    CONSTRAINT transactions_unit_price_non_negative CHECK (unit_price >= 0),
    CONSTRAINT transactions_fees_non_negative CHECK (fees >= 0),
    CONSTRAINT transactions_notes_not_blank CHECK (notes IS NULL OR btrim(notes) <> '')
);

CREATE INDEX transactions_user_occurred_at_idx
    ON transactions (user_id, occurred_at DESC, id DESC);

CREATE INDEX transactions_user_asset_broker_idx
    ON transactions (user_id, asset_id, broker_id);
