ALTER TABLE brokers
    ADD CONSTRAINT brokers_name_max_length
        CHECK (char_length(name) <= 50) NOT VALID,
    ADD CONSTRAINT brokers_name_normalized_max_length
        CHECK (char_length(name_normalized) <= 50) NOT VALID;

ALTER TABLE assets
    ADD CONSTRAINT assets_symbol_max_length
        CHECK (char_length(symbol) <= 20) NOT VALID,
    ADD CONSTRAINT assets_symbol_normalized_max_length
        CHECK (char_length(symbol_normalized) <= 20) NOT VALID,
    ADD CONSTRAINT assets_name_max_length
        CHECK (char_length(name) <= 120) NOT VALID;
