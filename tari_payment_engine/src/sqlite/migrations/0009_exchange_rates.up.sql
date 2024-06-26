CREATE TABLE if not exists exchange_rates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    base_currency TEXT NOT NULL,
    rate INTEGER NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX if not exists exchange_rates_currency ON exchange_rates (base_currency);

--Disable deletes
CREATE TRIGGER if not exists exchange_rates_no_delete BEFORE DELETE ON exchange_rates
BEGIN
    SELECT RAISE(FAIL, 'Deletes are not allowed on exchange_rates');
END;

