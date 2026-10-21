//! Repository tabel `contact` (orang di akun piutang/hutang, D10).

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use uuid::Uuid;

use crate::{Result, parse_uuid};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRow {
    pub id: Uuid,
    pub name: String,
}

/// ID kontak bernama `name` (tanpa membedakan huruf); dibuat bila belum ada.
/// `name` sudah dinormalisasi pemanggil.
pub fn ensure_contact(tx: &Transaction, name: &str, now: &str) -> Result<Uuid> {
    let found: Option<String> = tx
        .query_row("SELECT id FROM contact WHERE name = ?1", [name], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(id) = found {
        return parse_uuid("contact", &id);
    }
    let id = Uuid::now_v7();
    tx.execute(
        "INSERT INTO contact (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![id.to_string(), name, now],
    )?;
    Ok(id)
}

/// Semua kontak, urut nama tanpa membedakan huruf.
pub fn list_contacts(conn: &Connection) -> Result<Vec<ContactRow>> {
    let mut stmt = conn.prepare_cached("SELECT id, name FROM contact ORDER BY name, id")?;
    let raw = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter()
        .map(|(id, name)| {
            Ok(ContactRow {
                id: parse_uuid("contact", &id)?,
                name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{now_utc, open_in_memory};

    #[test]
    fn ensure_contact_is_idempotent_nocase() {
        let mut conn = open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let now = now_utc();
        let a = ensure_contact(&tx, "Budi", &now).unwrap();
        assert_eq!(ensure_contact(&tx, "Budi", &now).unwrap(), a);
        assert_eq!(ensure_contact(&tx, "BUDI", &now).unwrap(), a);
        let b = ensure_contact(&tx, "ani", &now).unwrap();
        assert_eq!(
            list_contacts(&tx).unwrap(),
            vec![
                ContactRow {
                    id: b,
                    name: "ani".into()
                },
                ContactRow {
                    id: a,
                    name: "Budi".into()
                },
            ]
        );
    }
}
