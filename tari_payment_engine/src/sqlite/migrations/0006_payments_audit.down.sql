DROP TRIGGER IF EXISTS payments_log_insert;
DROP TRIGGER IF EXISTS payments_log_update;

DROP INDEX IF EXISTS payments_log_columns_changed;
DROP INDEX IF EXISTS payments_log_updated_at;
DROP INDEX IF EXISTS payments_log_oid;

DROP TABLE IF EXISTS payments_log;
