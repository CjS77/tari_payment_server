CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    order_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    memo TEXT,
    total_price INTEGER NOT NULL,
    currency TEXT NOT NULL,
    timestamp DATETIME NOT NULL
);

CREATE INDEX orders_order_id_idx ON orders (order_id);
CREATE INDEX orders_order_history ON orders (order_id, id);
