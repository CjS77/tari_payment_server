use serde::{Deserialize, Serialize};
use tpg_common::MicroTari;

use crate::db_types::{Payment, SerializedTariAddress};

/// The reponse to `fetch_payments_for_address` calls. The array of payments is included along with the total value of
/// the payments and the address that the payments are associated with.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaymentsResult {
    pub address: SerializedTariAddress,
    pub total_payments: MicroTari,
    pub payments: Vec<Payment>,
}
