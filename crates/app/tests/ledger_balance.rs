use nabungo_app::{Ledger, TxnInput};
use nabungo_core::{AccountKind, CategoryKind, CurrencyCode, TxnKind};
use nabungo_storage::account::NewAccount;
use proptest::prelude::*;
use time::macros::date;

#[derive(Debug, Clone)]
enum Op {
    Income(usize, i64),
    /// amount > 0 = pengeluaran, < 0 = refund
    Expense(usize, i64),
    Transfer(usize, usize, i64),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..3usize, 1i64..=10_000_000).prop_map(|(a, x)| Op::Income(a, x)),
        (0..3usize, -1_000_000i64..=10_000_000)
            .prop_filter("non-zero", |(_, x)| *x != 0)
            .prop_map(|(a, x)| Op::Expense(a, x)),
        (0..3usize, 0..3usize, 1i64..=10_000_000)
            .prop_filter("distinct accounts", |(a, b, _)| a != b)
            .prop_map(|(a, b, x)| Op::Transfer(a, b, x)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn balances_match_model(
        openings in prop::array::uniform3(-5_000_000i64..=5_000_000),
        ops in prop::collection::vec(op(), 0..300),
    ) {
        let d = date!(2026 - 10 - 12);
        let mut ledger = Ledger::open_in_memory().unwrap();
        let accounts: Vec<_> = openings
            .iter()
            .enumerate()
            .map(|(i, &o)| {
                let new = NewAccount { name: format!("A{i}"), kind: AccountKind::Bank, currency: CurrencyCode::IDR };
                ledger.create_account(new, o, d).unwrap()
            })
            .collect();
        let salary = ledger.create_category(CategoryKind::Income, "Gaji").unwrap();
        let food = ledger.create_category(CategoryKind::Expense, "Makan").unwrap();

        let mut model = openings;
        let mut transfers_only_total = model.iter().sum::<i64>();
        for op in &ops {
            let input = match *op {
                Op::Income(a, x) => {
                    model[a] += x;
                    transfers_only_total += x;
                    TxnInput::single(TxnKind::Income, d, accounts[a], Some(salary), x)
                }
                Op::Expense(a, x) => {
                    model[a] -= x;
                    transfers_only_total -= x;
                    TxnInput::single(TxnKind::Expense, d, accounts[a], Some(food), -x)
                }
                Op::Transfer(a, b, x) => {
                    model[a] -= x;
                    model[b] += x;
                    TxnInput::transfer(d, accounts[a], accounts[b], x)
                }
            };
            ledger.record(&input).unwrap();
        }

        for (i, id) in accounts.iter().enumerate() {
            prop_assert_eq!(ledger.balance(*id).unwrap(), model[i]);
        }
        // Transfer tidak mengubah total: total = opening + income − expense.
        let total: i64 = accounts.iter().map(|id| ledger.balance(*id).unwrap()).sum();
        prop_assert_eq!(total, transfers_only_total);
    }
}
