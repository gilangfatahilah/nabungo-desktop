use nabungo_core::{CategoryKind, CategoryRef};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Lookup, Result, StorageError, map_unique, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCategory {
    pub kind: CategoryKind,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
}

impl NewCategory {
    /// Kategori tanpa ikon/warna, `sort_order` 0.
    pub fn new(kind: CategoryKind, name: impl Into<String>, parent_id: Option<Uuid>) -> Self {
        Self {
            kind,
            name: name.into(),
            parent_id,
            icon: None,
            color: None,
            sort_order: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRow {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub kind: CategoryKind,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub archived: bool,
    /// Ditunjuk posting (termasuk transaksi terhapus), budget, atau default payee.
    pub in_use: bool,
}

pub fn insert_category(tx: &Transaction, new: &NewCategory) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO category (id, parent_id, kind, name, icon, color, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id.to_string(),
            new.parent_id.map(|p| p.to_string()),
            new.kind.as_str(),
            new.name,
            new.icon,
            new.color,
            new.sort_order
        ],
    )
    .map_err(map_unique)?;
    Ok(id)
}

/// `sort_order` berikutnya di antara saudara (root sejenis, atau child parent yang sama).
pub fn next_sort_order(conn: &Connection, kind: CategoryKind, parent: Option<Uuid>) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(sort_order) + 1, 0) FROM category
         WHERE kind = ?1 AND parent_id IS ?2",
        params![kind.as_str(), parent.map(|p| p.to_string())],
        |r| r.get(0),
    )?)
}

const ROW_SQL: &str = "SELECT c.id, c.parent_id, c.kind, c.name, c.icon, c.color, c.sort_order,
            c.archived_at IS NOT NULL,
            EXISTS (SELECT 1 FROM posting p WHERE p.category_id = c.id)
            OR EXISTS (SELECT 1 FROM budget b WHERE b.category_id = c.id)
            OR EXISTS (SELECT 1 FROM payee y WHERE y.default_category_id = c.id)
     FROM category c";

type RawCategory = (
    String,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<String>,
    i64,
    bool,
    bool,
);

fn raw(r: &rusqlite::Row) -> rusqlite::Result<RawCategory> {
    Ok((
        r.get(0)?,
        r.get(1)?,
        r.get(2)?,
        r.get(3)?,
        r.get(4)?,
        r.get(5)?,
        r.get(6)?,
        r.get(7)?,
        r.get(8)?,
    ))
}

fn to_row(raw: RawCategory) -> Result<CategoryRow> {
    let (id, parent_id, kind, name, icon, color, sort_order, archived, in_use) = raw;
    Ok(CategoryRow {
        kind: CategoryKind::parse(&kind).ok_or_else(|| {
            StorageError::Corrupt(format!("category {id}: unknown kind {kind:?}"))
        })?,
        id: parse_uuid("category", &id)?,
        parent_id: parent_id
            .as_deref()
            .map(|p| parse_uuid("category", p))
            .transpose()?,
        name,
        icon,
        color,
        sort_order,
        archived,
        in_use,
    })
}

pub fn list_categories(conn: &Connection) -> Result<Vec<CategoryRow>> {
    let mut stmt = conn.prepare_cached(&format!("{ROW_SQL} ORDER BY c.sort_order, c.name"))?;
    let rows = stmt
        .query_map([], raw)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter().map(to_row).collect()
}

pub fn get_category(conn: &Connection, id: Uuid) -> Result<Option<CategoryRow>> {
    let row = conn
        .query_row(&format!("{ROW_SQL} WHERE c.id = ?1"), [id.to_string()], raw)
        .optional()?;
    row.map(to_row).transpose()
}

