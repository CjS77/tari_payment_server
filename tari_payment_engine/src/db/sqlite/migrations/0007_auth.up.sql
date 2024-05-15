CREATE TABLE auth_log (
    address TEXT NOT NULL UNIQUE,
    last_nonce INTEGER NOT NULL
);

CREATE TRIGGER auth_log_update_nonce
    BEFORE UPDATE OF last_nonce ON auth_log
    BEGIN
        SELECT RAISE(ABORT, 'nonce must strictly increase')
        WHERE NEW.last_nonce <= OLD.last_nonce;
    END;

CREATE TABLE roles (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
);

INSERT INTO roles VALUES
      (1, 'user'),
      (2, 'read_all'),
      (3, 'write'),
      (4, 'payment_wallet'),
      (5, 'super_admin')
;

CREATE TABLE role_assignments
(
    address INTEGER NOT NULL,
    role_id INTEGER NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    PRIMARY KEY (address, role_id) ON CONFLICT IGNORE
);

CREATE INDEX auth_log_address_idx ON auth_log(address);
CREATE INDEX role_assignments_address_idx ON role_assignments(address);
CREATE INDEX role_assignments_role_id_idx ON role_assignments(role_id);
