-- The user accounts table
-- User accounts are indexed on wallet public keys (for credits).
-- Customer ids for debits.
-- Users can associate multiple public keys to an account.
-- Users can associate multiple shopify customer ids to an account.
-- The current balance is the credit balance of Tari currently linked to the account. This will be used to pay
-- for orders as they come in.


CREATE TABLE IF NOT EXISTS user_accounts
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    -- The total amount of Tari received by the account in the account's history
    total_received  INTEGER NOT NULL    DEFAULT 0,
    -- The amount of Tari pending from unconfirmed deposits in the account
    current_pending   INTEGER NOT NULL    DEFAULT 0,
    -- The amount of Tari currently available for spending. i.e. the credit balance
    current_balance INTEGER NOT NULL    DEFAULT 0,
    -- The total amount of Tari spent by the account in the account's history
    total_orders    INTEGER NOT NULL    DEFAULT 0,
    -- The total value of orders currently waiting for payment
    current_orders  INTEGER NOT NULL    DEFAULT 0
);

CREATE TABLE IF NOT EXISTS user_account_public_keys
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_account_id INTEGER REFERENCES user_accounts (id) NOT NULL,
    public_key      TEXT UNIQUE NOT NULL,
    created_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_account_customer_ids
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    user_account_id INTEGER REFERENCES user_accounts (id) NOT NULL,
    customer_id     TEXT UNIQUE NOT NULL,
    created_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS user_accounts_id ON user_accounts (id);
CREATE INDEX IF NOT EXISTS user_account_public_keys_user_account_id ON user_account_public_keys (user_account_id);
CREATE INDEX IF NOT EXISTS user_account_public_keys_public_key ON user_account_public_keys (public_key);

CREATE INDEX IF NOT EXISTS user_account_customer_ids_customer_id ON user_account_customer_ids (customer_id);
CREATE INDEX IF NOT EXISTS user_account_customer_ids_user_account_id ON user_account_customer_ids (user_account_id);

-- Adjust the total orders received total when an order amount is updated.
CREATE TRIGGER order_updated_trigger AFTER UPDATE OF total_price ON orders
BEGIN
  UPDATE user_accounts
  SET
    total_orders = total_orders + (NEW.total_price - OLD.total_price),
    current_orders = current_orders + (NEW.total_price - OLD.total_price)
  WHERE id = (SELECT user_account_id FROM user_account_customer_ids WHERE customer_id = OLD.customer_id);
END;

-- Adjust total orders received balance down if an order is expired or cancelled.
CREATE TRIGGER order_cancelled_trigger AFTER UPDATE OF status ON orders
WHEN OLD.status = 'New' AND NEW.status in ('Cancelled', 'Expired')
BEGIN
    UPDATE user_accounts
    SET
      total_orders = total_orders - OLD.total_price,
      current_orders = current_orders - OLD.total_price
    WHERE id = (SELECT user_account_id FROM user_account_customer_ids WHERE customer_id = OLD.customer_id);
END;

-- CREATE TRIGGER order_created_trigger AFTER INSERT ON orders END..
-- We can't create this trigger in SQLite because we first need to try and associate/create an account for the customer
-- to the order. So the orders totals will be carried out in code.


