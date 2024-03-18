DROP TRIGGER IF EXISTS orders_log_insert;
DROP TRIGGER IF EXISTS orders_log_update;

DROP INDEX IF EXISTS orders_log_columns_changed;
DROP INDEX IF EXISTS orders_log_updated_at;
DROP INDEX IF EXISTS orders_log_oid;

DROP TABLE IF EXISTS orders_log;
