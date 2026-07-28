CREATE UNIQUE INDEX assets_id_user_id_unique
    ON assets (id, user_id);

CREATE UNIQUE INDEX brokers_id_user_id_unique
    ON brokers (id, user_id);

ALTER TABLE transactions
    ADD CONSTRAINT transactions_asset_same_user_fk
        FOREIGN KEY (asset_id, user_id)
        REFERENCES assets (id, user_id)
        ON DELETE RESTRICT,
    ADD CONSTRAINT transactions_broker_same_user_fk
        FOREIGN KEY (broker_id, user_id)
        REFERENCES brokers (id, user_id)
        ON DELETE RESTRICT;
