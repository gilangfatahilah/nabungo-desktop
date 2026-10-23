//! Use case alokasi modal (CAP-01..04): bucket bertingkat, mapping akun, ringkasan porsi
//! saat ini, dan riwayat porsi per periode (K9).

use std::collections::HashMap;

use nabungo_core::{
    AccountBalance, AllocationError, AllocationReport, Bucket, BudgetPeriod, Mapping, Target,
    allocation_report, check_shares, check_sibling_targets, check_target, normalize_name,
};
use nabungo_storage::allocation::{self, BucketRow, NewBucket};
use nabungo_storage::rusqlite::Connection;
use nabungo_storage::{account, report};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};
use crate::reports::periods_ending;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketFields {
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub target: Target,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationOverview {
    pub buckets: Vec<BucketRow>,
    pub mappings: Vec<Mapping>,
    pub report: AllocationReport,
}

/// Porsi bucket root per akhir satu periode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationPoint {
    pub period: BudgetPeriod,
    pub date: Date,
    pub gross: i64,
    /// `(bucket_id, actual_bp)`, hanya bucket root.
    pub buckets: Vec<(Uuid, i64)>,
}

impl Ledger {
    pub fn create_bucket(&mut self, f: &BucketFields) -> Result<Uuid, AppError> {
        let name = normalize_name(&f.name)?;
        let tx = self.conn.transaction()?;
        check_parent(&tx, None, f.parent_id)?;
        check_siblings(&tx, None, f.parent_id, f.target)?;
        let sort_order = allocation::next_bucket_sort_order(&tx, f.parent_id)?;
        let id = allocation::insert_bucket(&tx, &new_bucket(f, name, sort_order))
            .map_err(AppError::duplicate_as("bucket"))?;
        tx.commit()?;
        Ok(id)
    }

    /// Pindah parent → urutan di akhir parent baru; parent tetap → urutan dipertahankan.
    pub fn update_bucket(&mut self, id: Uuid, f: &BucketFields) -> Result<(), AppError> {
        let name = normalize_name(&f.name)?;
        let tx = self.conn.transaction()?;
        let current = allocation::get_bucket(&tx, id)?.ok_or(AppError::NotFound("bucket"))?;
        check_parent(&tx, Some(id), f.parent_id)?;
        check_siblings(&tx, Some(id), f.parent_id, f.target)?;
        let sort_order = if current.parent_id == f.parent_id {
            current.sort_order
        } else {
            allocation::next_bucket_sort_order(&tx, f.parent_id)?
        };
        allocation::update_bucket(&tx, id, &new_bucket(f, name, sort_order))
            .map_err(AppError::duplicate_as("bucket"))?;
        tx.commit()?;
        Ok(())
    }

