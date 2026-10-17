//! Onboarding pertama kali: bahasa, awal periode budget, akun pertama, kategori default.

use nabungo_core::{AccountKind, PeriodStartDay};
use nabungo_storage::{now_utc, setting};
use uuid::Uuid;

use crate::accounts::insert_account_tx;
use crate::defaults::seed_default_categories;
use crate::settings::{KEY_LOCALE, KEY_PERIOD_START_DAY};
use crate::{AccountFields, AppError, Ledger, Locale};

const KEY_COMPLETED_AT: &str = "onboarding.completed_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingInput {
    pub locale: Locale,
    pub period_start_day: PeriodStartDay,
    pub account_kind: AccountKind,
    pub account: AccountFields,
}

impl Ledger {
    pub fn is_onboarded(&self) -> Result<bool, AppError> {
        Ok(setting::get_setting(&self.conn, KEY_COMPLETED_AT)?.is_some())
    }

    /// Satu transaksi: gagal di langkah mana pun → tidak ada yang tersimpan.
    /// Kategori default hanya di-seed bila belum ada kategori. Mengembalikan ID akun pertama.
    pub fn complete_onboarding(&mut self, input: &OnboardingInput) -> Result<Uuid, AppError> {
        let now = now_utc();
        let tx = self.conn.transaction()?;
        if setting::get_setting(&tx, KEY_COMPLETED_AT)?.is_some() {
            return Err(AppError::AlreadyOnboarded);
        }
        setting::set_setting(&tx, KEY_LOCALE, input.locale.as_str())?;
        setting::set_setting(
            &tx,
            KEY_PERIOD_START_DAY,
            &input.period_start_day.day().to_string(),
        )?;
        seed_default_categories(&tx, input.locale)?;
        let id = insert_account_tx(&tx, input.account_kind, &input.account, &now)?;
        setting::set_setting(&tx, KEY_COMPLETED_AT, &now)?;
        tx.commit()?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CatalogError, CategoryFields};
    use nabungo_core::CategoryKind;
    use time::macros::date;

    fn input(name: &str) -> OnboardingInput {
        OnboardingInput {
            locale: Locale::En,
            period_start_day: PeriodStartDay::new(25).unwrap(),
            account_kind: AccountKind::Bank,
            account: AccountFields::new(name, 1_000_000, date!(2026 - 10 - 17)),
        }
    }

    fn count(ledger: &Ledger, table: &str) -> i64 {
        ledger
            .conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn complete_onboarding_sets_everything() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        assert!(!ledger.is_onboarded().unwrap());
        let acc = ledger.complete_onboarding(&input("BCA")).unwrap();
        assert!(ledger.is_onboarded().unwrap());
        assert_eq!(ledger.ui_settings().unwrap().locale, Locale::En);
        assert_eq!(ledger.period_start_day().unwrap().day(), 25);
        assert!(
            ledger
                .list_categories()
                .unwrap()
                .iter()
                .any(|c| c.name == "Salary")
        );
        assert_eq!(ledger.balance(acc).unwrap(), 1_000_000);
    }

    #[test]
    fn second_call_is_already_onboarded() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger.complete_onboarding(&input("BCA")).unwrap();
        let (cats, accs) = (count(&ledger, "category"), count(&ledger, "account"));
        assert!(matches!(
            ledger.complete_onboarding(&input("Mandiri")),
            Err(AppError::AlreadyOnboarded)
        ));
        assert_eq!(count(&ledger, "category"), cats);
        assert_eq!(count(&ledger, "account"), accs);
    }

    #[test]
    fn failed_onboarding_leaves_nothing() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        assert!(matches!(
            ledger.complete_onboarding(&input("  ")),
            Err(AppError::Catalog(CatalogError::EmptyName))
        ));
        assert!(!ledger.is_onboarded().unwrap());
        assert_eq!(count(&ledger, "category"), 0);
        assert_eq!(count(&ledger, "setting"), 0);
        ledger.complete_onboarding(&input("BCA")).unwrap();
        assert!(ledger.is_onboarded().unwrap());
    }

    #[test]
    fn existing_categories_are_kept() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        ledger
            .create_category(CategoryKind::Expense, &CategoryFields::named("Kopi"))
            .unwrap();
        ledger.complete_onboarding(&input("BCA")).unwrap();
        let names: Vec<_> = ledger
            .list_categories()
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["Kopi"]);
    }
}
