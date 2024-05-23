use std::net::SocketAddr;

use sqlx::{query, Row, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::traits::{WalletAuthApiError, WalletInfo};

pub async fn fetch_wallet_info_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<WalletInfo, WalletAuthApiError> {
    let address = address.to_hex();
    sqlx::query(r#"SELECT * FROM wallet_auth WHERE address = ?"#)
        .bind(address)
        .fetch_optional(conn)
        .await?
        .and_then(|row| {
            let ip_address = row.get::<&str, _>("ip_address").parse::<SocketAddr>().ok()?;
            let address = row.get("address");
            let last_nonce = row.get("last_nonce");
            Some(WalletInfo { address, ip_address, last_nonce })
        })
        .ok_or(WalletAuthApiError::WalletNotFound)
}

pub async fn update_wallet_nonce(
    address: &TariAddress,
    new_nonce: i64,
    conn: &mut SqliteConnection,
) -> Result<(), WalletAuthApiError> {
    let address = address.to_hex();
    let result = query!(r#"UPDATE wallet_auth SET last_nonce = ? WHERE address = ?"#, new_nonce, address)
        .execute(conn)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref de) = e {
                if let Some(code) = de.code() {
                    // TRIGGER on increasing nonce violation
                    if code.as_ref() == "1811" {
                        return WalletAuthApiError::InvalidNonce;
                    }
                }
            }
            WalletAuthApiError::from(e)
        })?;
    if result.rows_affected() == 0 {
        return Err(WalletAuthApiError::WalletNotFound);
    }
    Ok(())
}
