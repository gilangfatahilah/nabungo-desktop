//! Alokasi modal: bucket bertingkat dengan target, mapping akun → bucket, dan porsi aktual
//! yang diturunkan dari saldo (PRD §4.5, CAP-01..03). Persentase dalam basis point.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::{AccountKind, MoneyError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AllocationError {
    #[error("basis points must be within 0..=10000")]
    InvalidBasisPoints,
    #[error("sibling targets total {total} bp, more than 10000")]
    TargetsExceed { total: i64 },
    #[error("account shares total {total} bp, more than 10000")]
    SharesExceed { total: i64 },
    #[error("buckets can only be nested one level deep")]
    Depth,
    #[error("liability accounts cannot be mapped to buckets")]
    LiabilityAccount,
    #[error("an account can map to a bucket only once")]
    DuplicateBucket,
    #[error("target amount must be greater than 0")]
    InvalidAmount,
}

/// 10000 bp = 100%.
const FULL: i64 = 10_000;

pub fn check_bp(bp: i64) -> Result<i64, AllocationError> {
    if (0..=FULL).contains(&bp) {
        Ok(bp)
    } else {
        Err(AllocationError::InvalidBasisPoints)
    }
}

/// Target bucket: persen (bp) dari basis tingkatnya, atau nominal tetap (satuan minor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Percent(i64),
    Amount(i64),
}

pub fn check_target(target: Target) -> Result<Target, AllocationError> {
    match target {
        Target::Percent(bp) => check_bp(bp).map(Target::Percent),
        Target::Amount(n) if n > 0 => Ok(target),
        Target::Amount(_) => Err(AllocationError::InvalidAmount),
    }
}

/// Target persen satu tingkat (sibling) tidak boleh lebih dari 100%; kurang dari 100% hanya warning.
/// Bucket nominal tidak ikut dijumlah.
pub fn check_sibling_targets(
    targets: impl IntoIterator<Item = i64>,
) -> Result<(), AllocationError> {
    let total: i64 = targets.into_iter().map(check_bp).sum::<Result<_, _>>()?;
    if total > FULL {
        return Err(AllocationError::TargetsExceed { total });
    }
    Ok(())
}

