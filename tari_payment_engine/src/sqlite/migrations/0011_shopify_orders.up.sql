CREATE TABLE shopify_transactions (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL,
    amount TEXT NOT NULL,
    currency TEXT NOT NULL,
    test BOOLEAN NOT NULL,
    captured BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX shopify_tx_captured ON shopify_transactions (captured);
CREATE INDEX shopify_tx_orderid ON shopify_transactions (order_id);

ALTER TABLE orders ADD COLUMN amount_outstanding TEXT;
