-- The user accounts table
-- User accounts are indexed on wallet public keys (for credits).
-- Customer ids for debits.
-- Users can associate multiple public keys to an account.
-- Users can associate multiple shopify customer ids to an account.
-- The total balance is the sum of all non-cancelled payments received by associated public keys minus total order
-- values.
-- The available balance is the sum of all confirmed payments made by associated customer ids minus total order values.

CREATE TABLE IF NOT EXISTS user_accounts
(
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL  DEFAULT CURRENT_TIMESTAMP,
    total_received  INTEGER NOT NULL    DEFAULT 0,
    total_pending   INTEGER NOT NULL    DEFAULT 0,
    current_balance INTEGER NOT NULL    DEFAULT 0,
    total_orders    INTEGER NOT NULL    DEFAULT 0
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

CREATE TRIGGER order_updated_trigger AFTER UPDATE OF total_price ON orders
BEGIN
  UPDATE user_accounts
  SET total_orders = total_orders + (NEW.total_price - OLD.total_price)
  WHERE id = (SELECT user_account_id FROM user_account_customer_ids WHERE customer_id = NEW.customer_id);
END;

