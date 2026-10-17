//! Repository account group.

use rusqlite::{Connection, Transaction, params};
use uuid::Uuid;

use crate::{Result, map_unique, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountGroupRow {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i64,
}

pub fn insert_group(tx: &Transaction, name: &str) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO account_group (id, name, sort_order)
         VALUES (?1, ?2, COALESCE((SELECT MAX(sort_order) + 1 FROM account_group), 0))",
        params![id.to_string(), name],
    )
    .map_err(map_unique)?;
    Ok(id)
}

pub fn rename_group(tx: &Transaction, id: Uuid, name: &str) -> Result<bool> {
    let changed = tx
        .execute(
            "UPDATE account_group SET name = ?2 WHERE id = ?1",
            params![id.to_string(), name],
        )
        .map_err(map_unique)?;
    Ok(changed == 1)
}

/// Hapus group; akunnya pindah ke "tanpa grup".
pub fn delete_group(tx: &Transaction, id: Uuid) -> Result<bool> {
    let key = id.to_string();
    tx.execute(
        "UPDATE account SET group_id = NULL WHERE group_id = ?1",
        [&key],
    )?;
    Ok(tx.execute("DELETE FROM account_group WHERE id = ?1", [&key])? == 1)
}

pub fn set_group_order(tx: &Transaction, id: Uuid, sort_order: i64) -> Result<bool> {
    let changed = tx.execute(
        "UPDATE account_group SET sort_order = ?2 WHERE id = ?1",
        params![id.to_string(), sort_order],
    )?;
    Ok(changed == 1)
}

pub fn group_exists(conn: &Connection, id: Uuid) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM account_group WHERE id = ?1)",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

pub fn list_groups(conn: &Connection) -> Result<Vec<AccountGroupRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, name, sort_order FROM account_group ORDER BY sort_order, name",
    )?;
    let raw = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get(2)?))
        })?
        .collect::<rusqlite::Result<Vec<(String, String, i64)>>>()?;
    raw.into_iter()
        .map(|(id, name, sort_order)| {
            Ok(AccountGroupRow {
                id: parse_uuid("account_group", &id)?,
                name,
                sort_order,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::{NewAccount, insert_account, list_accounts};
    use crate::{StorageError, now_utc, open_in_memory};
    use nabungo_core::{AccountKind, CurrencyCode};

    fn names(conn: &Connection) -> Vec<(String, i64)> {
        list_groups(conn)
            .unwrap()
            .into_iter()
            .map(|g| (g.name, g.sort_order))
            .collect()
    }

    #[test]
    fn group_crud_and_order() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let h = insert_group(&tx, "Harian").unwrap();
        let t = insert_group(&tx, "Tabungan").unwrap();
        assert_eq!(
            names(&tx),
            [("Harian".to_owned(), 0), ("Tabungan".to_owned(), 1)]
        );
        assert!(rename_group(&tx, h, "Sehari-hari").unwrap());
        assert!(set_group_order(&tx, t, 0).unwrap());
        assert!(set_group_order(&tx, h, 1).unwrap());
        assert_eq!(
            names(&tx),
            [("Tabungan".to_owned(), 0), ("Sehari-hari".to_owned(), 1)]
        );
        assert!(group_exists(&tx, h).unwrap());
        assert!(!group_exists(&tx, Uuid::now_v7()).unwrap());
        assert!(!rename_group(&tx, Uuid::now_v7(), "X").unwrap());
    }

    #[test]
    fn group_names_unique_case_insensitive() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        insert_group(&tx, "Harian").unwrap();
        let t = insert_group(&tx, "Tabungan").unwrap();
        assert!(matches!(
            insert_group(&tx, "harian"),
            Err(StorageError::Duplicate)
        ));
        assert!(matches!(
            rename_group(&tx, t, "HARIAN"),
            Err(StorageError::Duplicate)
        ));
    }

    #[test]
    fn delete_group_ungroups_accounts() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let g = insert_group(&tx, "Harian").unwrap();
        let mut new = NewAccount::new("BCA", AccountKind::Bank, CurrencyCode::IDR);
        new.group_id = Some(g);
        let a = insert_account(&tx, &new, &now_utc()).unwrap();
        assert!(delete_group(&tx, g).unwrap());
        let acc = list_accounts(&tx).unwrap();
        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0].id, a);
        assert_eq!(acc[0].group_id, None);
        assert!(list_groups(&tx).unwrap().is_empty());
    }
}