/// Porsi satu akun ke beberapa bucket: tiap bucket sekali, Σ ≤ 100% (sisanya "belum dialokasikan").
pub fn check_shares(shares: &[(Uuid, i64)]) -> Result<(), AllocationError> {
    let mut seen = HashSet::new();
    let mut total = 0;
    for &(bucket, bp) in shares {
        total += check_bp(bp)?;
        if !seen.insert(bucket) {
            return Err(AllocationError::DuplicateBucket);
        }
    }
    if total > FULL {
        return Err(AllocationError::SharesExceed { total });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bucket {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub target: Target,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub account_id: Uuid,
    pub bucket_id: Uuid,
    pub share_bp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountBalance {
    pub id: Uuid,
    pub kind: AccountKind,
    pub balance: i64,
    pub include_in_net_worth: bool,
}

/// Target efektif: `target_bp` global (porsi dari gross) dan `target_amount` dalam satuan minor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketActual {
    pub bucket_id: Uuid,
    pub target_bp: i64,
    pub target_amount: i64,
    pub actual: i64,
    pub actual_bp: i64,
    pub diff_bp: i64,
    pub diff_amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unallocated {
    pub account_id: Uuid,
    pub remaining_bp: i64,
    pub amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SiblingWarning {
    pub parent_id: Option<Uuid>,
    pub total_bp: i64,
}

/// Σ target nominal satu tingkat melebihi basisnya (gross untuk root, target parent untuk child).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmountWarning {
    pub parent_id: Option<Uuid>,
    pub total_amount: i64,
    pub base: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationReport {
    pub gross: i64,
    pub liabilities: i64,
    pub buckets: Vec<BucketActual>,
    pub unallocated: Vec<Unallocated>,
    pub unallocated_total: i64,
    pub unallocated_bp: i64,
    pub warnings: Vec<SiblingWarning>,
    pub amount_warnings: Vec<AmountWarning>,
}

fn fit(v: i128) -> Result<i64, MoneyError> {
    i64::try_from(v).map_err(|_| MoneyError::Overflow)
}

/// Target efektif seluruh bucket dari basis root `root_base`: per tingkat, bucket nominal
/// dipenuhi lebih dulu dan bucket persen membagi sisanya; basis child = target parent.
/// `nominal` memetakan nominal ke ruang hitung yang dipakai (rupiah atau bp).
fn level_targets(
    buckets: &[Bucket],
    root_base: i128,
    nominal: impl Fn(i64) -> i128,
) -> HashMap<Uuid, i128> {
    let full = i128::from(FULL);
    let mut out = HashMap::new();
    let assign = |parent: Option<Uuid>, base: i128, out: &mut HashMap<Uuid, i128>| {
        let siblings = buckets.iter().filter(|b| b.parent_id == parent);
        let fixed: i128 = siblings
            .clone()
            .filter_map(|b| match b.target {
                Target::Amount(n) => Some(nominal(n)),
                Target::Percent(_) => None,
            })
            .sum();
        let rest = (base - fixed).max(0);
        for b in siblings {
            let t = match b.target {
                Target::Amount(n) => nominal(n),
                Target::Percent(bp) => rest * i128::from(bp) / full,
            };
            out.insert(b.id, t);
        }
    };
    assign(None, root_base, &mut out);
    for root in buckets.iter().filter(|b| b.parent_id.is_none()) {
        let base = out[&root.id];
        assign(Some(root.id), base, &mut out);
    }
    out
}

/// Porsi aktual tiap bucket vs target. Hanya akun aset (lihat
/// [`AccountKind::counts_as_liability`]) yang dihitung di net worth yang masuk `gross`; mapping dari akun lain diabaikan. Urutan `buckets` dipertahankan.
pub fn allocation_report(
    buckets: &[Bucket],
    mappings: &[Mapping],
    accounts: &[AccountBalance],
) -> Result<AllocationReport, MoneyError> {
    let full = i128::from(FULL);
    let (mut gross, mut liabilities) = (0i128, 0i128);
    let mut counted = HashMap::new();
    for a in accounts.iter().filter(|a| a.include_in_net_worth) {
        if a.kind.counts_as_liability(a.balance) {
            liabilities -= i128::from(a.balance);
        } else {
            gross += i128::from(a.balance);
            counted.insert(a.id, a.balance);
        }
    }

    let mut direct: HashMap<Uuid, i128> = HashMap::new();
    let mut mapped_bp: HashMap<Uuid, i64> = HashMap::new();
    for m in mappings {
        let Some(&balance) = counted.get(&m.account_id) else {
            continue;
        };
        *direct.entry(m.bucket_id).or_default() +=
            i128::from(balance) * i128::from(m.share_bp) / full;
        *mapped_bp.entry(m.account_id).or_default() += m.share_bp;
    }

    let actual_of = |id: Uuid| -> i128 {
        direct.get(&id).copied().unwrap_or(0)
            + buckets
                .iter()
                .filter(|c| c.parent_id == Some(id))
                .map(|c| direct.get(&c.id).copied().unwrap_or(0))
                .sum::<i128>()
    };
    let bp_of = |amount: i128| if gross > 0 { amount * full / gross } else { 0 };
    // Dua ruang hitung dengan aturan yang sama: rupiah (basis gross) dan bp (basis 10000).
    let amount_targets = level_targets(buckets, gross, i128::from);
    let bp_targets = level_targets(buckets, full, |n| bp_of(i128::from(n)));

    let rows = buckets
        .iter()
        .map(|b| {
            let actual = actual_of(b.id);
            // Bucket di luar 2 tingkat (DB diedit manual) tidak punya target: 0, bukan panik.
            let target_of = |m: &HashMap<Uuid, i128>| m.get(&b.id).copied().unwrap_or(0);
            let (target_bp, target_amount) = (target_of(&bp_targets), target_of(&amount_targets));
            let actual_bp = bp_of(actual);
            Ok(BucketActual {
                bucket_id: b.id,
                target_bp: fit(target_bp)?,
                target_amount: fit(target_amount)?,
                actual: fit(actual)?,
                actual_bp: fit(actual_bp)?,
                diff_bp: fit(actual_bp - target_bp)?,
                diff_amount: fit(actual - target_amount)?,
            })
        })
        .collect::<Result<Vec<_>, MoneyError>>()?;

    let mut unallocated = Vec::new();
    let mut unallocated_total = 0i128;
    for a in accounts.iter().filter(|a| counted.contains_key(&a.id)) {
        let remaining_bp = FULL - mapped_bp.get(&a.id).copied().unwrap_or(0);
        let amount = i128::from(a.balance) * i128::from(remaining_bp) / full;
        if remaining_bp <= 0 || amount == 0 {
            continue;
        }
        unallocated_total += amount;
        unallocated.push(Unallocated {
            account_id: a.id,
            remaining_bp,
            amount: fit(amount)?,
        });
    }

    // Per kelompok sibling, urut kemunculan parent: warning bila Σ target persen ≠ 100%
    // (hanya bila ada bucket persen) dan bila Σ nominal melebihi basis tingkatnya.
    let mut warnings: Vec<SiblingWarning> = Vec::new();
    let mut amount_warnings: Vec<AmountWarning> = Vec::new();
    let mut seen: HashSet<Option<Uuid>> = HashSet::new();
    for b in buckets {
        if !seen.insert(b.parent_id) {
            continue;
        }
        let siblings = || buckets.iter().filter(|s| s.parent_id == b.parent_id);
        let percents: Vec<i64> = siblings()
            .filter_map(|s| match s.target {
                Target::Percent(bp) => Some(bp),
                Target::Amount(_) => None,
            })
            .collect();
        let total_bp: i64 = percents.iter().sum();
        if !percents.is_empty() && total_bp != FULL {
            warnings.push(SiblingWarning {
                parent_id: b.parent_id,
                total_bp,
            });
        }
        let total_amount: i128 = siblings()
            .filter_map(|s| match s.target {
                Target::Amount(n) => Some(i128::from(n)),
                Target::Percent(_) => None,
            })
            .sum();
        let base = b
            .parent_id
            .map_or(gross, |p| amount_targets.get(&p).copied().unwrap_or(0));
        if gross > 0 && total_amount > base {
            amount_warnings.push(AmountWarning {
                parent_id: b.parent_id,
                total_amount: fit(total_amount)?,
                base: fit(base)?,
            });
        }
    }

    Ok(AllocationReport {
        gross: fit(gross)?,
        liabilities: fit(liabilities)?,
        buckets: rows,
        unallocated,
        unallocated_total: fit(unallocated_total)?,
        unallocated_bp: fit(bp_of(unallocated_total))?,
        warnings,
        amount_warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Uuid {
        Uuid::now_v7()
    }

    fn bucket(id: Uuid, parent_id: Option<Uuid>, target_bp: i64) -> Bucket {
        nominal(id, parent_id, Target::Percent(target_bp))
    }

    fn nominal(id: Uuid, parent_id: Option<Uuid>, target: Target) -> Bucket {
        Bucket {
            id,
            parent_id,
            target,
        }
    }

    fn map(account_id: Uuid, bucket_id: Uuid, share_bp: i64) -> Mapping {
        Mapping {
            account_id,
            bucket_id,
            share_bp,
        }
    }

    fn acct(id: Uuid, kind: AccountKind, balance: i64) -> AccountBalance {
        AccountBalance {
            id,
            kind,
            balance,
            include_in_net_worth: true,
        }
    }

    fn row(r: &AllocationReport, b: Uuid) -> BucketActual {
        *r.buckets.iter().find(|x| x.bucket_id == b).unwrap()
    }

    /// Aset 7000 → Saham 6000, Emas 4000; Liquid 3000.
    fn tree() -> (Vec<Bucket>, Uuid, Uuid, Uuid, Uuid) {
        let (aset, saham, emas, liquid) = (id(), id(), id(), id());
        let b = vec![
            bucket(aset, None, 7000),
            bucket(saham, Some(aset), 6000),
            bucket(emas, Some(aset), 4000),
            bucket(liquid, None, 3000),
        ];
        (b, aset, saham, emas, liquid)
    }

    #[test]
    fn nested_target_is_relative_to_parent() {
        let (b, aset, saham, emas, liquid) = tree();
        let r = allocation_report(&b, &[], &[]).unwrap();
        assert_eq!(row(&r, aset).target_bp, 7000);
        assert_eq!(row(&r, saham).target_bp, 4200);
        assert_eq!(row(&r, emas).target_bp, 2800);
        assert_eq!(row(&r, liquid).target_bp, 3000);
        assert_eq!(
            r.buckets.iter().map(|x| x.bucket_id).collect::<Vec<_>>(),
            vec![aset, saham, emas, liquid]
        );
    }

    #[test]
    fn split_mapping_and_liabilities_excluded() {
        let (b, aset, saham, _, liquid) = tree();
        let (rdn, bca, cc) = (id(), id(), id());
        let maps = [
            map(rdn, saham, 9000),
            map(rdn, liquid, 1000),
            map(bca, liquid, 10000),
        ];
        let accts = [
            acct(rdn, AccountKind::Investment, 10_000_000),
            acct(bca, AccountKind::Bank, 5_000_000),
            acct(cc, AccountKind::CreditCard, -2_000_000),
        ];
        let r = allocation_report(&b, &maps, &accts).unwrap();
        assert_eq!(r.gross, 15_000_000);
        assert_eq!(r.liabilities, 2_000_000);
        assert_eq!(row(&r, saham).actual, 9_000_000);
        assert_eq!(row(&r, aset).actual, 9_000_000);
        assert_eq!(row(&r, liquid).actual, 6_000_000);
        assert_eq!(row(&r, liquid).actual_bp, 4000);
        assert_eq!(row(&r, liquid).diff_bp, 1000);
        assert_eq!(row(&r, liquid).diff_amount, 6_000_000 - 4_500_000);
        assert_eq!(row(&r, saham).diff_amount, 9_000_000 - 6_300_000);
        assert!(r.unallocated.is_empty());
        assert_eq!((r.unallocated_total, r.unallocated_bp), (0, 0));
    }

    #[test]
    fn allocation_skips_negative_receivable() {
        let (b, _, _, _, liquid) = tree();
        let (bca, budi) = (id(), id());
        let maps = [map(bca, liquid, 10000), map(budi, liquid, 10000)];
        let accts = [
            acct(bca, AccountKind::Bank, 1_000),
            acct(budi, AccountKind::Receivable, -500),
        ];
        let r = allocation_report(&b, &maps, &accts).unwrap();
        assert_eq!(r.gross, 1_000);
        assert_eq!(r.liabilities, 500);
        assert_eq!(row(&r, liquid).actual, 1_000);
    }

    #[test]
    fn unmapped_and_partial_accounts_are_unallocated() {
        let (b, _, saham, _, _) = tree();
        let (gopay, rdn, excluded) = (id(), id(), id());
        let accts = [
            acct(gopay, AccountKind::Ewallet, 300_000),
            acct(rdn, AccountKind::Investment, 1_000_000),
            AccountBalance {
                include_in_net_worth: false,
                ..acct(excluded, AccountKind::Bank, 9_000)
            },
        ];
        let r = allocation_report(&b, &[map(rdn, saham, 6000)], &accts).unwrap();
        assert_eq!(r.gross, 1_300_000);
        assert_eq!(r.unallocated_total, 300_000 + 400_000);
        assert_eq!(r.unallocated.len(), 2);
        let partial = r.unallocated.iter().find(|u| u.account_id == rdn).unwrap();
        assert_eq!((partial.remaining_bp, partial.amount), (4000, 400_000));
        assert_eq!(r.unallocated_bp, 700_000 * 10000 / 1_300_000);
    }

    #[test]
    fn warnings_for_levels_not_100() {
        let (aset, saham, emas, liquid) = (id(), id(), id(), id());
        let b = [
            bucket(aset, None, 6000),
            bucket(saham, Some(aset), 6000),
            bucket(emas, Some(aset), 4000),
            bucket(liquid, None, 3000),
        ];
        let r = allocation_report(&b, &[], &[]).unwrap();
        assert_eq!(
            r.warnings,
            vec![SiblingWarning {
                parent_id: None,
                total_bp: 9000
            }]
        );
    }

    #[test]
    fn zero_gross_gives_zero_bp() {
        let (b, _, saham, _, _) = tree();
        let a = id();
        let r = allocation_report(
            &b,
            &[map(a, saham, 10000)],
            &[acct(a, AccountKind::Bank, 0)],
        )
        .unwrap();
        assert_eq!(r.gross, 0);
        assert!(r.buckets.iter().all(|x| x.actual_bp == 0 && x.actual == 0));
        assert_eq!(r.unallocated_bp, 0);
    }

    #[test]
    fn nominal_bucket_takes_amount_first() {
        let (darurat, invest, liquid, bca) = (id(), id(), id(), id());
        let b = [
            nominal(darurat, None, Target::Amount(30_000_000)),
            bucket(invest, None, 6000),
            bucket(liquid, None, 4000),
        ];
        let r = allocation_report(&b, &[], &[acct(bca, AccountKind::Bank, 50_000_000)]).unwrap();
        let t = |b| (row(&r, b).target_amount, row(&r, b).target_bp);
        assert_eq!(t(darurat), (30_000_000, 6000));
        assert_eq!(t(invest), (12_000_000, 2400));
        assert_eq!(t(liquid), (8_000_000, 1600));
        assert_eq!(row(&r, invest).diff_amount, -12_000_000);
        assert!(r.warnings.is_empty() && r.amount_warnings.is_empty());
    }

    #[test]
    fn child_percent_of_nominal_parent() {
        let (darurat, a, bca) = (id(), id(), id());
        let b = [
            nominal(darurat, None, Target::Amount(30_000_000)),
            bucket(a, Some(darurat), 5000),
        ];
        let r = allocation_report(&b, &[], &[acct(bca, AccountKind::Bank, 60_000_000)]).unwrap();
        assert_eq!(
            (row(&r, a).target_amount, row(&r, a).target_bp),
            (15_000_000, 2500)
        );
    }

    #[test]
    fn nominal_exceeding_base_warns() {
        let (x, y, z, bca) = (id(), id(), id(), id());
        let b = [
            nominal(x, None, Target::Amount(30_000_000)),
            nominal(y, None, Target::Amount(30_000_000)),
            bucket(z, None, 10000),
        ];
        let maps = [map(bca, x, 10000)];
        let r = allocation_report(&b, &maps, &[acct(bca, AccountKind::Bank, 50_000_000)]).unwrap();
        assert_eq!(
            r.amount_warnings,
            vec![AmountWarning {
                parent_id: None,
                total_amount: 60_000_000,
                base: 50_000_000
            }]
        );
        assert_eq!((row(&r, z).target_amount, row(&r, z).target_bp), (0, 0));
        assert_eq!(row(&r, x).diff_amount, 50_000_000 - 30_000_000);
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn zero_gross_with_nominal() {
        let (x, z) = (id(), id());
        let b = [
            nominal(x, None, Target::Amount(1_000_000)),
            bucket(z, None, 10000),
        ];
        let r = allocation_report(&b, &[], &[]).unwrap();
        assert_eq!(row(&r, x).target_bp, 0);
        assert_eq!(row(&r, x).target_amount, 1_000_000);
        assert_eq!(row(&r, z).target_bp, 10000);
        assert!(r.amount_warnings.is_empty());
    }

    #[test]
    fn percent_warning_ignores_nominal() {
        let (x, p, c) = (id(), id(), id());
        let b = [
            nominal(x, None, Target::Amount(1_000)),
            bucket(p, None, 10000),
            nominal(c, Some(p), Target::Amount(500)),
        ];
        let r = allocation_report(&b, &[], &[]).unwrap();
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn target_checks() {
        assert_eq!(
            check_target(Target::Amount(0)),
            Err(AllocationError::InvalidAmount)
        );
        assert_eq!(
            check_target(Target::Amount(-5)),
            Err(AllocationError::InvalidAmount)
        );
        assert_eq!(
            check_target(Target::Percent(10001)),
            Err(AllocationError::InvalidBasisPoints)
        );
        assert_eq!(check_target(Target::Amount(1)), Ok(Target::Amount(1)));
        assert_eq!(check_target(Target::Percent(0)), Ok(Target::Percent(0)));
    }

    #[test]
    fn share_and_target_checks() {
        let (a, b) = (id(), id());
        assert_eq!(check_bp(10001), Err(AllocationError::InvalidBasisPoints));
        assert_eq!(check_bp(-1), Err(AllocationError::InvalidBasisPoints));
        assert_eq!(check_bp(10000), Ok(10000));
        assert_eq!(
            check_sibling_targets([7000, 3001]),
            Err(AllocationError::TargetsExceed { total: 10001 })
        );
        assert_eq!(check_sibling_targets([7000, 3000]), Ok(()));
        assert_eq!(
            check_shares(&[(a, 6000), (a, 1000)]),
            Err(AllocationError::DuplicateBucket)
        );
        assert_eq!(
            check_shares(&[(a, 6000), (b, 5000)]),
            Err(AllocationError::SharesExceed { total: 11000 })
        );
        assert_eq!(
            check_shares(&[(a, 10001)]),
            Err(AllocationError::InvalidBasisPoints)
        );
        assert_eq!(check_shares(&[(a, 6000), (b, 4000)]), Ok(()));
    }

    #[test]
    fn child_diff_amount_uses_parent_amount() {
        // Target rupiah child = floor(floor(gross × p) × c), bukan gross × floor(p × c) (rumus Fase 5).
        let (root, child, bca) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
        let b = [bucket(root, None, 3333), bucket(child, Some(root), 3333)];
        let r = allocation_report(&b, &[], &[acct(bca, AccountKind::Bank, 1_000_000)]).unwrap();
        assert_eq!(row(&r, child).target_bp, 1110);
        assert_eq!(row(&r, child).target_amount, 111_088);
        assert_eq!(row(&r, child).diff_amount, -111_088);
    }

    #[test]
    fn deep_tree_does_not_panic() {
        // Tingkat ke-3 hanya mungkin lewat DB yang diedit manual; laporan tidak boleh panik.
        let (a, b1, c, bca) = (
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
            Uuid::now_v7(),
        );
        let b = [
            bucket(a, None, 10000),
            bucket(b1, Some(a), 10000),
            bucket(c, Some(b1), 10000),
        ];
        let r = allocation_report(&b, &[], &[acct(bca, AccountKind::Bank, 1_000)]).unwrap();
        assert_eq!((row(&r, c).target_bp, row(&r, c).target_amount), (0, 0));
    }
}
