DROP TRIGGER IF EXISTS orders_no_delete
DROP INDEX payments_status_idx;
DROP INDEX payments_sender_idx;
DROP INDEX payments_id_idx;
DROP INDEX payments_id_orderid;

DROP TABLE payments;
