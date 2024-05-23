//! Sqlite database operations for the tari payment authentication
//!
//! Generally clients should never call these methods directly, and prefer to use the [`AuthManagement`] trait methods.
//! that is implemented on the [`SqliteDatabase`] struct instead.

use std::collections::HashMap;

use log::{debug, error};
use sqlx::{QueryBuilder, Row, SqliteConnection};
use tari_common_types::tari_address::TariAddress;

use crate::{db_types::Role, traits::AuthApiError};

pub async fn auth_account_exists(address: &TariAddress, conn: &mut SqliteConnection) -> Result<bool, AuthApiError> {
    let address = address.to_hex();
    let row = sqlx::query!(r#"SELECT count(address) as "count" FROM auth_log WHERE address = ?"#, address)
        .fetch_one(conn)
        .await?;
    let count = row.count;
    match count {
        0 => Ok(false),
        1 => Ok(true),
        n => {
            error!("Account {address} appears multiple {n} times in database. This must be 0|1!");
            Err(AuthApiError::DatabaseError(
                "Account appears multiple times in database. Report this to the developers".to_string(),
            ))
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
        .map(|r| r.parse::<Role>().map_err(|_| AuthApiError::RoleNotFound))
        .collect::<Result<Vec<Role>, _>>()?;
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
    #[allow(clippy::cast_possible_truncation)]
    let num_matching_roles = sqlx::query(&q).bind(address).fetch_one(conn).await?.get::<i64, usize>(0) as usize;
    if num_matching_roles == roles.len() {
        Ok(())
    } else {
        let n = roles.len().saturating_sub(num_matching_roles);
        Err(AuthApiError::RoleNotAllowed(n))
    }
}

pub async fn upsert_nonce_for_address(
    address: &TariAddress,
    nonce: u64,
    conn: &mut SqliteConnection,
) -> Result<(), AuthApiError> {
    let address = address.to_hex();
    #[allow(clippy::cast_possible_wrap)]
    let nonce = nonce as i64;
    let res = sqlx::query!(
        r#"INSERT INTO auth_log (address, last_nonce) VALUES (?, ?) ON CONFLICT(address) DO
    UPDATE SET last_nonce = excluded.last_nonce"#,
        nonce,
        address
    )
    .execute(conn)
    .await;
    debug!("{res:?}");
    res.map_err(|e| {
        if let sqlx::Error::Database(ref de) = e {
            if let Some(code) = de.code() {
                // TRIGGER on increasing nonce violation
                if code.as_ref() == "1811" {
                    return AuthApiError::InvalidNonce;
                }
            }
        }
        AuthApiError::from(e)
    })
    .and_then(|res| match res.rows_affected() {
        0 => Err(AuthApiError::AddressNotFound),
        1 => Ok(()),
        _ => unreachable!("Updating auth log should only affect one row"),
    })
}

async fn fetch_roles(conn: &mut SqliteConnection) -> Result<HashMap<Role, i64>, AuthApiError> {
    let result = sqlx::query!("SELECT id, name FROM roles").fetch_all(conn).await?;
    let roles = result
        .iter()
        .map(|r| r.name.parse::<Role>().map(|role| (role, r.id)).map_err(|_| AuthApiError::RoleNotFound))
        .collect::<Result<HashMap<_, _>, _>>()?;
    debug!("Fetched current roles table: {:?}", roles);
    Ok(roles)
}

pub async fn assign_roles(
    address: &TariAddress,
    roles: &[Role],
    conn: &mut SqliteConnection,
) -> Result<(), AuthApiError> {
    let all_roles = fetch_roles(conn).await?;

    let role_ids = roles
        .iter()
        .map(|r| all_roles.get(r).ok_or(AuthApiError::RoleNotFound).copied())
        .collect::<Result<Vec<i64>, _>>()?;
    let address = address.to_hex();

    let mut qb = QueryBuilder::new("INSERT INTO role_assignments (address, role_id) VALUES ");
    let mut values = qb.separated(", ");
    for role_id in role_ids {
        values.push("(");
        values.push_bind_unseparated(address.clone());
        values.push_unseparated(", ");
        values.push_bind_unseparated(role_id);
        values.push_unseparated(")");
    }
    let res = qb.build().execute(conn).await?;

    if res.rows_affected() == roles.len() as u64 {
        Ok(())
    } else {
        error!("Expected to insert {} roles, but inserted {}", roles.len(), res.rows_affected());
        Err(AuthApiError::DatabaseError(
            "Inserted unexpected number of Roles. Report this to the developers".to_string(),
        ))
    }
}

pub async fn remove_roles(
    address: &TariAddress,
    roles: &[Role],
    conn: &mut SqliteConnection,
) -> Result<u64, AuthApiError> {
    let all_roles = fetch_roles(conn).await?;

    let role_ids = roles
        .iter()
        .map(|r| all_roles.get(r).ok_or(AuthApiError::RoleNotFound).copied())
        .collect::<Result<Vec<i64>, _>>()?;

    let address = address.to_hex();

    let mut qb = QueryBuilder::new("DELETE FROM role_assignments WHERE address = ");
    qb.push_bind(address.clone());
    qb.push(" AND role_id IN (");
    let mut values = qb.separated(", ");
    role_ids.iter().for_each(|id| {
        values.push_bind(*id);
    });
    qb.push(")");
    let res = qb.build().execute(conn).await?;

    Ok(res.rows_affected())
}
