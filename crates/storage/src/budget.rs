//! Repository tabel `budget` (satu baris per kategori × periode budget).

use rusqlite::{Connection, Transaction, params};
use uuid::Uuid;

use crate::{Result, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetRow {
    pub category_id: Uuid,
    pub period: String,
    pub amount: i64,
    /// Sisa periode ini dibawa ke periode berikutnya (BUD-03).
    pub rollover: bool,
}

/// Simpan budget; menimpa nominal bila kategori × periode sudah ada (flag
/// `rollover` dipertahankan; baris baru tanpa rollover).
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

/// `false` bila tidak ada budget untuk kategori × periode itu.
pub fn set_rollover(tx: &Transaction, category_id: Uuid, period: &str, on: bool) -> Result<bool> {
    let n = tx.execute(
        "UPDATE budget SET rollover = ?3 WHERE category_id = ?1 AND period = ?2",
        params![category_id.to_string(), period, on],
    )?;
    Ok(n == 1)
}

const COLUMNS: &str = "SELECT category_id, period, amount, rollover FROM budget";

pub fn list_budgets(conn: &Connection, period: &str) -> Result<Vec<BudgetRow>> {
    let mut stmt =
        conn.prepare_cached(&format!("{COLUMNS} WHERE period = ?1 ORDER BY category_id"))?;
    collect_rows(stmt.query_map([period], read_row)?)
}

/// Semua budget sebelum `period` (kunci `YYYY-MM` urut leksikal), urut kategori
/// lalu periode — untuk rantai rollover.
pub fn list_budgets_before(conn: &Connection, period: &str) -> Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare_cached(&format!(
        "{COLUMNS} WHERE period < ?1 ORDER BY category_id, period"
    ))?;
    collect_rows(stmt.query_map([period], read_row)?)
}

/// Semua budget (semua periode), urut periode lalu kategori — untuk ekspor.
pub fn list_all_budgets(conn: &Connection) -> Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare_cached(&format!("{COLUMNS} ORDER BY period, category_id"))?;
    collect_rows(stmt.query_map([], read_row)?)
}

type RawRow = (String, String, i64, bool);

fn read_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RawRow> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
}

fn collect_rows(rows: impl Iterator<Item = rusqlite::Result<RawRow>>) -> Result<Vec<BudgetRow>> {
    rows.map(|row| {
        let (id, period, amount, rollover) = row?;
        Ok(BudgetRow {
            category_id: parse_uuid("category", &id)?,
            period,
            amount,
            rollover,
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
                amount: 200,
                rollover: false,
            }]
        );
        assert!(delete_budget(&tx, cat, "2026-10").unwrap());
        assert!(list_budgets(&tx, "2026-10").unwrap().is_empty());
        assert!(!delete_budget(&tx, cat, "2026-10").unwrap());
        assert_eq!(list_budgets(&tx, "2026-11").unwrap().len(), 1);
    }

    #[test]
    fn rollover_flag_survives_amount_update_and_list_before() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let cat =
            insert_category(&tx, &NewCategory::new(CategoryKind::Expense, "Makan", None)).unwrap();
        upsert_budget(&tx, cat, "2026-10", 100).unwrap();
        assert!(set_rollover(&tx, cat, "2026-10", true).unwrap());
        upsert_budget(&tx, cat, "2026-10", 200).unwrap();
        let rows = list_budgets(&tx, "2026-10").unwrap();
        assert_eq!((rows[0].amount, rows[0].rollover), (200, true));
        assert!(!set_rollover(&tx, cat, "2026-11", true).unwrap());

        upsert_budget(&tx, cat, "2026-11", 50).unwrap();
        let before = list_budgets_before(&tx, "2026-11").unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].period, "2026-10");
        assert!(list_all_budgets(&tx).unwrap()[0].rollover);

        assert!(delete_budget(&tx, cat, "2026-10").unwrap());
        upsert_budget(&tx, cat, "2026-10", 100).unwrap();
        assert!(!list_budgets(&tx, "2026-10").unwrap()[0].rollover);
    }
}
