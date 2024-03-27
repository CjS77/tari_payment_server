use log::{debug, error};
use sqlx::{Row, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::{db_types::Role, AuthApiError};

pub async fn auth_account_exists(address: &TariAddress, conn: &mut SqliteConnection) -> Result<bool, AuthApiError> {
    let address = address.to_hex();
    let row = sqlx::query!(r#"SELECT count(address) as "count" FROM auth_log WHERE address = ?"#, address)
        .fetch_one(conn)
        .await
        .map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
    let count = row.count;
    match count {
        0 => Ok(false),
        1 => Ok(true),
        n => {
            error!("Account {address} appears multiple {n} times in database. This must be 0|1!");
            Err(AuthApiError::DatabaseError("Internal error. Report this to the developers".to_string()))
        },
    }
}

pub async fn roles_for_address(address: &TariAddress, conn: &mut SqliteConnection) -> Result<Vec<Role>, AuthApiError> {
    let address = address.to_hex();
    let result = sqlx::query!(
        r#"SELECT name FROM
            role_assignments LEFT JOIN roles ON role_assignments.role_id = roles.id
            WHERE address = ?"#,
        address
    )
    .fetch_all(conn)
    .await
    .map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
    let roles = result
        .iter()
        .filter_map(|r| r.name.as_ref())
        .map(|r| r.parse::<Role>())
        .collect::<Result<Vec<Role>, _>>()
        .map_err(|e| AuthApiError::DatabaseError(e.to_string()))?;
    Ok(roles)
}

pub async fn address_has_roles(
    address: &TariAddress,
    roles: &[Role],
    conn: &mut SqliteConnection,
) -> Result<(), AuthApiError> {
    let address = address.to_hex();
    let role_strings = roles.iter().map(|r| format!("'{r}'")).collect::<Vec<String>>().join(",");
    let q = format!(
        r#"SELECT count(name) as "num_roles"
                FROM role_assignments LEFT JOIN roles ON role_assignments.role_id = roles.id
                WHERE address = ? AND name IN ({role_strings})"#
    );
    let num_matching_roles = sqlx::query(&q)
        .bind(address)
        .fetch_one(conn)
        .await
        .map_err(|e| AuthApiError::DatabaseError(e.to_string()))?
        .get::<i64, usize>(0) as usize;
    if num_matching_roles == roles.len() {
        Ok(())
    } else {
        let n = roles.len().saturating_sub(num_matching_roles);
        Err(AuthApiError::RoleNotAllowed(n))
    }
}
pub async fn update_nonce_for_address(
    address: &TariAddress,
    nonce: u64,
    conn: &mut SqliteConnection,
) -> Result<(), AuthApiError> {
    let address = address.to_string();
    let nonce = nonce as i64;
    let res = sqlx::query!("UPDATE auth_log SET last_nonce = ? WHERE address = ?", nonce, address).execute(conn).await;
    res.map_err(|e| match e {
        sqlx::Error::Database(de) => {
            debug!("de stuff {} {}", de.is_check_violation(), de.to_string());
            AuthApiError::DatabaseError(de.to_string())
        },
        _ => AuthApiError::DatabaseError(e.to_string()),
    })
    .and_then(|res| match res.rows_affected() {
        0 => Err(AuthApiError::AddressNotFound),
        1 => Ok(()),
        _ => unreachable!("Updating auth log should only affect one row"),
    })
}
