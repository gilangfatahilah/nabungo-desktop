//! Use case akun & account group (ACC-01..03).

use nabungo_core::{
    AccountKind, AccountRef, CurrencyCode, NetWorth, PostingDraft, TxnDraft, TxnKind, ValidTxn,
    net_worth, normalize_name, validate_credit_limit, validate_statement_day,
};
use nabungo_storage::account::{self, AccountPatch, AccountRow, NewAccount};
use nabungo_storage::account_group::{self, AccountGroupRow};
use nabungo_storage::contact::{self, ContactRow};
use nabungo_storage::rusqlite::{Connection, Transaction};
use nabungo_storage::{now_utc, txn};
use time::Date;
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};

/// Isian form akun. Jenis akun terpisah karena tidak bisa diubah setelah dibuat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountFields {
    pub name: String,
    pub group_id: Option<Uuid>,
    pub include_in_net_worth: bool,
    pub credit_limit: Option<i64>,
    pub statement_day: Option<u8>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub opening_balance: i64,
    pub opening_date: Date,
    /// Nama kontak; hanya dipakai untuk `receivable` (piutang/hutang), jenis lain diabaikan.
    pub contact: Option<String>,
}

impl AccountFields {
    /// Tanpa group, ikut net worth, tanpa atribut opsional.
    pub fn new(name: impl Into<String>, opening_balance: i64, opening_date: Date) -> Self {
        Self {
            name: name.into(),
            group_id: None,
            include_in_net_worth: true,
            credit_limit: None,
            statement_day: None,
            icon: None,
            color: None,
            opening_balance,
            opening_date,
            contact: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountsOverview {
    pub groups: Vec<AccountGroupRow>,
    pub accounts: Vec<AccountRow>,
    pub net_worth: NetWorth,
}

/// Validasi isian form akun → patch storage. Limit & tanggal tagihan hanya untuk kartu kredit;
/// kontak hanya untuk piutang/hutang (dibuat bila belum ada, tanpa membedakan huruf).
fn to_patch(
    tx: &Transaction,
    kind: AccountKind,
    f: &AccountFields,
    now: &str,
) -> Result<AccountPatch, AppError> {
    let conn: &Connection = tx;
    let name = normalize_name(&f.name)?;
    let (credit_limit, statement_day) = if kind == AccountKind::CreditCard {
        (
            validate_credit_limit(f.credit_limit)?,
            validate_statement_day(f.statement_day)?,
        )
    } else {
        (None, None)
    };
    if let Some(g) = f.group_id
        && !account_group::group_exists(conn, g)?
    {
        return Err(AppError::NotFound("account_group"));
    }
    Ok(AccountPatch {
        name,
        group_id: f.group_id,
        include_in_net_worth: f.include_in_net_worth,
        credit_limit,
        statement_day,
        icon: f.icon.clone(),
        color: f.color.clone(),
        contact_id: match f.contact.as_deref().map(str::trim) {
            Some(c) if kind == AccountKind::Receivable && !c.is_empty() => {
                Some(contact::ensure_contact(tx, &normalize_name(c)?, now)?)
            }
            _ => None,
        },
    })
}

/// Catat saldo awal ≠ 0 sebagai transaksi `opening`.
fn insert_opening(
    tx: &Transaction,
    account: AccountRef,
    f: &AccountFields,
    now: &str,
) -> Result<(), AppError> {
    if f.opening_balance == 0 {
        return Ok(());
    }
    let draft = TxnDraft {
        kind: TxnKind::Opening,
        date: f.opening_date,
        note: None,
        postings: vec![PostingDraft {
            account,
            category: None,
            amount: f.opening_balance,
            memo: None,
        }],
    };
    txn::insert_txn(tx, &ValidTxn::new(draft)?, None, now)?;
    Ok(())
}

/// Buat akun IDR di dalam transaksi milik pemanggil (dipakai juga oleh onboarding).
pub(crate) fn insert_account_tx(
    tx: &Transaction,
    kind: AccountKind,
    f: &AccountFields,
    now: &str,
) -> Result<Uuid, AppError> {
    let p = to_patch(tx, kind, f, now)?;
    let new = NewAccount {
        name: p.name,
        kind,
        currency: CurrencyCode::IDR,
        group_id: p.group_id,
        include_in_net_worth: p.include_in_net_worth,
        credit_limit: p.credit_limit,
        statement_day: p.statement_day,
        icon: p.icon,
        color: p.color,
        contact_id: p.contact_id,
    };
    let id = account::insert_account(tx, &new, now)?;
    let account = AccountRef {
        id,
        kind,
        currency: CurrencyCode::IDR,
    };
    insert_opening(tx, account, f, now)?;
    Ok(id)
}

fn found(changed: bool, what: &'static str) -> Result<(), AppError> {
    if changed {
        Ok(())
    } else {
        Err(AppError::NotFound(what))
    }
}

impl Ledger {
    pub fn create_account(
        &mut self,
        kind: AccountKind,
        fields: &AccountFields,
    ) -> Result<Uuid, AppError> {
        let tx = self.conn.transaction()?;
        let id = insert_account_tx(&tx, kind, fields, &now_utc())?;
        tx.commit()?;
        Ok(id)
    }

    /// Ubah akun; saldo awal ditulis ulang (dibuat, diganti, atau dihapus bila 0) hanya bila
    /// nominal/tanggalnya berubah, agar tanda reconciled-nya tidak hilang karena ganti nama dsb.
    pub fn update_account(&mut self, id: Uuid, fields: &AccountFields) -> Result<(), AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        let account = account::get_account_ref(&tx, id)?
            .ok_or(AppError::NotFound("account"))?
            .value;
        let patch = to_patch(&tx, account.kind, fields, &now)?;
        account::update_account(&tx, id, &patch, &now)?;
        let wanted =
            (fields.opening_balance != 0).then_some((fields.opening_balance, fields.opening_date));
        if account::opening_of(&tx, id)? != wanted {
            account::delete_opening(&tx, id)?;
            insert_opening(&tx, account, fields, &now)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn archive_account(&mut self, id: Uuid) -> Result<(), AppError> {
        self.set_archived(id, true)
    }

    pub fn unarchive_account(&mut self, id: Uuid) -> Result<(), AppError> {
        self.set_archived(id, false)
    }

    fn set_archived(&mut self, id: Uuid, archived: bool) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        found(
            account::set_account_archived(&tx, id, archived, &now_utc())?,
            "account",
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Hanya akun tanpa transaksi selain saldo awal; selain itu `InUse` (tawarkan arsip).
    pub fn delete_account(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        if account::get_account_ref(&tx, id)?.is_none() {
            return Err(AppError::NotFound("account"));
        }
        if account::has_non_opening_postings(&tx, id)? {
            return Err(AppError::InUse("account"));
        }
        account::delete_account(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Pindahkan `ordered` ke `group_id` dengan urutan sesuai indeks. Atomik.
    pub fn reorder_accounts(
        &mut self,
        group_id: Option<Uuid>,
        ordered: &[Uuid],
    ) -> Result<(), AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        if let Some(g) = group_id
            && !account_group::group_exists(&tx, g)?
        {
            return Err(AppError::NotFound("account_group"));
        }
        for (i, &id) in ordered.iter().enumerate() {
            let pos = i64::try_from(i).unwrap_or(i64::MAX);
            found(
                account::set_account_position(&tx, id, group_id, pos, &now)?,
                "account",
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn create_group(&mut self, name: &str) -> Result<Uuid, AppError> {
        let name = normalize_name(name)?;
        let tx = self.conn.transaction()?;
        let id = account_group::insert_group(&tx, &name)
            .map_err(AppError::duplicate_as("account_group"))?;
        tx.commit()?;
        Ok(id)
    }

    pub fn rename_group(&mut self, id: Uuid, name: &str) -> Result<(), AppError> {
        let name = normalize_name(name)?;
        let tx = self.conn.transaction()?;
        let changed = account_group::rename_group(&tx, id, &name)
            .map_err(AppError::duplicate_as("account_group"))?;
        found(changed, "account_group")?;
        tx.commit()?;
        Ok(())
    }

    /// Akun di dalamnya pindah ke "tanpa grup".
    pub fn delete_group(&mut self, id: Uuid) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        found(account_group::delete_group(&tx, id)?, "account_group")?;
        tx.commit()?;
        Ok(())
    }

    pub fn reorder_groups(&mut self, ordered: &[Uuid]) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        for (i, &id) in ordered.iter().enumerate() {
            let pos = i64::try_from(i).unwrap_or(i64::MAX);
            found(
                account_group::set_group_order(&tx, id, pos)?,
                "account_group",
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Group, akun (termasuk diarsip) beserta saldo, dan net worth akun
    /// `include_in_net_worth` — arsip tidak mengeluarkan akun dari net worth.
    /// Kontak untuk saran nama di form akun piutang/hutang.
    pub fn list_contacts(&self) -> Result<Vec<ContactRow>, AppError> {
        Ok(contact::list_contacts(&self.conn)?)
    }

    pub fn accounts_overview(&self) -> Result<AccountsOverview, AppError> {
        let groups = account_group::list_groups(&self.conn)?;
        let accounts = account::list_accounts(&self.conn)?;
        let net_worth = net_worth(
            accounts
                .iter()
                .filter(|a| a.include_in_net_worth)
                .map(|a| (a.kind, a.balance)),
        )?;
        Ok(AccountsOverview {
            groups,
            accounts,
            net_worth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppError, Ledger, TxnInput};
    use nabungo_core::{CatalogError, CategoryKind};
    use time::macros::date;

    const D: Date = date!(2026 - 10 - 12);

    fn fields(name: &str, opening: i64) -> AccountFields {
        AccountFields::new(name, opening, D)
    }

    fn find(ledger: &Ledger, id: Uuid) -> AccountRow {
        ledger
            .accounts_overview()
            .unwrap()
            .accounts
            .into_iter()
            .find(|a| a.id == id)
            .unwrap()
    }

    fn opening_count(ledger: &Ledger) -> i64 {
        ledger
            .conn
            .query_row("SELECT count(*) FROM txn WHERE kind = 'opening'", [], |r| {
                r.get(0)
            })
            .unwrap()
    }

    fn receivable(ledger: &mut Ledger, name: &str, contact: Option<&str>, opening: i64) -> Uuid {
        let mut f = fields(name, opening);
        f.contact = contact.map(str::to_owned);
        ledger.create_account(AccountKind::Receivable, &f).unwrap()
    }

    fn contact_names(ledger: &Ledger) -> Vec<String> {
        ledger
            .list_contacts()
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect()
    }

    #[test]
    fn receivable_contact_created_and_listed() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = receivable(&mut ledger, "Pinjaman Budi", Some(" Budi "), 0);
        assert_eq!(find(&ledger, id).contact.as_deref(), Some("Budi"));
        assert_eq!(contact_names(&ledger), ["Budi"]);
    }

    #[test]
    fn contact_reused_case_insensitively() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        receivable(&mut ledger, "Pinjaman Budi", Some("Budi"), 0);
        let second = receivable(&mut ledger, "Utang ke Budi", Some("budi"), 0);
        assert_eq!(contact_names(&ledger), ["Budi"]);
        assert_eq!(find(&ledger, second).contact.as_deref(), Some("Budi"));
    }

    #[test]
    fn contact_ignored_for_non_receivable() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut f = fields("BCA", 0);
        f.contact = Some("Budi".into());
        let id = ledger.create_account(AccountKind::Bank, &f).unwrap();
        assert_eq!(find(&ledger, id).contact, None);
        assert!(contact_names(&ledger).is_empty());
    }

    #[test]
    fn empty_contact_is_none_and_long_rejected() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = receivable(&mut ledger, "Pinjaman", Some("  "), 0);
        assert_eq!(find(&ledger, id).contact, None);
        let mut f = fields("Pinjaman 2", 0);
        f.contact = Some("x".repeat(nabungo_core::MAX_NAME_LEN + 1));
        assert!(matches!(
            ledger.create_account(AccountKind::Receivable, &f),
            Err(AppError::Catalog(CatalogError::NameTooLong))
        ));
        let mut patch = fields("Pinjaman", 0);
        patch.contact = Some("Ani".into());
        ledger.update_account(id, &patch).unwrap();
        assert_eq!(find(&ledger, id).contact.as_deref(), Some("Ani"));
    }

    #[test]
    fn receivable_payable_opening_counts_in_net_worth() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let before = ledger.accounts_overview().unwrap().net_worth;
        receivable(&mut ledger, "Utang ke Ani", Some("Ani"), -50_000);
        let after = ledger.accounts_overview().unwrap().net_worth;
        assert_eq!(after.liabilities, before.liabilities + 50_000);
        assert_eq!(after.assets, before.assets);
    }

    #[test]
    fn create_with_attributes_and_opening() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut f = fields("Kartu", -2_000_000);
        f.credit_limit = Some(10_000_000);
        f.statement_day = Some(17);
        let id = ledger.create_account(AccountKind::CreditCard, &f).unwrap();
        let a = find(&ledger, id);
        assert_eq!(a.kind, AccountKind::CreditCard);
        assert_eq!(a.credit_limit, Some(10_000_000));
        assert_eq!(a.statement_day, Some(17));
        assert_eq!(a.balance, -2_000_000);
        let nw = ledger.accounts_overview().unwrap().net_worth;
        assert_eq!(nw.liabilities, 2_000_000);
        assert_eq!(nw.net, -2_000_000);
    }

    #[test]
    fn credit_fields_cleared_for_non_credit_card() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut f = fields("BCA", 0);
        f.credit_limit = Some(1);
        f.statement_day = Some(5);
        let id = ledger.create_account(AccountKind::Bank, &f).unwrap();
        let a = find(&ledger, id);
        assert_eq!(a.credit_limit, None);
        assert_eq!(a.statement_day, None);
    }

    #[test]
    fn name_is_normalized_and_validated() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = ledger
            .create_account(AccountKind::Bank, &fields("  BCA  ", 0))
            .unwrap();
        assert_eq!(find(&ledger, id).name, "BCA");
        assert!(matches!(
            ledger.create_account(AccountKind::Bank, &fields("   ", 0)),
            Err(AppError::Catalog(CatalogError::EmptyName))
        ));
        let mut f = fields("Kartu", 0);
        f.statement_day = Some(32);
        assert!(matches!(
            ledger.create_account(AccountKind::CreditCard, &f),
            Err(AppError::Catalog(CatalogError::InvalidStatementDay(32)))
        ));
    }

    #[test]
    fn edit_opening_balance() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = ledger
            .create_account(AccountKind::Bank, &fields("BCA", 100))
            .unwrap();
        let mut f = fields("BCA", 250);
        f.opening_date = date!(2026 - 09 - 01);
        ledger.update_account(id, &f).unwrap();
        let a = find(&ledger, id);
        assert_eq!(a.balance, 250);
        assert_eq!(a.opening.unwrap().date, date!(2026 - 09 - 01));
        assert_eq!(opening_count(&ledger), 1);

        ledger.update_account(id, &fields("BCA", 0)).unwrap();
        assert_eq!(find(&ledger, id).balance, 0);
        assert_eq!(find(&ledger, id).opening, None);
        assert_eq!(opening_count(&ledger), 0);

        ledger.update_account(id, &fields("BCA", 75)).unwrap();
        assert_eq!(find(&ledger, id).balance, 75);
        assert_eq!(opening_count(&ledger), 1);

        assert!(matches!(
            ledger.update_account(Uuid::now_v7(), &fields("X", 0)),
            Err(AppError::NotFound("account"))
        ));
    }

    #[test]
    fn archive_keeps_net_worth() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = ledger
            .create_account(AccountKind::Bank, &fields("BCA", 1_000_000))
            .unwrap();
        ledger.archive_account(id).unwrap();
        let o = ledger.accounts_overview().unwrap();
        assert_eq!(o.net_worth.net, 1_000_000);
        assert!(o.accounts[0].archived);
        assert!(matches!(
            ledger.record(&TxnInput::single(TxnKind::Adjustment, D, id, None, 1)),
            Err(AppError::Archived("account"))
        ));
        ledger.unarchive_account(id).unwrap();
        assert!(!find(&ledger, id).archived);
    }

    #[test]
    fn excluded_account_not_in_net_worth() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger
            .create_account(AccountKind::Bank, &fields("BCA", 500_000))
            .unwrap();
        let mut f = fields("KPR", -300_000);
        f.include_in_net_worth = false;
        ledger.create_account(AccountKind::Loan, &f).unwrap();
        let nw = ledger.accounts_overview().unwrap().net_worth;
        assert_eq!(nw.assets, 500_000);
        assert_eq!(nw.liabilities, 0);
        assert_eq!(nw.net, 500_000);
    }

    #[test]
    fn delete_unused_account() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = ledger
            .create_account(AccountKind::Bank, &fields("BCA", 1_000))
            .unwrap();
        ledger.delete_account(id).unwrap();
        assert!(ledger.accounts_overview().unwrap().accounts.is_empty());
        assert_eq!(opening_count(&ledger), 0);
        assert!(matches!(
            ledger.delete_account(id),
            Err(AppError::NotFound("account"))
        ));
    }