    /// Bucket yang masih punya child tidak bisa dihapus; mapping-nya ikut terhapus.
    pub fn delete_bucket(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if allocation::get_bucket(&tx, id)?.is_none() {
            return Err(AppError::NotFound("bucket"));
        }
        if allocation::child_bucket_count(&tx, id)? > 0 {
            return Err(AppError::InUse("bucket"));
        }
        allocation::delete_bucket(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Mengganti seluruh porsi akun; daftar kosong menghapus semua mapping-nya.
    pub fn set_account_allocation(
        &mut self,
        account_id: Uuid,
        shares: &[(Uuid, i64)],
    ) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let acc =
            account::get_account_ref(&tx, account_id)?.ok_or(AppError::NotFound("account"))?;
        if acc.value.kind.is_liability() {
            return Err(AllocationError::LiabilityAccount.into());
        }
        check_shares(shares)?;
        for (bucket_id, _) in shares {
            if allocation::get_bucket(&tx, *bucket_id)?.is_none() {
                return Err(AppError::NotFound("bucket"));
            }
        }
        allocation::replace_account_mappings(&tx, account_id, shares)?;
        tx.commit()?;
        Ok(())
    }

    /// Porsi aktual saat ini dari saldo seluruh akun (termasuk yang diarsip, K8).
    pub fn allocation_overview(&self) -> Result<AllocationOverview, AppError> {
        let buckets = allocation::list_buckets(&self.conn)?;
        let mappings = allocation::list_mappings(&self.conn)?;
        let accounts: Vec<_> = account::list_accounts(&self.conn)?
            .into_iter()
            .map(|a| AccountBalance {
                id: a.id,
                kind: a.kind,
                balance: a.balance,
                include_in_net_worth: a.include_in_net_worth,
            })
            .collect();
        let report = allocation_report(&core_buckets(&buckets), &mappings, &accounts)?;
        Ok(AllocationOverview {
            buckets,
            mappings,
            report,
        })
    }

    /// Porsi bucket root per akhir tiap periode (K9), terlama dulu, memakai mapping & target saat ini.
    /// `count` di-clamp ke `1..=24`.
    pub fn allocation_history(
        &self,
        end: BudgetPeriod,
        count: u8,
    ) -> Result<Vec<AllocationPoint>, AppError> {
        let rows = allocation::list_buckets(&self.conn)?;
        let buckets = core_buckets(&rows);
        let mappings = allocation::list_mappings(&self.conn)?;
        self.period_points(end, count)?
            .into_iter()
            .map(|(period, date, accounts)| {
                let report = allocation_report(&buckets, &mappings, &accounts)?;
                let roots = rows
                    .iter()
                    .zip(&report.buckets)
                    .filter(|(row, _)| row.parent_id.is_none())
                    .map(|(row, actual)| (row.id, actual.actual_bp))
                    .collect();
                Ok(AllocationPoint {
                    period,
                    date,
                    gross: report.gross,
                    buckets: roots,
                })
            })
            .collect()
    }

    /// `count` periode berakhir di `end` (terlama dulu), masing-masing dengan saldo akun per
    /// hari terakhir periode. Akun tanpa posting sampai tanggal itu bersaldo 0.
    /// `count` di-clamp ke `1..=24` (lihat [`periods_ending`]).
    pub(crate) fn period_points(
        &self,
        end: BudgetPeriod,
        count: u8,
    ) -> Result<Vec<(BudgetPeriod, Date, Vec<AccountBalance>)>, AppError> {
        let start = self.period_start_day()?;
        let accounts = account::list_accounts(&self.conn)?;
        periods_ending(end, count)
            .into_iter()
            .map(|period| {
                let date = period.last_day(start);
                let balances: HashMap<_, _> = report::balances_as_of(&self.conn, date)?
                    .into_iter()
                    .collect();
                let list = accounts
                    .iter()
                    .map(|a| AccountBalance {
                        id: a.id,
                        kind: a.kind,
                        balance: balances.get(&a.id).copied().unwrap_or(0),
                        include_in_net_worth: a.include_in_net_worth,
                    })
                    .collect();
                Ok((period, date, list))
            })
            .collect()
    }
}

fn new_bucket(f: &BucketFields, name: String, sort_order: i64) -> NewBucket {
    NewBucket {
        parent_id: f.parent_id,
        name,
        target: f.target,
        color: f.color.clone(),
        sort_order,
    }
}

fn core_buckets(rows: &[BucketRow]) -> Vec<Bucket> {
    rows.iter()
        .map(|r| Bucket {
            id: r.id,
            parent_id: r.parent_id,
            target: r.target,
        })
        .collect()
}

/// Maksimal dua tingkat: parent harus root, bukan diri sendiri, dan bucket yang punya child
/// tidak boleh dijadikan child.
fn check_parent(conn: &Connection, id: Option<Uuid>, parent: Option<Uuid>) -> Result<(), AppError> {
    let Some(parent_id) = parent else {
        return Ok(());
    };
    if Some(parent_id) == id {
        return Err(AllocationError::Depth.into());
    }
    let parent = allocation::get_bucket(conn, parent_id)?.ok_or(AppError::NotFound("bucket"))?;
    if parent.parent_id.is_some() {
        return Err(AllocationError::Depth.into());
    }
    if let Some(id) = id
        && allocation::child_bucket_count(conn, id)? > 0
    {
        return Err(AllocationError::Depth.into());
    }
    Ok(())
}

/// Σ target persen sibling (tanpa bucket itu sendiri) + target baru ≤ 100%; bucket nominal
/// tidak ikut dijumlah.
fn check_siblings(
    conn: &Connection,
    id: Option<Uuid>,
    parent: Option<Uuid>,
    target: Target,
) -> Result<(), AppError> {
    let percent = |t: Target| match t {
        Target::Percent(bp) => Some(bp),
        Target::Amount(_) => None,
    };
    let target = check_target(target)?;
    let others = allocation::list_buckets(conn)?
        .into_iter()
        .filter(|b| b.parent_id == parent && Some(b.id) != id)
        .filter_map(|b| percent(b.target));
    check_sibling_targets(others.chain(percent(target)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountFields;
    use crate::testkit::Kit;
    use nabungo_core::{AccountKind, Target};
    use time::macros::date;

    fn fields(name: &str, parent_id: Option<Uuid>, target_bp: i64) -> BucketFields {
        BucketFields {
            name: name.into(),
            parent_id,
            target: Target::Percent(target_bp),
            color: None,
        }
    }

    fn nominal(name: &str, parent_id: Option<Uuid>, amount: i64) -> BucketFields {
        BucketFields {
            target: Target::Amount(amount),
            ..fields(name, parent_id, 0)
        }
    }

    fn kit() -> Kit {
        Kit::new(1, date!(2026 - 09 - 01))
    }

    #[test]
    fn bucket_depth_limited_to_two() {
        let mut k = kit();
        let aset = k.ledger.create_bucket(&fields("Aset", None, 7000)).unwrap();
        let liquid = k
            .ledger
            .create_bucket(&fields("Liquid", None, 3000))
            .unwrap();
        let saham = k
            .ledger
            .create_bucket(&fields("Saham", Some(aset), 6000))
            .unwrap();
        assert!(matches!(
            k.ledger
                .create_bucket(&fields("Bluechip", Some(saham), 10000)),
            Err(AppError::Allocation(AllocationError::Depth))
        ));
        assert!(matches!(
            k.ledger
                .update_bucket(aset, &fields("Aset", Some(liquid), 7000)),
            Err(AppError::Allocation(AllocationError::Depth))
        ));
        assert!(matches!(
            k.ledger
                .update_bucket(liquid, &fields("Liquid", Some(liquid), 3000)),
            Err(AppError::Allocation(AllocationError::Depth))
        ));
        assert!(matches!(
            k.ledger
                .create_bucket(&fields("x", Some(Uuid::now_v7()), 0)),
            Err(AppError::NotFound("bucket"))
        ));
        assert!(matches!(
            k.ledger.create_bucket(&fields(" aset ", None, 0)),
            Err(AppError::Duplicate("bucket"))
        ));
    }

    #[test]
    fn sibling_targets_cannot_exceed_100() {
        let mut k = kit();
        let aset = k.ledger.create_bucket(&fields("Aset", None, 7000)).unwrap();
        assert!(matches!(
            k.ledger.create_bucket(&fields("Liquid", None, 3001)),
            Err(AppError::Allocation(AllocationError::TargetsExceed {
                total: 10001
            }))
        ));
        let liquid = k
            .ledger
            .create_bucket(&fields("Liquid", None, 3000))
            .unwrap();
        k.ledger
            .update_bucket(aset, &fields("Aset", None, 7000))
            .unwrap();
        assert!(matches!(
            k.ledger
                .update_bucket(liquid, &fields("Liquid", None, 3500)),
            Err(AppError::Allocation(AllocationError::TargetsExceed {
                total: 10500
            }))
        ));
        assert!(matches!(
            k.ledger.create_bucket(&fields("Emas", Some(aset), 10001)),
            Err(AppError::Allocation(AllocationError::InvalidBasisPoints))
        ));
    }

    #[test]
    fn delete_bucket_with_children_is_in_use() {
        let mut k = kit();
        let aset = k.ledger.create_bucket(&fields("Aset", None, 7000)).unwrap();
        let saham = k
            .ledger
            .create_bucket(&fields("Saham", Some(aset), 6000))
            .unwrap();
        let bca = k.bca;
        k.ledger
            .set_account_allocation(bca, &[(saham, 10000)])
            .unwrap();
        assert!(matches!(
            k.ledger.delete_bucket(aset),
            Err(AppError::InUse("bucket"))
        ));
        k.ledger.delete_bucket(saham).unwrap();
        k.ledger.delete_bucket(aset).unwrap();
        let o = k.ledger.allocation_overview().unwrap();
        assert!(o.buckets.is_empty() && o.mappings.is_empty());
        assert!(matches!(
            k.ledger.delete_bucket(aset),
            Err(AppError::NotFound("bucket"))
        ));
    }

    #[test]
    fn liability_account_cannot_be_allocated() {
        let mut k = kit();
        let b = k
            .ledger
            .create_bucket(&fields("Liquid", None, 10000))
            .unwrap();
        let cc = k
            .ledger
            .create_account(
                AccountKind::CreditCard,
                &AccountFields::new("Kartu", 0, date!(2026 - 09 - 01)),
            )
            .unwrap();
        assert!(matches!(
            k.ledger.set_account_allocation(cc, &[(b, 10000)]),
            Err(AppError::Allocation(AllocationError::LiabilityAccount))
        ));
        let bca = k.bca;
        assert!(matches!(
            k.ledger
                .set_account_allocation(bca, &[(Uuid::now_v7(), 10000)]),
            Err(AppError::NotFound("bucket"))
        ));
        assert!(matches!(
            k.ledger.set_account_allocation(Uuid::now_v7(), &[]),
            Err(AppError::NotFound("account"))
        ));
        assert!(matches!(
            k.ledger
                .set_account_allocation(bca, &[(b, 6000), (b, 4000)]),
            Err(AppError::Allocation(AllocationError::DuplicateBucket))
        ));
    }

    #[test]
    fn overview_matches_core_report() {
        let mut k = kit();
        let saham = k
            .ledger
            .create_bucket(&fields("Saham", None, 6000))
            .unwrap();
        let liquid = k
            .ledger
            .create_bucket(&fields("Liquid", None, 4000))
            .unwrap();
        let rdn = k
            .ledger
            .create_account(
                AccountKind::Investment,
                &AccountFields::new("RDN", 10_000_000, date!(2026 - 09 - 01)),
            )
            .unwrap();
        let bca = k.bca;
        k.ledger
            .set_account_allocation(rdn, &[(saham, 9000), (liquid, 1000)])
            .unwrap();
        k.ledger
            .set_account_allocation(bca, &[(liquid, 10000)])
            .unwrap();
        let o = k.ledger.allocation_overview().unwrap();
        assert_eq!(
            o.buckets.iter().map(|b| b.id).collect::<Vec<_>>(),
            vec![saham, liquid]
        );
        assert_eq!(o.mappings.len(), 3);
        assert_eq!(o.report.gross, 11_000_000);
        let actual = |id| {
            o.report
                .buckets
                .iter()
                .find(|b| b.bucket_id == id)
                .unwrap()
                .actual
        };
        assert_eq!((actual(saham), actual(liquid)), (9_000_000, 2_000_000));
        k.ledger.set_account_allocation(rdn, &[]).unwrap();
        assert_eq!(k.ledger.allocation_overview().unwrap().mappings.len(), 1);
    }

    #[test]
    fn delete_account_removes_mappings() {
        let mut k = kit();
        let b = k
            .ledger
            .create_bucket(&fields("Liquid", None, 10000))
            .unwrap();
        let gopay = k
            .ledger
            .create_account(
                AccountKind::Ewallet,
                &AccountFields::new("GoPay", 0, date!(2026 - 09 - 01)),
            )
            .unwrap();
        k.ledger
            .set_account_allocation(gopay, &[(b, 10000)])
            .unwrap();
        k.ledger.delete_account(gopay).unwrap();
        assert!(k.ledger.allocation_overview().unwrap().mappings.is_empty());
    }

    #[test]
    fn history_recomputes_from_postings() {
        let mut k = kit();
        let liquid = k
            .ledger
            .create_bucket(&fields("Liquid", None, 5000))
            .unwrap();
        let child = k
            .ledger
            .create_bucket(&fields("Kas", Some(liquid), 10000))
            .unwrap();
        k.ledger
            .create_account(
                AccountKind::Investment,
                &AccountFields::new("RDN", 1_000_000, date!(2026 - 09 - 01)),
            )
            .unwrap();
        let bca = k.bca;
        k.ledger
            .set_account_allocation(bca, &[(child, 10000)])
            .unwrap();
        k.expense(date!(2026 - 09 - 15), "Kopi & jajan", 500_000);
        k.expense(date!(2026 - 10 - 05), "Kopi & jajan", 250_000);
        let h = k
            .ledger
            .allocation_history(BudgetPeriod::new(2026, 10).unwrap(), 2)
            .unwrap();
        assert_eq!(
            h,
            vec![
                AllocationPoint {
                    period: BudgetPeriod::new(2026, 9).unwrap(),
                    date: date!(2026 - 09 - 30),
                    gross: 1_500_000,
                    buckets: vec![(liquid, 3333)],
                },
                AllocationPoint {
                    period: BudgetPeriod::new(2026, 10).unwrap(),
                    date: date!(2026 - 10 - 31),
                    gross: 1_250_000,
                    buckets: vec![(liquid, 2000)],
                },
            ]
        );
    }

    #[test]
    fn create_nominal_bucket_and_overview() {
        let mut k = kit();
        let darurat = k
            .ledger
            .create_bucket(&nominal("Darurat", None, 500_000))
            .unwrap();
        let invest = k
            .ledger
            .create_bucket(&fields("Investasi", None, 10000))
            .unwrap();
        let o = k.ledger.allocation_overview().unwrap();
        assert_eq!(o.buckets[0].target, Target::Amount(500_000));
        let t = |id| {
            let b = o.report.buckets.iter().find(|b| b.bucket_id == id).unwrap();
            (b.target_amount, b.target_bp)
        };
        assert_eq!(t(darurat), (500_000, 5000));
        assert_eq!(t(invest), (500_000, 5000));
    }

    #[test]
    fn nominal_must_be_positive() {
        let mut k = kit();
        assert!(matches!(
            k.ledger.create_bucket(&nominal("Darurat", None, 0)),
            Err(AppError::Allocation(AllocationError::InvalidAmount))
        ));
    }

    #[test]
    fn switch_target_kind_roundtrip() {
        let mut k = kit();
        let a = k.ledger.create_bucket(&fields("A", None, 6000)).unwrap();
        k.ledger.create_bucket(&fields("B", None, 4000)).unwrap();
        k.ledger
            .update_bucket(a, &nominal("A", None, 1_000))
            .unwrap();
        k.ledger.create_bucket(&fields("C", None, 6000)).unwrap();
        assert!(matches!(
            k.ledger.update_bucket(a, &fields("A", None, 1)),
            Err(AppError::Allocation(AllocationError::TargetsExceed {
                total: 10001
            }))
        ));
    }
}
