use nabungo_core::{CategoryKind, CategoryRef};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Lookup, Result, StorageError};

pub fn insert_category(
    tx: &Transaction,
    kind: CategoryKind,
    name: &str,
    parent: Option<Uuid>,
) -> Result<Uuid> {
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO category (id, parent_id, kind, name) VALUES (?1, ?2, ?3, ?4)",
        params![
            id.to_string(),
            parent.map(|p| p.to_string()),
            kind.as_str(),
            name
        ],
    )?;
    Ok(id)
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
