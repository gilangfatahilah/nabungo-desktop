//! Penyimpanan App Password email di OS keychain (EML-01, PRD §8). Credential tidak pernah
//! ditulis ke DB, setting, file, atau log.

use crate::AppError;

/// Nama service entry keychain; `account` = alamat email (lowercase).
pub const SERVICE: &str = "nabungo.email";

pub trait SecretStore: Send + Sync {
    /// Entry tidak ada → `Ok(None)`.
    fn get(&self, account: &str) -> Result<Option<String>, AppError>;
    fn set(&self, account: &str, secret: &str) -> Result<(), AppError>;
    /// Entry tidak ada → `Ok(())`.
    fn delete(&self, account: &str) -> Result<(), AppError>;
}

/// OS keychain: Secret Service (Linux, lewat libdbus) atau Credential Manager (Windows).
#[derive(Debug)]
pub struct KeyringStore(());

impl KeyringStore {
    /// Memasang store OS sebagai default `keyring_core` sekali per proses. Hanya dipanggil
    /// saat email dikonfigurasi/di-sync, tidak saat startup (koneksi D-Bus tidak dibuka).
    pub fn new() -> Result<Self, AppError> {
        static INIT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let ok = *INIT.get_or_init(|| match native_store() {
            Ok(store) => {
                keyring_core::set_default_store(store);
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "keychain tidak tersedia");
                false
            }
        });
        if ok {
            Ok(Self(()))
        } else {
            Err(AppError::KeychainUnavailable)
        }
    }

    fn entry(&self, account: &str) -> Result<keyring_core::Entry, AppError> {
        keyring_core::Entry::new(SERVICE, account).map_err(keychain_error)
    }
}

#[cfg(target_os = "linux")]
fn native_store() -> keyring_core::Result<std::sync::Arc<keyring_core::CredentialStore>> {
    Ok(dbus_secret_service_keyring_store::Store::new()?)
}

#[cfg(windows)]
fn native_store() -> keyring_core::Result<std::sync::Arc<keyring_core::CredentialStore>> {
    Ok(windows_native_keyring_store::Store::new()?)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn native_store() -> keyring_core::Result<std::sync::Arc<keyring_core::CredentialStore>> {
    Err(keyring_core::Error::NotSupportedByStore(
        "platform tanpa keychain bawaan".into(),
    ))
}

/// Pesan error store tidak memuat secret; yang dicatat hanya jenisnya.
fn keychain_error(e: keyring_core::Error) -> AppError {
    tracing::warn!(error = %e, "operasi keychain gagal");
    AppError::KeychainUnavailable
}

impl SecretStore for KeyringStore {
    fn get(&self, account: &str) -> Result<Option<String>, AppError> {
        match self.entry(account)?.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(keychain_error(e)),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), AppError> {
        self.entry(account)?
            .set_password(secret)
            .map_err(keychain_error)
    }

    fn delete(&self, account: &str) -> Result<(), AppError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(keychain_error(e)),
        }
    }
}

/// Store in-memory untuk test; `failing()` meniru keychain yang tidak tersedia.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    entries: std::sync::Mutex<std::collections::HashMap<String, String>>,
    failing: bool,
}

impl MemorySecretStore {
    pub fn failing() -> Self {
        Self {
            failing: true,
            ..Self::default()
        }
    }
}

impl MemorySecretStore {
    fn entries(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, std::collections::HashMap<String, String>>, AppError>
    {
        if self.failing {
            return Err(AppError::KeychainUnavailable);
        }
        Ok(self.entries.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

impl SecretStore for MemorySecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, AppError> {
        Ok(self.entries()?.get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), AppError> {
        self.entries()?
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), AppError> {
        self.entries()?.remove(account);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_missing_is_none() {
        let s = MemorySecretStore::default();
        assert_eq!(s.get("me@gmail.com").unwrap(), None);
    }

    #[test]
    fn set_get_delete_roundtrip() {
        let s = MemorySecretStore::default();
        s.set("me@gmail.com", "rahasia").unwrap();
        assert_eq!(s.get("me@gmail.com").unwrap().as_deref(), Some("rahasia"));
        assert_eq!(s.get("lain@gmail.com").unwrap(), None);
        s.delete("me@gmail.com").unwrap();
        assert_eq!(s.get("me@gmail.com").unwrap(), None);
    }

    #[test]
    fn delete_missing_is_ok() {
        MemorySecretStore::default().delete("me@gmail.com").unwrap();
    }

    #[test]
    fn failing_store_reports_keychain_unavailable() {
        let s = MemorySecretStore::failing();
        assert!(matches!(s.get("a@b"), Err(AppError::KeychainUnavailable)));
        assert!(matches!(
            s.set("a@b", "x"),
            Err(AppError::KeychainUnavailable)
        ));
        assert!(matches!(
            s.delete("a@b"),
            Err(AppError::KeychainUnavailable)
        ));
    }

    /// Keychain sungguhan; jalankan manual: `cargo test -p nabungo-app secret::keyring_roundtrip_real -- --ignored`.
    #[test]
    #[ignore = "butuh keychain OS (Secret Service / Credential Manager)"]
    fn keyring_roundtrip_real() {
        let s = KeyringStore::new().unwrap();
        let account = "nabungo.test@example.invalid";
        s.delete(account).unwrap();
        assert_eq!(s.get(account).unwrap(), None);
        s.set(account, "rahasia-uji").unwrap();
        assert_eq!(s.get(account).unwrap().as_deref(), Some("rahasia-uji"));
        s.delete(account).unwrap();
        assert_eq!(s.get(account).unwrap(), None);
    }
}
