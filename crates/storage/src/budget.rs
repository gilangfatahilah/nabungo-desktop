//! Repository tabel `budget` (satu baris per kategori × periode budget).

use rusqlite::{Connection, Transaction, params};
use uuid::Uuid;

use crate::{Result, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetRow {
    pub category_id: Uuid,
    pub period: String,
    pub amount: i64,
}

/// Simpan budget; menimpa nominal bila kategori × periode sudah ada.
pub fn upsert_budget(tx: &Transaction, category_id: Uuid, period: &str, amount: i64) -> Result<()> {
    tx.execute(
        "INSERT INTO budget (id, category_id, period, amount) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (category_id, period) DO UPDATE SET amount = excluded.amount",
        params![
            Uuid::now_v7().to_string(),
            category_id.to_string(),
            period,
            amount
        ],
    )?;
    Ok(())
}

/// `false` bila tidak ada budget untuk kategori × periode itu.
pub fn delete_budget(tx: &Transaction, category_id: Uuid, period: &str) -> Result<bool> {
    let n = tx.execute(
        "DELETE FROM budget WHERE category_id = ?1 AND period = ?2",
        params![category_id.to_string(), period],
    )?;
    Ok(n == 1)
}

pub fn list_budgets(conn: &Connection, period: &str) -> Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT category_id, period, amount FROM budget WHERE period = ?1 ORDER BY category_id",
    )?;
    collect_rows(stmt.query_map([period], |r| {
        Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?))
    })?)
}

/// Semua budget (semua periode), urut periode lalu kategori — untuk ekspor.
pub fn list_all_budgets(conn: &Connection) -> Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare_cached(
        "SELECT category_id, period, amount FROM budget ORDER BY period, category_id",
    )?;
    collect_rows(stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?)))?)
}

fn collect_rows(
    rows: impl Iterator<Item = rusqlite::Result<(String, String, i64)>>,
) -> Result<Vec<BudgetRow>> {
    rows.map(|row| {
        let (id, period, amount) = row?;
        Ok(BudgetRow {
            category_id: parse_uuid("category", &id)?,
            period,
            amount,
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::{NewCategory, insert_category};
    use crate::open_in_memory;
    use nabungo_core::CategoryKind;

    #[test]
    fn upsert_replaces_amount_and_delete_removes() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let cat =
            insert_category(&tx, &NewCategory::new(CategoryKind::Expense, "Makan", None)).unwrap();
        upsert_budget(&tx, cat, "2026-10", 100).unwrap();
        upsert_budget(&tx, cat, "2026-10", 200).unwrap();
        upsert_budget(&tx, cat, "2026-11", 300).unwrap();
        assert_eq!(
            list_budgets(&tx, "2026-10").unwrap(),
            vec![BudgetRow {
                category_id: cat,
                period: "2026-10".into(),
                amount: 200
            }]
        );
        assert!(delete_budget(&tx, cat, "2026-10").unwrap());
        assert!(list_budgets(&tx, "2026-10").unwrap().is_empty());
        assert!(!delete_budget(&tx, cat, "2026-10").unwrap());
        assert_eq!(list_budgets(&tx, "2026-11").unwrap().len(), 1);
    }
}
