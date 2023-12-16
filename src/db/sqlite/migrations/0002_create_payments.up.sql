CREATE TABLE payments (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    timestamp    DATETIME                          NOT NULL,
    block_height INTEGER                           NOT NULL,
    sender       TEXT                              NOT NULL,
    receiver     TEXT                              NOT NULL,
    amount       INTEGER                           NOT NULL,
    memo         TEXT
);

CREATE INDEX payments_id_idx ON payments (id);
CREATE INDEX payments_sender_idx ON payments (sender);

