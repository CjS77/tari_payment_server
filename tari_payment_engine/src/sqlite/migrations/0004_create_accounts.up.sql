CREATE TABLE IF NOT EXISTS address_customer_id_link
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address         TEXT NOT NULL,
    customer_id     TEXT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (address, customer_id) ON CONFLICT IGNORE
);



CREATE INDEX IF NOT EXISTS address_links ON address_customer_id_link (address);
CREATE INDEX IF NOT EXISTS custid_links ON address_customer_id_link (customer_id);
CREATE INDEX IF NOT EXISTS join_links ON address_customer_id_link (address, customer_id);

CREATE TABLE settlement_journal (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    order_id        TEXT NOT NULL references orders (order_id),
    payment_address TEXT NOT NULL,
    settlement_type TEXT NOT NULL CHECK (settlement_type IN ('Multiple', 'Single')),
    amount          INTEGER NOT NULL
);

CREATE VIEW IF NOT EXISTS address_pending_balance (address, status, balance) AS
SELECT
    sender,
    status,
    SUM(amount)
FROM payments
WHERE status = 'Received'
GROUP BY sender;

CREATE VIEW IF NOT EXISTS address_balance (address, total_confirmed, total_paid, current_balance, last_update) AS
WITH
    wallets AS (
    SELECT sender, sum(amount) as total_confirmed, updated_at
    FROM payments
    WHERE status = 'Confirmed'
    GROUP BY sender
),
    settlements AS (
    SELECT sum(amount) as total, payment_address, created_at
    FROM settlement_journal
    GROUP BY payment_address
)
SELECT
    wallets.sender as address,
    wallets.total_confirmed as total_confirmed,
    coalesce(settlements.total, 0) as total_paid,
    wallets.total_confirmed - coalesce(settlements.total, 0) as current_balance,
    coalesce(settlements.created_at, wallets.updated_at) as last_update
FROM wallets
LEFT OUTER JOIN settlements ON wallets.sender = settlements.payment_address;


CREATE VIEW IF NOT EXISTS customer_order_balance (customer_id, status, total_orders) AS
SELECT
    customer_id,
    status,
    SUM(total_price)
FROM orders group by customer_id, status;
