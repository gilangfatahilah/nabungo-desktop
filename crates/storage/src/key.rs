//! Key SQLCipher: 32 byte acak yang dipasang sebagai raw key (tanpa KDF SQLCipher).

use std::fmt::Write as _;

use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct DbKey([u8; 32]);

impl DbKey {
    /// Key baru dari RNG sistem operasi.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("RNG sistem operasi tidak tersedia");
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Pernyataan `PRAGMA key`/`ATTACH ... KEY` berbentuk raw key: `"x'<64 hex>'"`.
    pub(crate) fn sql_literal(&self) -> Zeroizing<String> {
        let mut s = Zeroizing::new(String::with_capacity(68));
        s.push_str("\"x'");
        for b in self.0 {
            let _ = write!(s, "{b:02x}");
        }
        s.push_str("'\"");
        s
    }
}

impl std::fmt::Debug for DbKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DbKey(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_leak() {
        assert_eq!(format!("{:?}", DbKey::from_bytes([7; 32])), "DbKey(..)");
    }

    #[test]
    fn sql_literal_is_raw_hex() {
        let lit = DbKey::from_bytes([0xab; 32]).sql_literal();
        assert_eq!(lit.as_str(), format!("\"x'{}'\"", "ab".repeat(32)));
    }

    #[test]
    fn generate_is_random() {
        assert_ne!(DbKey::generate(), DbKey::generate());
    }
}
