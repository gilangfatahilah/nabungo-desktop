//! Vault: membungkus key SQLCipher (DEK) dengan password (Argon2id + XChaCha20-Poly1305) di
//! file `nabungo.key`, dan recovery key = DEK dalam Crockford base32.

use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use nabungo_storage::DbKey;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::AppError;

const AAD: &[u8] = b"nabungo-keyfile-v1";
const VERSION: u32 = 1;
const KDF: &str = "argon2id";
const MIN_PASSWORD_CHARS: usize = 8;

/// Parameter Argon2id (memori KiB, iterasi, paralelisme).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl KdfParams {
    pub const DEFAULT: Self = Self {
        m_kib: 65536,
        t: 3,
        p: 1,
    };
    /// Murah, khusus test.
    pub const FAST: Self = Self {
        m_kib: 8,
        t: 1,
        p: 1,
    };
}

/// Isi `nabungo.key`: DEK terbungkus KEK hasil Argon2id dari password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    v: u32,
    kdf: String,
    m_kib: u32,
    t: u32,
    p: u32,
    salt: String,
    nonce: String,
    wrapped: String,
}

/// `nabungo.db` → `nabungo.key`.
pub fn keyfile_path(db: &Path) -> PathBuf {
    db.with_extension("key")
}

/// Password minimal 8 karakter (Unicode scalar), tanpa normalisasi.
pub fn validate_password(pw: &str) -> Result<(), AppError> {
    if pw.chars().count() < MIN_PASSWORD_CHARS {
        return Err(AppError::PasswordTooShort);
    }
    Ok(())
}

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::fill(&mut b).expect("RNG sistem operasi tidak tersedia");
    b
}

fn derive_kek(
    password: &str,
    salt: &[u8],
    params: KdfParams,
) -> Result<Zeroizing<[u8; 32]>, AppError> {
    let p = Params::new(params.m_kib, params.t, params.p, Some(32))
        .map_err(|_| AppError::KeyFileInvalid)?;
    let mut kek = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, p)
        .hash_password_into(password.as_bytes(), salt, kek.as_mut())
        .map_err(|_| AppError::KeyFileInvalid)?;
    Ok(kek)
}

fn cipher(kek: &[u8; 32]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(&Key::from(*kek))
}

fn b64_array<const N: usize>(s: &str) -> Result<[u8; N], AppError> {
    B64.decode(s)
        .ok()
        .and_then(|v| <[u8; N]>::try_from(v).ok())
        .ok_or(AppError::KeyFileInvalid)
}

impl KeyFile {
    /// Bungkus `dek` dengan password; salt & nonce acak.
    pub fn seal(dek: &DbKey, password: &str, params: KdfParams) -> Result<Self, AppError> {
        let salt: [u8; 16] = random();
        let nonce: [u8; 24] = random();
        let kek = derive_kek(password, &salt, params)?;
        let wrapped = cipher(&kek)
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: dek.as_bytes(),
                    aad: AAD,
                },
            )
            .map_err(|_| AppError::KeyFileInvalid)?;
        Ok(Self {
            v: VERSION,
            kdf: KDF.to_owned(),
            m_kib: params.m_kib,
            t: params.t,
            p: params.p,
            salt: B64.encode(salt),
            nonce: B64.encode(nonce),
            wrapped: B64.encode(wrapped),
        })
    }

    /// Buka DEK dengan password; params KDF diambil dari file.
    pub fn unseal(&self, password: &str) -> Result<DbKey, AppError> {
        if self.v != VERSION || self.kdf != KDF {
            return Err(AppError::KeyFileInvalid);
        }
        let salt: [u8; 16] = b64_array(&self.salt)?;
        let nonce: [u8; 24] = b64_array(&self.nonce)?;
        let wrapped = B64
            .decode(&self.wrapped)
            .map_err(|_| AppError::KeyFileInvalid)?;
        let params = KdfParams {
            m_kib: self.m_kib,
            t: self.t,
            p: self.p,
        };
        let kek = derive_kek(password, &salt, params)?;
        let dek = Zeroizing::new(
            cipher(&kek)
                .decrypt(
                    &XNonce::from(nonce),
                    Payload {
                        msg: &wrapped,
                        aad: AAD,
                    },
                )
                .map_err(|_| AppError::WrongPassword)?,
        );
        let bytes: [u8; 32] = dek
            .as_slice()
            .try_into()
            .map_err(|_| AppError::KeyFileInvalid)?;
        Ok(DbKey::from_bytes(bytes))
    }

    pub fn read(path: &Path) -> Result<Self, AppError> {
        let raw = std::fs::read(path)?;
        serde_json::from_slice(&raw).map_err(|_| AppError::KeyFileInvalid)
    }

    /// Tulis ke `<path>.partial`, `sync_all`, lalu rename.
    pub fn write_atomic(&self, path: &Path) -> Result<(), AppError> {
        use std::io::Write as _;
        let mut partial = path.as_os_str().to_owned();
        partial.push(".partial");
        let partial = PathBuf::from(partial);
        let result = (|| -> std::io::Result<()> {
            let mut f = std::fs::File::create(&partial)?;
            f.write_all(&serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&partial, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&partial);
        }
        Ok(result?)
    }
}

