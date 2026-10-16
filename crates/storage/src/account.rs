use nabungo_core::{AccountKind, AccountRef, CurrencyCode};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Lookup, Result, StorageError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub name: String,
    pub kind: AccountKind,
    pub currency: CurrencyCode,
}

pub fn insert_account(tx: &Transaction, new: &NewAccount, now: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO account (id, name, kind, currency, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            id.to_string(),
            new.name,
            new.kind.as_str(),
            new.currency.as_str(),
            now
        ],
    )?;
    Ok(id)
}

pub fn get_account_ref(conn: &Connection, id: Uuid) -> Result<Option<Lookup<AccountRef>>> {
    let row = conn
        .query_row(
            "SELECT kind, currency, archived_at IS NOT NULL FROM account WHERE id = ?1",
            [id.to_string()],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            },
        )
        .optional()?;
    let Some((kind, currency, archived)) = row else {
        return Ok(None);
    };
    let kind = AccountKind::parse(&kind)
        .ok_or_else(|| StorageError::Corrupt(format!("account {id}: unknown kind {kind:?}")))?;
    let currency = CurrencyCode::parse(&currency)
        .map_err(|e| StorageError::Corrupt(format!("account {id}: {e}")))?;
    Ok(Some(Lookup {
        value: AccountRef { id, kind, currency },
        archived,
    }))
}
