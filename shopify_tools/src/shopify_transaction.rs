use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ShopifyTransaction {
    pub id: i64,
    pub order_id: i64,
    pub amount: String,
    pub authorization: Option<String>,
    pub authorization_expires_at: Option<String>,
    pub created_at: String,
    pub currency: String,
    pub device_id: Option<i64>,
    pub error_code: Option<String>,
    pub gateway: Option<String>,
    pub kind: String,
    pub message: String,
    pub parent_id: Option<i64>,
    pub processed_at: String,
    pub source_name: String,
    pub status: String,
    pub total_unsettled_set: TotalUnsettledSet,
    pub test: bool,
    pub user_id: Option<i64>,
    pub currency_exchange_adjustment: Option<CurrencyExchangeAdjustment>,
}

#[derive(Serialize, Deserialize)]
pub struct CurrencyExchangeAdjustment {
    pub id: i64,
    pub adjustment: String,
    pub original_amount: String,
    pub final_amount: String,
    pub currency: String,
}

#[derive(Serialize, Deserialize)]
pub struct OutstandingValue {
    pub amount: String,
    pub currency: String,
}

#[derive(Serialize, Deserialize)]
pub struct TotalUnsettledSet {
    pub presentment_money: OutstandingValue,
    pub shop_money: OutstandingValue,
}

#[derive(Serialize, Deserialize)]
pub struct ShopifyPaymentCapture {
    pub transaction: CaptureTransaction,
}

#[derive(Serialize, Deserialize)]
pub struct CaptureTransaction {
    pub parent_id: i64,
    pub kind: String,
    pub amount: String,
    pub currency: String,
    pub test: bool,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_shopify_transaction() {
        let json = include_str!("./test_assets/transaction1.json");
        let tx: ShopifyTransaction = serde_json::from_str(json).unwrap();
        assert_eq!(tx.amount, "6.00");
        assert_eq!(tx.id, 6674280546516);
    }
}
