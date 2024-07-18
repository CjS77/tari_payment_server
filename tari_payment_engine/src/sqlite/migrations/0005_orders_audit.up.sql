-- order_id = 1
-- customer_id = 2
-- memo = 4
-- total_price = 8
-- currency = 16
-- status = 32
-- original_price = 64

CREATE TABLE orders_log
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    oid             INTEGER                           NOT NULL REFERENCES orders (id),
    columns_changed INTEGER                           NOT NULL,
    old_order_id    TEXT,
    new_order_id    TEXT,
    old_customer_id TEXT,
    new_customer_id TEXT,
    old_memo        TEXT,
    new_memo        TEXT,
    old_total_price INTEGER,
    new_total_price INTEGER,
    old_original_price TEXT,
    new_original_price TEXT,
    old_currency    TEXT,
    new_currency    TEXT,
    old_status      TEXT,
    new_status      TEXT,
    updated_at      DATETIME                          NOT NULL
);

CREATE INDEX orders_log_oid ON orders_log (oid);
CREATE INDEX orders_log_updated_at ON orders_log (updated_at);
CREATE INDEX orders_log_columns_changed ON orders_log (columns_changed);

-- Trigger to log changes to orders
CREATE TRIGGER orders_log_update
    AFTER UPDATE
    ON orders
BEGIN
    INSERT INTO orders_log (oid,
                            columns_changed,
                            old_order_id,
                            new_order_id,
                            old_customer_id,
                            new_customer_id,
                            old_memo,
                            new_memo,
                            old_total_price,
                            new_total_price,
                            old_original_price,
                            new_original_price,
                            old_currency,
                            new_currency,
                            old_status,
                            new_status,
                            updated_at)
    VALUES (NEW.id,
            iif(OLD.order_id != NEW.order_id, 1, 0) +
            iif(OLD.customer_id != NEW.customer_id, 2, 0) +
            iif(OLD.memo != NEW.memo, 4, 0) +
            iif(OLD.total_price != NEW.total_price, 8, 0) +
            iif(OLD.currency != NEW.currency, 16, 0) +
            iif(OLD.status != NEW.status, 32, 0) +
            iif(OLD.original_price != NEW.original_price, 64, 0),
            nullif(OLD.order_id, NEW.order_id),
            nullif(NEW.order_id, OLD.order_id),
            nullif(OLD.customer_id, NEW.customer_id),
            nullif(NEW.customer_id, OLD.customer_id),
            nullif(OLD.memo, NEW.memo),
            nullif(NEW.memo, OLD.memo),
            nullif(OLD.total_price, NEW.total_price),
            nullif(NEW.total_price, OLD.total_price),
            nullif(OLD.original_price, NEW.original_price),
            nullif(NEW.original_price, OLD.original_price),
            nullif(OLD.currency, NEW.currency),
            nullif(NEW.currency, OLD.currency),
            nullif(OLD.status, NEW.status),
            nullif(NEW.status, OLD.status),
            NEW.updated_at);
END;

CREATE TRIGGER orders_log_insert
    AFTER INSERT
    ON orders
BEGIN
    INSERT INTO orders_log (oid,
                            columns_changed,
                            new_order_id,
                            new_customer_id,
                            new_memo,
                            new_total_price,
                            new_currency,
                            new_status,
                            updated_at)
    VALUES (NEW.id,
            1 + 2 + 4 + 8 + 16 + 32,
            NEW.order_id,
            NEW.customer_id,
            NEW.memo,
            NEW.total_price,
            NEW.currency,
            NEW.status,
            NEW.updated_at);
END;



