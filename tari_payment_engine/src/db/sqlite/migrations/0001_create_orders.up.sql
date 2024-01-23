CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    order_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    memo TEXT,
    total_price INTEGER NOT NULL,
    currency TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL CHECK(status IN ('Paid', 'Cancelled', 'Expired', 'New')) DEFAULT 'New'
);

CREATE INDEX orders_order_id_idx ON orders (order_id);
CREATE INDEX orders_order_history ON orders (order_id, id);
CREATE INDEX orders_cid_oid_idx ON orders (customer_id, order_id);

CREATE INDEX orders_status_idx ON orders (status);
CREATE INDEX orders_customer_idx ON orders (customer_id);