/// DEK → 52 karakter Crockford base32, 13 kelompok 4 karakter dipisah `-`.
pub fn format_recovery_key(dek: &DbKey) -> String {
    let raw = Zeroizing::new(base32::encode(base32::Alphabet::Crockford, dek.as_bytes()));
    raw.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).expect("base32 ASCII"))
        .collect::<Vec<_>>()
        .join("-")
}

/// Kebalikan [`format_recovery_key`]; toleran terhadap spasi, `-`, huruf kecil, dan O/I/L.
pub fn parse_recovery_key(s: &str) -> Result<DbKey, AppError> {
    let normalized: Zeroizing<String> = Zeroizing::new(
        s.chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| match c.to_ascii_uppercase() {
                'O' => '0',
                'I' | 'L' => '1',
                c => c,
            })
            .collect(),
    );
    let bytes = Zeroizing::new(
        base32::decode(base32::Alphabet::Crockford, &normalized)
            .ok_or(AppError::InvalidRecoveryKey)?,
    );
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| AppError::InvalidRecoveryKey)?;
    Ok(DbKey::from_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dek() -> DbKey {
        DbKey::from_bytes(std::array::from_fn(|i| i as u8))
    }

    #[test]
    fn seal_unseal_roundtrip() {
        let kf = KeyFile::seal(&dek(), "rahasia-123", KdfParams::FAST).unwrap();
        assert_eq!(kf.unseal("rahasia-123").unwrap(), dek());
    }

    #[test]
    fn wrong_password_rejected() {
        let kf = KeyFile::seal(&dek(), "rahasia-123", KdfParams::FAST).unwrap();
        assert!(matches!(
            kf.unseal("salah-1234"),
            Err(AppError::WrongPassword)
        ));
    }

    #[test]
    fn password_unicode_roundtrip() {
        let kf = KeyFile::seal(&dek(), "páss wörd 🔑 ", KdfParams::FAST).unwrap();
        assert_eq!(kf.unseal("páss wörd 🔑 ").unwrap(), dek());
        assert!(matches!(
            kf.unseal("páss wörd 🔑"),
            Err(AppError::WrongPassword)
        ));
    }

    #[test]
    fn keyfile_json_roundtrip_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let path = keyfile_path(&dir.path().join("nabungo.db"));
        assert_eq!(path.file_name().unwrap(), "nabungo.key");
        KeyFile::seal(&dek(), "rahasia-123", KdfParams::FAST)
            .unwrap()
            .write_atomic(&path)
            .unwrap();
        assert_eq!(
            KeyFile::read(&path).unwrap().unseal("rahasia-123").unwrap(),
            dek()
        );
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec!["nabungo.key".to_owned()]);
    }

    #[test]
    fn keyfile_bad_version_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nabungo.key");
        KeyFile::seal(&dek(), "rahasia-123", KdfParams::FAST)
            .unwrap()
            .write_atomic(&path)
            .unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["v"] = 2.into();
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(matches!(
            KeyFile::read(&path).unwrap().unseal("rahasia-123"),
            Err(AppError::KeyFileInvalid)
        ));
        std::fs::write(&path, b"{bukan json").unwrap();
        assert!(matches!(
            KeyFile::read(&path),
            Err(AppError::KeyFileInvalid)
        ));
    }

    #[test]
    fn password_min_length() {
        assert!(matches!(
            validate_password("1234567"),
            Err(AppError::PasswordTooShort)
        ));
        validate_password("12345678").unwrap();
        assert!(matches!(
            validate_password("ééééééé"),
            Err(AppError::PasswordTooShort)
        ));
    }

    #[test]
    fn recovery_key_format() {
        let s = format_recovery_key(&dek());
        assert_eq!(s.len(), 64);
        let groups: Vec<_> = s.split('-').collect();
        assert_eq!(groups.len(), 13);
        assert!(groups.iter().all(|g| g.len() == 4));
    }

    #[test]
    fn recovery_key_roundtrip() {
        let k = DbKey::generate();
        assert_eq!(parse_recovery_key(&format_recovery_key(&k)).unwrap(), k);
    }

    #[test]
    fn recovery_key_normalizes_typos() {
        let k = DbKey::from_bytes([0; 32]); // semua '0' → uji O → 0
        let s = format_recovery_key(&k);
        let typo = s.replace('0', "O").to_lowercase().replace('-', " ");
        assert_eq!(parse_recovery_key(&typo).unwrap(), k);
        let k1 = DbKey::from_bytes([0x08; 32]); // mengandung '1'
        let s1 = format_recovery_key(&k1);
        assert!(s1.contains('1'));
        assert_eq!(parse_recovery_key(&s1.replace('1', "l")).unwrap(), k1);
        assert_eq!(parse_recovery_key(&s1.replace('1', "I")).unwrap(), k1);
    }

    #[test]
    fn recovery_key_wrong_length_rejected() {
        let s = format_recovery_key(&dek());
        for bad in [&s[..s.len() - 5], &format!("{s}ABCD"), "", "U!!!"] {
            assert!(
                matches!(parse_recovery_key(bad), Err(AppError::InvalidRecoveryKey)),
                "{bad:?}"
            );
        }
    }
}
