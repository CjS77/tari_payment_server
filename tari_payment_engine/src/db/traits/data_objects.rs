pub enum InsertOrderResult {
    Inserted(i64),
    AlreadyExists(i64),
}

pub enum InsertPaymentResult {
    Inserted(String),
    AlreadyExists(String),
}
