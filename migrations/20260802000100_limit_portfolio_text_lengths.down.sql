ALTER TABLE assets
    DROP CONSTRAINT assets_name_max_length,
    DROP CONSTRAINT assets_symbol_normalized_max_length,
    DROP CONSTRAINT assets_symbol_max_length;

ALTER TABLE brokers
    DROP CONSTRAINT brokers_name_normalized_max_length,
    DROP CONSTRAINT brokers_name_max_length;
