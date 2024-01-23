DROP TRIGGER IF EXISTS update_account_total;
DROP TRIGGER IF EXISTS create_account_on_payment;
DROP TRIGGER IF EXISTS adjust_account_total;

DROP TABLE IF EXISTS user_account_customer_ids;
DROP TABLE IF EXISTS user_account_public_keys;
DROP TABLE IF EXISTS user_accounts;

DROP INDEX IF EXISTS user_accounts_id;
DROP INDEX IF EXISTS user_account_public_keys_user_account_id;
DROP INDEX IF EXISTS user_account_public_keys_public_key;
DROP INDEX IF EXISTS user_account_customer_ids_customer_id;
DROP INDEX IF EXISTS user_account_customer_ids_user_account_id;
