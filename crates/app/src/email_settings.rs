//! Setting akun email capture (EML-01). Hanya alamat, label, dan status; App Password ada di
//! OS keychain (`crate::secret`), tidak pernah di tabel `setting`.

use nabungo_storage::setting;
use time::OffsetDateTime;

use crate::{AppError, Ledger};

pub const DEFAULT_MAILBOX: &str = "Finance";

const KEY_ENABLED: &str = "email.enabled";
const KEY_ADDRESS: &str = "email.address";
const KEY_MAILBOX: &str = "email.mailbox";
const KEY_LAST_SYNC_AT: &str = "email.last_sync_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailSettings {
    pub enabled: bool,
    pub address: Option<String>,
    pub mailbox: String,
    /// ISO-8601 UTC putaran sync terakhir yang sukses.
    pub last_sync_at: Option<String>,
}

/// Trim + lowercase; tepat satu `@` dengan bagian kiri & kanan tidak kosong, tanpa whitespace,
/// ≤ 254 karakter.
pub fn normalize_address(input: &str) -> Result<String, AppError> {
    let address = input.trim().to_lowercase();
    let valid = address.len() <= 254
        && !address.chars().any(char::is_whitespace)
        && matches!(address.split_once('@'), Some((local, domain))
            if !local.is_empty() && !domain.is_empty() && !domain.contains('@'));
    if valid {
        Ok(address)
    } else {
        Err(AppError::EmailAddressInvalid)
    }
}

/// Trim; kosong → `EmailMailboxEmpty`. Label bersarang (`Finance/Bank`) diterima apa adanya.
pub fn normalize_mailbox(input: &str) -> Result<String, AppError> {
    let mailbox = input.trim();
    if mailbox.is_empty() {
        return Err(AppError::EmailMailboxEmpty);
    }
    Ok(mailbox.to_owned())
}

impl Ledger {
    pub fn email_settings(&self) -> Result<EmailSettings, AppError> {
        Ok(EmailSettings {
            enabled: setting::get_setting(&self.conn, KEY_ENABLED)?.as_deref() == Some("1"),
            address: setting::get_setting(&self.conn, KEY_ADDRESS)?,
            mailbox: setting::get_setting(&self.conn, KEY_MAILBOX)?
                .unwrap_or_else(|| DEFAULT_MAILBOX.to_owned()),
            last_sync_at: setting::get_setting(&self.conn, KEY_LAST_SYNC_AT)?,
        })
    }

    /// Menyimpan akun dan mengaktifkan sync; input sudah dinormalisasi.
    pub fn save_email_account(&mut self, address: &str, mailbox: &str) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_ADDRESS, address)?;
        setting::set_setting(&tx, KEY_MAILBOX, mailbox)?;
        setting::set_setting(&tx, KEY_ENABLED, "1")?;
        tx.commit()?;
        Ok(())
    }

    /// `email.enabled = 0`; alamat, label, cursor, dan capture tetap.
    pub fn disable_email(&mut self) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_ENABLED, "0")?;
        tx.commit()?;
        Ok(())
    }

    pub fn mark_email_synced(&mut self, at: OffsetDateTime) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_LAST_SYNC_AT, &crate::format_timestamp(at))?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn defaults_when_unset() {
        let ledger = Ledger::open_in_memory().unwrap();
        assert_eq!(
            ledger.email_settings().unwrap(),
            EmailSettings {
                enabled: false,
                address: None,
                mailbox: "Finance".into(),
                last_sync_at: None,
            }
        );
    }

    #[test]
    fn save_then_read() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger.save_email_account("me@gmail.com", "Bank").unwrap();
        let s = ledger.email_settings().unwrap();
        assert!(s.enabled);
        assert_eq!(s.address.as_deref(), Some("me@gmail.com"));
        assert_eq!(s.mailbox, "Bank");
    }

    #[test]
    fn disable_keeps_address_and_mailbox() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger.save_email_account("me@gmail.com", "Bank").unwrap();
        ledger.disable_email().unwrap();
        let s = ledger.email_settings().unwrap();
        assert!(!s.enabled);
        assert_eq!(s.address.as_deref(), Some("me@gmail.com"));
        assert_eq!(s.mailbox, "Bank");
    }

    #[test]
    fn normalize_address_cases() {
        assert_eq!(normalize_address(" Me@Gmail.COM ").unwrap(), "me@gmail.com");
        for bad in ["", "me", "a@@b", "@b", "a@", "a b@c"] {
            assert!(
                matches!(normalize_address(bad), Err(AppError::EmailAddressInvalid)),
                "{bad:?}"
            );
        }
        let long = format!("{}@b.c", "a".repeat(251));
        assert!(matches!(
            normalize_address(&long),
            Err(AppError::EmailAddressInvalid)
        ));
    }

    #[test]
    fn normalize_mailbox_trims_and_rejects_empty() {
        assert_eq!(normalize_mailbox("  Finance ").unwrap(), "Finance");
        assert_eq!(normalize_mailbox("Finance/Bank").unwrap(), "Finance/Bank");
        assert!(matches!(
            normalize_mailbox("   "),
            Err(AppError::EmailMailboxEmpty)
        ));
    }

    #[test]
    fn mark_synced_sets_timestamp() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let at = datetime!(2026-10-28 01:02:03 UTC);
        ledger.mark_email_synced(at).unwrap();
        assert_eq!(
            ledger.email_settings().unwrap().last_sync_at,
            Some(crate::format_timestamp(at))
        );
    }
}
