CREATE TABLE payments (
    txid         TEXT PRIMARY KEY NOT NULL,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP  NOT NULL,
    updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    sender       TEXT                              NOT NULL,
    amount       INTEGER                           NOT NULL,
    memo         TEXT,
    order_id     TEXT,
    payment_type TEXT NOT NULL CHECK (payment_type IN ('OnChain', 'Manual')) DEFAULT 'OnChain',
    status     TEXT NOT NULL CHECK (status IN ('Received', 'Confirmed', 'Cancelled')) DEFAULT 'Received'
);

CREATE INDEX payments_id_idx ON payments (txid);
CREATE INDEX payments_sender_idx ON payments (sender);
CREATE INDEX payments_status_idx ON payments (status);
CREATE INDEX payments_orderid_idx ON payments (order_id);
