DROP INDEX IF EXISTS role_assignments_role_id_idx;
DROP INDEX IF EXISTS role_assignments_address_idx;
DROP INDEX IF EXISTS auth_log_address_idx;

DROP TABLE IF EXISTS role_assignments;
DROP TABLE IF EXISTS roles;
DROP TABLE IF EXISTS auth_log;
