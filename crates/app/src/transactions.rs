//! Use case form transaksi (TRX-01..05): buat, edit, detail, daftar, saran payee/tag.

use std::collections::HashSet;

use nabungo_core::{CategoryKind, FormBody, TxnKind, ValidTxn, form_postings, normalize_name};
use nabungo_storage::rusqlite::Transaction;
use nabungo_storage::txn::PostingRecord;
use nabungo_storage::txn_list::{TxnPage, TxnQuery};
use nabungo_storage::{category, now_utc, payee, tag, txn, txn_list};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger, PostingInput, TxnInput, resolve};
use crate::payee::resolve_payee;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnForm {
    pub body: FormBody,
    pub date: Date,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnDetail {
    pub id: Uuid,
    pub kind: TxnKind,
    pub date: Date,
    pub payee: Option<String>,
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub postings: Vec<PostingRecord>,
}

/// `#kantor`, ` kantor ` → `kantor`.
pub(crate) fn normalize_tag(raw: &str) -> Result<String, AppError> {
    Ok(normalize_name(raw.trim().trim_start_matches('#'))?)
}

/// Validasi + simpan (insert bila `existing` kosong, selain itu tulis ulang) beserta payee & tag.
pub(crate) fn write_txn(
    tx: &Transaction,
    existing: Option<Uuid>,
    input: &TxnInput,
    archived_ok: &HashSet<Uuid>,
) -> Result<Uuid, AppError> {
    let now = now_utc();
    let valid = ValidTxn::new(resolve(tx, input, archived_ok)?)?;
    let payee = resolve_payee(tx, input.payee.as_deref())?;
    let id = match existing {
        None => txn::insert_txn(tx, &valid, payee, &now)?,
        Some(id) => {
            if !txn::replace_txn(tx, id, &valid, payee, &now)? {
                return Err(AppError::NotFound("transaction"));
            }
            id
        }
    };
    let mut names = Vec::new();
    for raw in &input.tags {
        let name = normalize_tag(raw)?;
        if !names.iter().any(|n: &String| n.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    let ids = names
        .iter()
        .map(|n| tag::find_or_insert_tag(tx, n))
        .collect::<Result<Vec<_>, _>>()?;
    tag::set_txn_tags(tx, id, &ids)?;
    Ok(id)
}

/// Nama kategori seed biaya admin (id, en), lihat `defaults.rs`.
const SEED_FEE_NAMES: [&str; 2] = ["Biaya admin", "Fees"];

fn form_input(form: &TxnForm) -> Result<TxnInput, AppError> {
    let postings = form_postings(&form.body)?
        .into_iter()
        .map(|p| PostingInput {
            account_id: p.account_id,
            category_id: p.category_id,
            amount: p.amount,
            memo: p.memo,
        })
        .collect();
    Ok(TxnInput {
        kind: form.body.kind(),
        date: form.date,
        payee: form.payee.clone(),
        note: form.note.clone(),
        postings,
        tags: form.tags.clone(),
    })
}

impl Ledger {
    pub fn create_txn(&mut self, form: &TxnForm) -> Result<Uuid, AppError> {
        let input = form_input(form)?;
        let tx = self.conn.transaction()?;
        let id = write_txn(&tx, None, &input, &HashSet::new())?;
        tx.commit()?;
        Ok(id)
    }

    /// Akun/kategori yang dipakai versi lama boleh sudah diarsip.
    pub fn update_txn(&mut self, id: Uuid, form: &TxnForm) -> Result<(), AppError> {
        let input = form_input(form)?;
        let tx = self.conn.transaction()?;
        let old = match txn::get_txn(&tx, id)? {
            Some(old) if !old.deleted => old,
            _ => return Err(AppError::NotFound("transaction")),
        };
        match old.kind {
            TxnKind::Opening => return Err(AppError::OpeningNotAllowed),
            TxnKind::Valuation | TxnKind::Adjustment => return Err(AppError::NotEditable),
            _ => {}
        }
        let archived_ok: HashSet<Uuid> = old
            .postings
            .iter()
            .flat_map(|p| std::iter::once(p.account_id).chain(p.category_id))
            .collect();
        write_txn(&tx, Some(id), &input, &archived_ok)?;
        tx.commit()?;
        Ok(())
    }

    /// `NotFound("transaction")` bila tidak ada atau sudah terhapus.
    pub fn get_txn(&self, id: Uuid) -> Result<TxnDetail, AppError> {
        let rec = match txn::get_txn(&self.conn, id)? {
            Some(rec) if !rec.deleted => rec,
            _ => return Err(AppError::NotFound("transaction")),
        };
        Ok(TxnDetail {
            id: rec.id,
            kind: rec.kind,
            date: rec.date,
            payee: rec.payee,
            note: rec.note,
            tags: tag::txn_tags(&self.conn, id)?,
            postings: rec.postings,
        })
    }

    pub fn list_txns(&self, q: &TxnQuery) -> Result<TxnPage, AppError> {
        Ok(txn_list::list_txns(&self.conn, q)?)
    }

    /// Maksimal 8 nama, yang terakhir dipakai lebih dulu.
    pub fn search_payees(&self, query: &str) -> Result<Vec<String>, AppError> {
        Ok(payee::search_payees(&self.conn, query.trim(), 8)?)
    }

    pub fn list_tags(&self) -> Result<Vec<String>, AppError> {
        Ok(tag::list_tags(&self.conn)?)
    }

    /// Akun aktif dari income/expense terakhir, untuk default form.
    pub fn last_used_account(&self) -> Result<Option<Uuid>, AppError> {
        Ok(txn::last_used_account(&self.conn)?)
    }

    /// Kategori biaya transfer terakhir (bila aktif), selain itu kategori seed "Biaya admin"/"Fees".
    pub fn default_fee_category(&self) -> Result<Option<Uuid>, AppError> {
        if let Some(id) = txn::last_fee_category(&self.conn)?
            && category::get_category_ref(&self.conn, id)?.is_some_and(|c| !c.archived)
        {
            return Ok(Some(id));
        }
        Ok(category::list_categories(&self.conn)?
            .into_iter()
            .find(|c| {
                c.kind == CategoryKind::Expense
                    && !c.archived
                    && SEED_FEE_NAMES
                        .iter()
                        .any(|n| c.name.eq_ignore_ascii_case(n))
            })
            .map(|c| c.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountFields, CategoryFields, OnboardingInput, TxnError};
    use crate::{Locale, PeriodStartDay};
    use nabungo_core::{AccountKind, Fee, SplitLine};
    use time::macros::date;

    const D: Date = date!(2026 - 10 - 17);

    struct Fx {
        ledger: Ledger,
        a: Uuid,
        b: Uuid,
        makan: Uuid,
        kopi: Uuid,
        gaji: Uuid,
        biaya: Uuid,
    }

    fn fx() -> Fx {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let a = ledger
            .create_account(AccountKind::Bank, &AccountFields::new("A", 1_000_000, D))
            .unwrap();
        let b = ledger
            .create_account(AccountKind::Bank, &AccountFields::new("B", 0, D))
            .unwrap();
        let mut cat = |kind, name: &str| {
            ledger
                .create_category(kind, &CategoryFields::named(name))
                .unwrap()
        };
        let makan = cat(CategoryKind::Expense, "Makan");
        let kopi = cat(CategoryKind::Expense, "Kopi");
        let gaji = cat(CategoryKind::Income, "Gaji");
        let biaya = cat(CategoryKind::Expense, "Biaya bank");
        Fx {
            ledger,
            a,
            b,
            makan,
            kopi,
            gaji,
            biaya,
        }
    }

    fn line(category_id: Uuid, amount: i64) -> SplitLine {
        SplitLine {
            category_id,
            amount,
            memo: None,
        }
    }

    fn form(body: FormBody) -> TxnForm {
        TxnForm {
            body,
            date: D,
            payee: None,
            note: None,
            tags: vec![],
        }
    }

    fn expense(account_id: Uuid, amount: i64, splits: Vec<SplitLine>) -> TxnForm {
        form(FormBody::Expense {
            account_id,
            amount,
            splits,
        })
    }

    fn count(ledger: &Ledger, table: &str) -> i64 {
        ledger
            .conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn create_expense_with_split_tags_and_payee() {
        let mut f = fx();
        let mut input = expense(
            f.a,
            75_000,
            vec![line(f.makan, 50_000), line(f.kopi, 25_000)],
        );
        input.payee = Some("Indomaret".into());
        input.tags = vec!["#kantor".into(), " belanja ".into(), "KANTOR".into()];
        let id = f.ledger.create_txn(&input).unwrap();
        assert_eq!(f.ledger.balance(f.a).unwrap(), 925_000);
        let d = f.ledger.get_txn(id).unwrap();
        assert_eq!(d.kind, TxnKind::Expense);
        assert_eq!(d.postings.len(), 2);
        assert_eq!(d.payee.as_deref(), Some("Indomaret"));
        assert_eq!(d.tags, vec!["belanja", "kantor"]);
        assert_eq!(f.ledger.list_tags().unwrap(), vec!["belanja", "kantor"]);
    }

    #[test]
    fn split_mismatch_writes_nothing() {
        let mut f = fx();
        let before = ["txn", "payee", "tag"].map(|t| count(&f.ledger, t));
        let mut input = expense(
            f.a,
            75_000,
            vec![line(f.makan, 50_000), line(f.kopi, 24_999)],
        );
        input.payee = Some("Baru".into());
        input.tags = vec!["baru".into()];
        let err = f.ledger.create_txn(&input).unwrap_err();
        assert!(matches!(err, AppError::Txn(TxnError::SplitMismatch(1))));
        assert_eq!(["txn", "payee", "tag"].map(|t| count(&f.ledger, t)), before);
    }

    #[test]
    fn transfer_with_fee_moves_and_charges() {
        let mut f = fx();
        f.ledger
            .create_txn(&form(FormBody::Transfer {
                from: f.a,
                to: f.b,
                amount: 200_000,
                fee: Some(Fee {
                    amount: 6_500,
                    category_id: f.biaya,
                }),
            }))
            .unwrap();
        assert_eq!(f.ledger.balance(f.a).unwrap(), 793_500);
        assert_eq!(f.ledger.balance(f.b).unwrap(), 200_000);
    }

    #[test]
    fn refund_split_raises_balance() {
        let mut f = fx();
        f.ledger
            .create_txn(&expense(
                f.a,
                -30_000,
                vec![line(f.makan, -10_000), line(f.kopi, -20_000)],
            ))
            .unwrap();
        assert_eq!(f.ledger.balance(f.a).unwrap(), 1_030_000);
    }

    #[test]
    fn update_replaces_and_keeps_id() {
        let mut f = fx();
        let mut input = expense(f.a, 50_000, vec![line(f.makan, 50_000)]);
        input.tags = vec!["lama".into()];
        let id = f.ledger.create_txn(&input).unwrap();
        let mut new = form(FormBody::Income {
            account_id: f.a,
            amount: 300_000,
            splits: vec![line(f.gaji, 300_000)],
        });
        new.tags = vec!["baru".into()];
        f.ledger.update_txn(id, &new).unwrap();
        assert_eq!(f.ledger.balance(f.a).unwrap(), 1_300_000);
        let d = f.ledger.get_txn(id).unwrap();
        assert_eq!(d.id, id);
        assert_eq!(d.kind, TxnKind::Income);
        assert_eq!(d.tags, vec!["baru"]);
        assert_eq!(count(&f.ledger, "txn"), 2);
    }

    #[test]
    fn update_on_archived_account_keeps_working() {
        let mut f = fx();
        let id = f
            .ledger
            .create_txn(&expense(f.a, 50_000, vec![line(f.makan, 50_000)]))
            .unwrap();
        f.ledger.archive_account(f.a).unwrap();
        f.ledger.archive_category(f.makan).unwrap();
        let mut same = expense(f.a, 50_000, vec![line(f.makan, 50_000)]);
        same.note = Some("catatan".into());
        f.ledger.update_txn(id, &same).unwrap();
        assert_eq!(
            f.ledger.get_txn(id).unwrap().note.as_deref(),
            Some("catatan")
        );

        f.ledger.archive_account(f.b).unwrap();
        let err = f
            .ledger
            .update_txn(id, &expense(f.b, 50_000, vec![line(f.makan, 50_000)]))
            .unwrap_err();
        assert!(matches!(err, AppError::Archived("account")));
        // Ref arsip tidak boleh dipakai transaksi baru.
        let err = f
            .ledger
            .create_txn(&expense(f.a, 50_000, vec![line(f.kopi, 50_000)]))
            .unwrap_err();
        assert!(matches!(err, AppError::Archived("account")));
    }

    #[test]
    fn update_opening_rejected() {
        let mut f = fx();
        let opening: Uuid = f
            .ledger
            .conn
            .query_row("SELECT id FROM txn WHERE kind = 'opening'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap()
            .parse()
            .unwrap();
        let err = f
            .ledger
            .update_txn(opening, &expense(f.a, 1, vec![line(f.makan, 1)]))
            .unwrap_err();
        assert!(matches!(err, AppError::OpeningNotAllowed));
    }

    #[test]
    fn update_deleted() {
        let mut f = fx();
        let input = expense(f.a, 1, vec![line(f.makan, 1)]);
        let id = f.ledger.create_txn(&input).unwrap();
        f.ledger.delete_txn(id).unwrap();
        assert!(matches!(
            f.ledger.update_txn(id, &input),
            Err(AppError::NotFound("transaction"))
        ));
        assert!(matches!(
            f.ledger.get_txn(id),
            Err(AppError::NotFound("transaction"))
        ));
        assert!(matches!(
            f.ledger.update_txn(Uuid::now_v7(), &input),
            Err(AppError::NotFound("transaction"))
        ));
    }

    #[test]
    fn delete_undo_roundtrip_restores_balance_and_list() {
        let mut f = fx();
        let id = f
            .ledger
            .create_txn(&expense(f.a, 25_000, vec![line(f.kopi, 25_000)]))
            .unwrap();
        let q = TxnQuery {
            limit: 50,
            ..TxnQuery::default()
        };
        assert_eq!(f.ledger.list_txns(&q).unwrap().total, 2);
        f.ledger.delete_txn(id).unwrap();
        assert_eq!(f.ledger.list_txns(&q).unwrap().total, 1);
        assert_eq!(f.ledger.balance(f.a).unwrap(), 1_000_000);
        f.ledger.undo_delete_txn(id).unwrap();
        let page = f.ledger.list_txns(&q).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.rows[0].id, id);
        assert_eq!(f.ledger.balance(f.a).unwrap(), 975_000);
    }

    #[test]
    fn default_fee_category_falls_back_to_seed_name() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let a = ledger
            .complete_onboarding(&OnboardingInput {
                locale: Locale::Id,
                period_start_day: PeriodStartDay::new(1).unwrap(),
                account_kind: AccountKind::Bank,
                account: AccountFields::new("A", 1_000_000, D),
            })
            .unwrap();
        let seed = ledger
            .list_categories()
            .unwrap()
            .into_iter()
            .find(|c| c.name == "Biaya admin")
            .unwrap()
            .id;
        assert_eq!(ledger.default_fee_category().unwrap(), Some(seed));

        let b = ledger
            .create_account(AccountKind::Bank, &AccountFields::new("B", 0, D))
            .unwrap();
        let other = ledger
            .create_category(CategoryKind::Expense, &CategoryFields::named("Biaya lain"))
            .unwrap();
        ledger
            .create_txn(&form(FormBody::Transfer {
                from: a,
                to: b,
                amount: 10_000,
                fee: Some(Fee {
                    amount: 2_500,
                    category_id: other,
                }),
            }))
            .unwrap();
        assert_eq!(ledger.default_fee_category().unwrap(), Some(other));
        ledger.archive_category(other).unwrap();
        assert_eq!(ledger.default_fee_category().unwrap(), Some(seed));
    }

    #[test]
    fn suggestions_payees_tags_last_account() {
        let mut f = fx();
        assert_eq!(f.ledger.last_used_account().unwrap(), None);
        let mut input = expense(f.b, 1, vec![line(f.kopi, 1)]);
        input.payee = Some("Kopi Kenangan".into());
        f.ledger.create_txn(&input).unwrap();
        assert_eq!(f.ledger.last_used_account().unwrap(), Some(f.b));
        assert_eq!(
            f.ledger.search_payees("kenang").unwrap(),
            vec!["Kopi Kenangan"]
        );
        assert!(f.ledger.search_payees("zzz").unwrap().is_empty());
    }
}
