//! Aturan katalog: normalisasi nama, atribut akun, dan pohon kategori.

use uuid::Uuid;

use crate::{CategoryKind, MAX_AMOUNT};

/// Panjang maksimum nama akun, group, kategori, dan payee (dalam karakter).
pub const MAX_NAME_LEN: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CatalogError {
    #[error("name is empty")]
    EmptyName,
    #[error("name is longer than {MAX_NAME_LEN} characters")]
    NameTooLong,
    #[error("statement day {0} is outside 1..=31")]
    InvalidStatementDay(u8),
    #[error("credit limit is outside 0..=MAX_AMOUNT")]
    InvalidCreditLimit,
    #[error("categories may only be two levels deep")]
    CategoryDepth,
    #[error("category kind does not match")]
    CategoryKindMismatch,
    #[error("category still has children")]
    CategoryHasChildren,
    #[error("category cannot be merged into itself")]
    MergeIntoSelf,
}

/// Trim nama; tolak kosong atau lebih dari [`MAX_NAME_LEN`] karakter.
pub fn normalize_name(input: &str) -> Result<String, CatalogError> {
    let name = input.trim();
    if name.is_empty() {
        return Err(CatalogError::EmptyName);
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(CatalogError::NameTooLong);
    }
    Ok(name.to_owned())
}

pub fn validate_statement_day(day: Option<u8>) -> Result<Option<u8>, CatalogError> {
    match day {
        Some(d) if !(1..=31).contains(&d) => Err(CatalogError::InvalidStatementDay(d)),
        _ => Ok(day),
    }
}

pub fn validate_credit_limit(limit: Option<i64>) -> Result<Option<i64>, CatalogError> {
    match limit {
        Some(l) if !(0..=MAX_AMOUNT).contains(&l) => Err(CatalogError::InvalidCreditLimit),
        _ => Ok(limit),
    }
}

/// Ringkasan calon parent kategori.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentInfo {
    pub kind: CategoryKind,
    pub has_parent: bool,
}

/// Pohon kategori maksimal 2 level dan child sejenis dengan parent.
pub fn check_parent(
    kind: CategoryKind,
    parent: Option<ParentInfo>,
    has_children: bool,
) -> Result<(), CatalogError> {
    let Some(parent) = parent else {
        return Ok(());
    };
    if parent.has_parent {
        return Err(CatalogError::CategoryDepth);
    }
    if parent.kind != kind {
        return Err(CatalogError::CategoryKindMismatch);
    }
    if has_children {
        return Err(CatalogError::CategoryDepth);
    }
    Ok(())
}

/// Satu sisi operasi merge kategori.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeSide {
    pub id: Uuid,
    pub kind: CategoryKind,
    pub has_children: bool,
}

/// Syarat merge `source → target`.
pub fn check_merge(source: MergeSide, target: MergeSide) -> Result<(), CatalogError> {
    if source.id == target.id {
        return Err(CatalogError::MergeIntoSelf);
    }
    if source.kind != target.kind {
        return Err(CatalogError::CategoryKindMismatch);
    }
    if source.has_children {
        return Err(CatalogError::CategoryHasChildren);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_table() {
        assert_eq!(normalize_name("  BCA "), Ok("BCA".to_owned()));
        assert_eq!(normalize_name(""), Err(CatalogError::EmptyName));
        assert_eq!(normalize_name("   "), Err(CatalogError::EmptyName));
        assert!(normalize_name(&"a".repeat(80)).is_ok());
        assert_eq!(
            normalize_name(&"a".repeat(81)),
            Err(CatalogError::NameTooLong)
        );
        assert!(normalize_name(&"é".repeat(80)).is_ok());
    }

    #[test]
    fn statement_day_and_credit_limit_bounds() {
        for bad in [0, 32] {
            assert_eq!(
                validate_statement_day(Some(bad)),
                Err(CatalogError::InvalidStatementDay(bad))
            );
        }
        for ok in [Some(1), Some(31), None] {
            assert_eq!(validate_statement_day(ok), Ok(ok));
        }
        for bad in [-1, MAX_AMOUNT + 1] {
            assert_eq!(
                validate_credit_limit(Some(bad)),
                Err(CatalogError::InvalidCreditLimit)
            );
        }
        for ok in [Some(0), None] {
            assert_eq!(validate_credit_limit(ok), Ok(ok));
        }
    }

    #[test]
    fn check_parent_rules() {
        let root = |kind| ParentInfo {
            kind,
            has_parent: false,
        };
        let e = CategoryKind::Expense;
        assert_eq!(check_parent(e, None, true), Ok(()));
        assert_eq!(check_parent(e, Some(root(e)), false), Ok(()));
        let child = ParentInfo {
            kind: e,
            has_parent: true,
        };
        assert_eq!(
            check_parent(e, Some(child), false),
            Err(CatalogError::CategoryDepth)
        );
        assert_eq!(
            check_parent(e, Some(root(CategoryKind::Income)), false),
            Err(CatalogError::CategoryKindMismatch)
        );
        assert_eq!(
            check_parent(e, Some(root(e)), true),
            Err(CatalogError::CategoryDepth)
        );
    }

    #[test]
    fn check_merge_rules() {
        let side = |id, kind, has_children| MergeSide {
            id,
            kind,
            has_children,
        };
        let (a, b) = (Uuid::now_v7(), Uuid::now_v7());
        let e = CategoryKind::Expense;
        assert_eq!(
            check_merge(side(a, e, false), side(a, e, false)),
            Err(CatalogError::MergeIntoSelf)
        );
        assert_eq!(
            check_merge(side(a, e, false), side(b, CategoryKind::Income, false)),
            Err(CatalogError::CategoryKindMismatch)
        );
        assert_eq!(
            check_merge(side(a, e, true), side(b, e, false)),
            Err(CatalogError::CategoryHasChildren)
        );
        assert_eq!(check_merge(side(a, e, false), side(b, e, true)), Ok(()));
    }
}
