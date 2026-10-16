use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::Result;

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM setting WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set_setting(tx: &Transaction, key: &str, value: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO setting (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_in_memory;

    #[test]
    fn setting_upsert_roundtrip() {
        let mut conn = open_in_memory().unwrap();
        assert_eq!(get_setting(&conn, "ui.theme").unwrap(), None);
        let tx = conn.transaction().unwrap();
        set_setting(&tx, "ui.theme", "dark").unwrap();
        set_setting(&tx, "ui.theme", "light").unwrap();
        tx.commit().unwrap();
        assert_eq!(
            get_setting(&conn, "ui.theme").unwrap().as_deref(),
            Some("light")
        );
    }
}