    #[test]
    fn delete_used_account_is_in_use() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let id = ledger
            .create_account(AccountKind::Bank, &fields("BCA", 0))
            .unwrap();
        let food = ledger
            .create_category(
                CategoryKind::Expense,
                &crate::CategoryFields::named("Makan"),
            )
            .unwrap();
        let txn = ledger
            .record(&TxnInput::single(
                TxnKind::Expense,
                D,
                id,
                Some(food),
                -5_000,
            ))
            .unwrap();
        assert!(matches!(
            ledger.delete_account(id),
            Err(AppError::InUse("account"))
        ));
        ledger.delete_txn(txn).unwrap();
        assert!(matches!(
            ledger.delete_account(id),
            Err(AppError::InUse("account"))
        ));
    }

    #[test]
    fn reorder_across_groups() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let g1 = ledger.create_group("Harian").unwrap();
        let g2 = ledger.create_group("Tabungan").unwrap();
        let mut f = fields("A", 0);
        f.group_id = Some(g1);
        let a = ledger.create_account(AccountKind::Bank, &f).unwrap();
        f.name = "B".into();
        let b = ledger.create_account(AccountKind::Bank, &f).unwrap();

        ledger.reorder_accounts(Some(g2), &[b, a]).unwrap();
        let (ra, rb) = (find(&ledger, a), find(&ledger, b));
        assert_eq!((ra.group_id, ra.sort_order), (Some(g2), 1));
        assert_eq!((rb.group_id, rb.sort_order), (Some(g2), 0));

        assert!(matches!(
            ledger.reorder_accounts(None, &[a, Uuid::now_v7()]),
            Err(AppError::NotFound("account"))
        ));
        let ra = find(&ledger, a);
        assert_eq!((ra.group_id, ra.sort_order), (Some(g2), 1));

        ledger.reorder_groups(&[g2, g1]).unwrap();
        let groups = ledger.accounts_overview().unwrap().groups;
        assert_eq!(groups[0].id, g2);
        assert_eq!(groups[1].id, g1);
    }

    #[test]
    fn group_duplicate_and_delete() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let g = ledger.create_group("Harian").unwrap();
        assert!(matches!(
            ledger.create_group(" harian "),
            Err(AppError::Duplicate("account_group"))
        ));
        let t = ledger.create_group("Tabungan").unwrap();
        assert!(matches!(
            ledger.rename_group(t, "HARIAN"),
            Err(AppError::Duplicate("account_group"))
        ));
        ledger.rename_group(t, "Investasi").unwrap();

        let mut f = fields("BCA", 0);
        f.group_id = Some(g);
        let a = ledger.create_account(AccountKind::Bank, &f).unwrap();
        ledger.delete_group(g).unwrap();
        assert_eq!(find(&ledger, a).group_id, None);

        f.group_id = Some(Uuid::now_v7());
        assert!(matches!(
            ledger.create_account(AccountKind::Bank, &f),
            Err(AppError::NotFound("account_group"))
        ));
        assert!(matches!(
            ledger.delete_group(g),
            Err(AppError::NotFound("account_group"))
        ));
    }
}
