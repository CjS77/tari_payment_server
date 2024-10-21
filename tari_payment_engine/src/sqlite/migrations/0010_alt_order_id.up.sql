-- The alternative order id is an optional (unique) additional identifier for an order. The order_id is still
-- considered the primary id, but some queries can also search over the alt_id.
-- Also see https://sqlite.org/faq.html#q26
ALTER TABLE orders ADD COLUMN alt_id TEXT;
ALTER TABLE payments ADD COLUMN alt_order_id TEXT;

CREATE UNIQUE INDEX orders_alt_id_idx ON orders (alt_id);
CREATE INDEX payments_alt_order_id_idx ON payments (alt_order_id);
