use std::net::IpAddr;

use sqlx::{query, Row, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::{
    db_types::SerializedTariAddress,
    traits::{NewWalletInfo, WalletAuthApiError, WalletInfo, WalletManagementError},
};

pub async fn fetch_wallet_info_for_address(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<WalletInfo, WalletAuthApiError> {
    let address = address.to_base58();
    sqlx::query(r#"SELECT * FROM wallet_auth WHERE address = ?"#)
        .bind(address)
        .fetch_optional(conn)
        .await?
        .and_then(|row| {
            let ip_address = row.get::<&str, _>("ip_address").parse::<IpAddr>().ok()?;
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
    let address = address.to_base58();
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

pub(crate) async fn register_wallet(
    info: NewWalletInfo,
    conn: &mut SqliteConnection,
) -> Result<(), WalletManagementError> {
    let address = info.address.as_base58();
    let ip_address = info.ip_address.to_string();
    let nonce = info.initial_nonce.unwrap_or(0);
    let result = query!(
        r#"INSERT INTO wallet_auth (address, ip_address, last_nonce) VALUES (?, ?, ?)"#,
        address,
        ip_address,
        nonce
    )
    .execute(conn)
    .await?;
    if result.rows_affected() == 0 {
        panic!("Find out what caused this... Wallet already registered?");
    }
    Ok(())
}

pub(crate) async fn deregister_wallet(
    address: &TariAddress,
    conn: &mut SqliteConnection,
) -> Result<(), WalletManagementError> {
    let address = address.to_base58();
    let result = query!(r#"DELETE FROM wallet_auth WHERE address = ?"#, address).execute(conn).await?;
    if result.rows_affected() == 0 {
        return Err(WalletManagementError::DatabaseError("Wallet not found".to_string()));
    }
    Ok(())
}

pub(crate) async fn fetch_authorized_wallets(
    conn: &mut SqliteConnection,
) -> Result<Vec<WalletInfo>, WalletManagementError> {
    query("SELECT * FROM wallet_auth")
        .fetch_all(conn)
        .await?
        .into_iter()
        .map(|row| {
            let ip_address = row
                .get::<&str, _>("ip_address")
                .parse::<IpAddr>()
                .map_err(|e| WalletManagementError::DatabaseError(format!("Invalid IP address. {e}")))?;
            let address = TariAddress::from_base58(row.get("address"))
                .map_err(|e| WalletManagementError::DatabaseError(format!("Invalid TariAddress. {e}")))?;
            let address = SerializedTariAddress::from(address);
            let last_nonce = row.get("last_nonce");
            Ok(WalletInfo { address, ip_address, last_nonce })
        })
        .collect::<Result<Vec<WalletInfo>, WalletManagementError>>()
}
