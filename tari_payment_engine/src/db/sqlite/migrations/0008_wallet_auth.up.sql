-- Tracks state for hot wallet authentication
CREATE TABLE wallet_auth (
    address TEXT NOT NULL UNIQUE PRIMARY KEY,
    ip_address TEXT NOT NULL,
    last_nonce INTEGER NOT NULL
);

CREATE TRIGGER wallet_auth_update_nonce
    BEFORE UPDATE OF last_nonce ON wallet_auth
    BEGIN
        SELECT RAISE(ABORT, 'nonce must strictly increase')
        WHERE NEW.last_nonce <= OLD.last_nonce;
    END;

CREATE TABLE wallet_auth_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    changed TEXT NOT NULL,
    address TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX wallet_auth_address_idx ON wallet_auth_log(address);
CREATE INDEX wallet_auth_updated_idx ON wallet_auth_log(updated_at);

CREATE TRIGGER on_wallet_auth_insert
    AFTER INSERT ON wallet_auth
    BEGIN
        INSERT INTO wallet_auth_log (address, ip_address, changed)
        VALUES (NEW.address, NEW.ip_address, "New Entry");
    END;

CREATE TRIGGER on_wallet_auth_update
    AFTER UPDATE OF address, ip_address ON wallet_auth
    BEGIN
        INSERT INTO wallet_auth_log (address, ip_address, changed)
        VALUES (
          NEW.address,
          NEW.ip_address,
          concat_ws(',',
            iif(OLD.address != NEW.address, 'address', NULL),
            iif(OLD.ip_address != NEW.ip_address, 'ip_address', NULL)
          )
        );
    END;