pub fn update_category(
    tx: &Transaction,
    id: Uuid,
    name: &str,
    parent_id: Option<Uuid>,
    icon: Option<&str>,
    color: Option<&str>,
) -> Result<bool> {
    let changed = tx
        .execute(
            "UPDATE category SET name = ?2, parent_id = ?3, icon = ?4, color = ?5 WHERE id = ?1",
            params![
                id.to_string(),
                name,
                parent_id.map(|p| p.to_string()),
                icon,
                color
            ],
        )
        .map_err(map_unique)?;
    Ok(changed == 1)
}

/// Arsip ikut mengarsip child; pulihkan hanya kategori itu sendiri.
pub fn set_category_archived(
    tx: &Transaction,
    id: Uuid,
    archived: bool,
    now: &str,
) -> Result<bool> {
    let key = id.to_string();
    if archived {
        tx.execute(
            "UPDATE category SET archived_at = ?2 WHERE parent_id = ?1 AND archived_at IS NULL",
            params![key, now],
        )?;
    }
    let changed = tx.execute(
        "UPDATE category SET archived_at = CASE WHEN ?2 THEN ?3 END WHERE id = ?1",
        params![key, archived, now],
    )?;
    Ok(changed == 1)
}

/// Ditunjuk posting (termasuk transaksi terhapus), budget, atau default payee.
pub fn is_referenced(conn: &Connection, id: Uuid) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM posting WHERE category_id = ?1)
             OR EXISTS (SELECT 1 FROM budget WHERE category_id = ?1)
             OR EXISTS (SELECT 1 FROM payee WHERE default_category_id = ?1)",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

pub fn child_count(conn: &Connection, id: Uuid) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM category WHERE parent_id = ?1",
        [id.to_string()],
        |r| r.get(0),
    )?)
}

/// Pindahkan posting (termasuk transaksi terhapus), default payee, dan budget dari
/// `source` ke `target`, lalu hapus `source`. Budget di periode yang sama dijumlahkan.
pub fn merge_category(tx: &Transaction, source: Uuid, target: Uuid) -> Result<()> {
    let (s, t) = (source.to_string(), target.to_string());
    tx.execute(
        "UPDATE posting SET category_id = ?2 WHERE category_id = ?1",
        [&s, &t],
    )?;
    tx.execute(
        "UPDATE payee SET default_category_id = ?2 WHERE default_category_id = ?1",
        [&s, &t],
    )?;
    // Periode yang ada di kedua kategori: jumlahkan ke target, buang baris source.
    tx.execute(
        "UPDATE budget SET amount = amount + (SELECT src.amount FROM budget src
                                              WHERE src.category_id = ?1
                                                AND src.period = budget.period)
         WHERE category_id = ?2
           AND period IN (SELECT period FROM budget WHERE category_id = ?1)",
        [&s, &t],
    )?;
    tx.execute(
        "DELETE FROM budget WHERE category_id = ?1
           AND period IN (SELECT period FROM budget WHERE category_id = ?2)",
        [&s, &t],
    )?;
    tx.execute(
        "UPDATE budget SET category_id = ?2 WHERE category_id = ?1",
        [&s, &t],
    )?;
    tx.execute("DELETE FROM category WHERE id = ?1", [&s])?;
    Ok(())
}

pub fn delete_category(tx: &Transaction, id: Uuid) -> Result<bool> {
    Ok(tx.execute("DELETE FROM category WHERE id = ?1", [id.to_string()])? == 1)
}

pub fn get_category_ref(conn: &Connection, id: Uuid) -> Result<Option<Lookup<CategoryRef>>> {
    let row = conn
        .query_row(
            "SELECT kind, archived_at IS NOT NULL FROM category WHERE id = ?1",
            [id.to_string()],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, bool>(1)?)),
        )
        .optional()?;
    let Some((kind, archived)) = row else {
        return Ok(None);
    };
    let kind = CategoryKind::parse(&kind)
        .ok_or_else(|| StorageError::Corrupt(format!("category {id}: unknown kind {kind:?}")))?;
    Ok(Some(Lookup {
        value: CategoryRef { id, kind },
        archived,
    }))
}
