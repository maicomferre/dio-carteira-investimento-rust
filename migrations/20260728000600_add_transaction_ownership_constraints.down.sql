ALTER TABLE transactions
    DROP CONSTRAINT transactions_broker_same_user_fk,
    DROP CONSTRAINT transactions_asset_same_user_fk;

DROP INDEX brokers_id_user_id_unique;
DROP INDEX assets_id_user_id_unique;
