//! Fixture bersama untuk test use case Fase 5: ledger hasil onboarding + kategori seed.

use nabungo_core::{AccountKind, Fee, FormBody, PeriodStartDay, SplitLine};
use time::Date;
use uuid::Uuid;

use crate::{AccountFields, Ledger, Locale, OnboardingInput, TxnForm};

pub(crate) struct Kit {
    pub ledger: Ledger,
    pub bca: Uuid,
    pub dompet: Uuid,
}

impl Kit {
    /// Onboarding (locale id) dengan BCA bersaldo awal 1.000.000 per `opening`, plus Dompet kosong.
    pub fn new(start_day: u8, opening: Date) -> Self {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let bca = ledger
            .complete_onboarding(&OnboardingInput {
                locale: Locale::Id,
                period_start_day: PeriodStartDay::new(start_day).unwrap(),
                account_kind: AccountKind::Bank,
                account: AccountFields::new("BCA", 1_000_000, opening),
            })
            .unwrap();
        let dompet = ledger
            .create_account(AccountKind::Cash, &AccountFields::new("Dompet", 0, opening))
            .unwrap();
        Kit {
            ledger,
            bca,
            dompet,
        }
    }

    pub fn cat(&self, name: &str) -> Uuid {
        self.ledger
            .list_categories()
            .unwrap()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("category {name}"))
            .id
    }

    fn form(&mut self, date: Date, body: FormBody) -> Uuid {
        self.ledger
            .create_txn(&TxnForm {
                body,
                date,
                payee: None,
                note: None,
                tags: vec![],
            })
            .unwrap()
    }

    /// `amount` positif = belanja; negatif = refund.
    pub fn expense(&mut self, date: Date, category: &str, amount: i64) -> Uuid {
        let category_id = self.cat(category);
        let account_id = self.bca;
        self.form(
            date,
            FormBody::Expense {
                account_id,
                amount,
                splits: vec![SplitLine {
                    category_id,
                    amount,
                    memo: None,
                }],
            },
        )
    }

    pub fn income(&mut self, date: Date, category: &str, amount: i64) -> Uuid {
        let category_id = self.cat(category);
        let account_id = self.bca;
        self.form(
            date,
            FormBody::Income {
                account_id,
                amount,
                splits: vec![SplitLine {
                    category_id,
                    amount,
                    memo: None,
                }],
            },
        )
    }

    /// BCA → Dompet, dengan biaya opsional di kategori "Biaya admin".
    pub fn transfer(&mut self, date: Date, amount: i64, fee: Option<i64>) -> Uuid {
        let fee = fee.map(|amount| Fee {
            amount,
            category_id: self.cat("Biaya admin"),
        });
        let (from, to) = (self.bca, self.dompet);
        self.form(
            date,
            FormBody::Transfer {
                from,
                to,
                amount,
                fee,
            },
        )
    }
}
