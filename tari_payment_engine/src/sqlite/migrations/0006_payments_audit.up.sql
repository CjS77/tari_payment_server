-- sender = 1
-- amount = 2
-- memo = 4
-- payment_type = 8
-- status = 16

CREATE TABLE payments_log
(
    id               INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    txid             TEXT                              NOT NULL REFERENCES payments (txid),
    columns_changed  INTEGER                           NOT NULL,
    old_sender       TEXT,
    new_sender       TEXT,
    old_amount       INTEGER,
    new_amount       INTEGER,
    old_memo         TEXT,
    new_memo         TEXT,
    old_payment_type TEXT,
    new_payment_type TEXT,
    old_status       TEXT,
    new_status       TEXT,
    old_order_id     TEXT,
    new_order_id     TEXT,
    updated_at       DATETIME                          NOT NULL
);

CREATE INDEX payments_log_txid ON payments_log (txid);
CREATE INDEX payments_log_updated_at ON payments_log (updated_at);
CREATE INDEX payments_log_columns_changed ON payments_log (columns_changed);

-- Trigger to log changes to payments
CREATE TRIGGER payments_log_update AFTER UPDATE ON payments
BEGIN
    SELECT RAISE(FAIL, 'txid cannot be changed') WHERE NEW.txid != OLD.txid;
    INSERT INTO payments_log (txid,
                              columns_changed,
                              old_sender,
                              new_sender,
                              old_amount,
                              new_amount,
                              old_memo,
                              new_memo,
                              old_payment_type,
                              new_payment_type,
                              old_status,
                              new_status,
                              old_order_id,
                              new_order_id,
                              updated_at)
    VALUES (NEW.txid,
            iif(OLD.sender != NEW.sender, 1, 0) +
            iif(OLD.amount != NEW.amount, 2, 0) +
            iif(OLD.memo != NEW.memo, 4, 0) +
            iif(OLD.payment_type != NEW.payment_type, 8, 0) +
            iif(OLD.status != NEW.status, 16, 0) +
            iif(OLD.order_id != NEW.order_id, 32, 0),
            nullif(OLD.sender, NEW.sender),
            nullif(NEW.sender, OLD.sender),
            nullif(OLD.amount, NEW.amount),
            nullif(NEW.amount, OLD.amount),
            nullif(OLD.memo, NEW.memo),
            nullif(NEW.memo, OLD.memo),
            nullif(OLD.payment_type, NEW.payment_type),
            nullif(NEW.payment_type, OLD.payment_type),
            nullif(OLD.status, NEW.status),
            nullif(NEW.status, OLD.status),
            nullif(OLD.order_id, NEW.order_id),
            nullif(NEW.order_id, OLD.order_id),
            NEW.updated_at);
END;

CREATE TRIGGER payments_log_insert AFTER INSERT ON payments
BEGIN
    INSERT INTO payments_log (txid,
                              columns_changed,
                              new_sender,
                              new_amount,
                              new_memo,
                              new_payment_type,
                              new_status,
                              new_order_id,
                              updated_at
    ) VALUES (NEW.txid,
              1 + 2 + 4 + 8 + 16 + iif(NEW.order_id, 32, 0),
              NEW.sender,
              NEW.amount,
              NEW.memo,
              NEW.payment_type,
              NEW.status,
              NEW.order_id,
              NEW.updated_at);
END;



