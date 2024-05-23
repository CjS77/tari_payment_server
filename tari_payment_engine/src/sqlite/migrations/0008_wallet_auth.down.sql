DROP TRIGGER IF EXISTS on_wallet_auth_update;
DROP TRIGGER IF EXISTS on_wallet_auth_insert;

DROP INDEX IF EXISTS wallet_auth_updated_idx;
DROP INDEX IF EXISTS wallet_auth_address_idx;

DROP TABLE IF EXISTS wallet_auth_log;

DROP TRIGGER IF EXISTS wallet_auth_update_nonce;

DROP TABLE IF EXISTS wallet_auth;
