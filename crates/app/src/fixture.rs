//! Data sintetis deterministik untuk benchmark Fase 8a (bukan untuk produksi).
//!
//! Seed yang sama menghasilkan isi yang sama: PRNG SplitMix64 lokal, tanpa jam dinding
//! kecuali timestamp audit. Transaksi ditulis per 1.000 dalam satu SQLite transaction.

use std::collections::HashSet;

use nabungo_core::{AccountKind, CategoryKind, Fee, FormBody, PeriodStartDay, SplitLine, Target};
use nabungo_storage::{now_utc, txn};
use time::{Date, Duration};
use uuid::Uuid;

use crate::allocation::BucketFields;
use crate::ledger::{AppError, Ledger};
use crate::onboarding::OnboardingInput;
use crate::recurring::RuleInput;
use crate::settings::Locale;
use crate::transactions::{TxnForm, form_input, write_txn};
use crate::{AccountFields, Freq};

const PERIODS: i32 = 36;
const BATCH: u32 = 1_000;
const SALARY: i64 = 15_000_000;

pub struct FixtureSpec {
    /// Transaksi user (di luar saldo awal & valuation), termasuk yang dihapus.
    pub txns: u32,
    pub seed: u64,
    /// Tanggal terakhir; 36 periode berakhir di periode tanggal ini.
    pub end: Date,
}

impl FixtureSpec {
    pub fn standard(end: Date) -> Self {
        Self {
            txns: 50_000,
            seed: 42,
            end,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FixtureSummary {
    pub accounts: Vec<Uuid>,
    pub busiest_account: Uuid,
    pub expense_category: Uuid,
    pub txns: u32,
    pub deleted: u32,
}

/// SplitMix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// `0..n`; `n` > 0.
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }

    /// Kelipatan 500 dalam `[lo, hi]`.
    fn amount(&mut self, lo: i64, hi: i64) -> i64 {
        let steps = (hi - lo) / 500 + 1;
        lo + 500 * self.below(steps as u64) as i64
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

const PAYEES: [&str; 40] = [
    "Kopi Kenangan",
    "Janji Jiwa Kopi",
    "Kopi Tuku",
    "Starbucks Kopi",
    "Indomaret",
    "Alfamart",
    "Superindo",
    "Hypermart",
    "Transmart",
    "Pasar Santa",
    "Warung Bu Tini",
    "Sate Khas Senayan",
    "Bakmi GM",
    "Solaria",
    "Hokben",
    "McDonald's",
    "KFC",
    "Pizza Hut",
    "Gojek",
    "Grab",
    "Pertamina",
    "Shell",
    "Transjakarta",
    "KRL Commuter",
    "PLN",
    "PDAM",
    "Telkomsel",
    "IndiHome",
    "Netflix",
    "Spotify",
    "Tokopedia",
    "Shopee",
    "Lazada",
    "Gramedia",
    "Apotek K-24",
    "Guardian",
    "RS Pondok Indah",
    "Kantor",
    "Pak Budi",
    "Arisan RT",
];

const TAGS: [&str; 15] = [
    "kantor",
    "liburan",
    "keluarga",
    "darurat",
    "hadiah",
    "proyek",
    "anak",
    "rumah",
    "mobil",
    "kesehatan",
    "langganan",
    "pendidikan",
    "hobi",
    "reimburse",
    "mudik",
];

pub fn seed(ledger: &mut Ledger, spec: &FixtureSpec) -> Result<FixtureSummary, AppError> {
    let start_day = PeriodStartDay::new(25)?;
    let last = nabungo_core::period_of(spec.end, start_day);
    let mut first = last;
    for _ in 1..PERIODS {
        first = first.prev();
    }
    let start = first.date_range(start_day).0;
    let total_days = (spec.end - start).whole_days() + 1;
    let mut rng = Rng(spec.seed);

    let main = ledger.complete_onboarding(&OnboardingInput {
        locale: Locale::Id,
        period_start_day: start_day,
        account_kind: AccountKind::Bank,
        account: AccountFields::new("BCA", 25_000_000, start),
    })?;
    let mut account = |kind, fields: AccountFields| ledger.create_account(kind, &fields);
    let salary_acc = account(
        AccountKind::Bank,
        AccountFields::new("Mandiri", 5_000_000, start),
    )?;
    let cash = account(
        AccountKind::Cash,
        AccountFields::new("Dompet", 1_000_000, start),
    )?;
    let gopay = account(
        AccountKind::Ewallet,
        AccountFields::new("GoPay", 500_000, start),
    )?;
    let ovo = account(
        AccountKind::Ewallet,
        AccountFields::new("OVO", 300_000, start),
    )?;
    let card = account(
        AccountKind::CreditCard,
        AccountFields {
            credit_limit: Some(30_000_000),
            statement_day: Some(17),
            ..AccountFields::new("Kartu Kredit", 0, start)
        },
    )?;
    let invest = account(
        AccountKind::Investment,
        AccountFields::new("Reksa Dana", 10_000_000, start),
    )?;
    let savings = account(
        AccountKind::Bank,
        AccountFields::new("Tabungan", 20_000_000, start),
    )?;
    let receivable = account(
        AccountKind::Receivable,
        AccountFields {
            contact: Some("Budi".into()),
            ..AccountFields::new("Piutang Budi", 2_000_000, start)
        },
    )?;
    let old = account(
        AccountKind::Cash,
        AccountFields::new("Dompet Lama", 150_000, start),
    )?;
    ledger.archive_account(old)?;
    let accounts = vec![
        main, salary_acc, cash, gopay, ovo, card, invest, savings, receivable, old,
    ];
    // BCA sengaja muncul berkali-kali: akun tersibuk untuk benchmark filter akun.
    let spend = [
        main, main, main, salary_acc, cash, cash, gopay, ovo, card, card,
    ];
    let movers = [
        main, salary_acc, cash, gopay, ovo, card, savings, receivable,
    ];

    let cats = ledger.list_categories()?;
    let leaves = |kind: CategoryKind| -> Vec<Uuid> {
        cats.iter()
            .filter(|c| c.kind == kind && !c.archived)
            .filter(|c| !cats.iter().any(|o| o.parent_id == Some(c.id)))
            .map(|c| c.id)
            .collect()
    };
    let expense = leaves(CategoryKind::Expense);
    let income = leaves(CategoryKind::Income);
    let by_name = |name: &str| {
        cats.iter()
            .find(|c| c.name == name)
            .map(|c| c.id)
            .ok_or(AppError::NotFound("category"))
    };
    let salary_cat = by_name("Gaji")?;
    let fee_cat = by_name("Biaya admin")?;

    seed_plans(ledger, &mut rng, &expense, first, &[main, invest, savings])?;

    let salaries = (PERIODS as u32).min(spec.txns);
    let random = spec.txns - salaries;
    let mut forms = Vec::with_capacity(spec.txns as usize);
    let mut period = first;
    for _ in 0..salaries {
        let date = period.date_range(start_day).0;
        forms.push(TxnForm {
            body: FormBody::Income {
                account_id: salary_acc,
                amount: SALARY,
                splits: vec![line(salary_cat, SALARY)],
            },
            date,
            payee: Some("Kantor".into()),
            note: None,
            tags: Vec::new(),
        });
        period = period.next();
    }
    for i in 0..random {
        let offset = i64::from(i) * total_days / i64::from(random);
        let date = start + Duration::days(offset);
        let body = match rng.below(100) {
            0..72 => {
                let lines = if rng.chance(10) { 2 + rng.below(2) } else { 1 };
                let splits: Vec<_> = (0..lines)
                    .map(|_| line(*rng.pick(&expense), rng.amount(5_000, 500_000)))
                    .collect();
                FormBody::Expense {
                    account_id: *rng.pick(&spend),
                    amount: splits.iter().map(|s| s.amount).sum(),
                    splits,
                }
            }
            72..80 => {
                let amount = rng.amount(50_000, 5_000_000);
                FormBody::Income {
                    account_id: *rng.pick(&movers),
                    amount,
                    splits: vec![line(*rng.pick(&income), amount)],
                }
            }
            80..95 => {
                let from = *rng.pick(&movers);
                let to = loop {
                    let to = *rng.pick(&movers);
                    if to != from {
                        break to;
                    }
                };
                let fee = rng.chance(20).then(|| Fee {
                    amount: *rng.pick(&[2_500, 6_500]),
                    category_id: fee_cat,
                });
                FormBody::Transfer {
                    from,
                    to,
                    amount: rng.amount(10_000, 2_000_000),
                    fee,
                }
            }
            _ => {
                let amount = -rng.amount(5_000, 200_000);
                FormBody::Expense {
                    account_id: *rng.pick(&spend),
                    amount,
                    splits: vec![line(*rng.pick(&expense), amount)],
                }
            }
        };
        let payee = rng.chance(90).then(|| rng.pick(&PAYEES).to_string());
        let tags = if rng.chance(20) {
            (0..1 + rng.below(2))
                .map(|_| rng.pick(&TAGS).to_string())
                .collect()
        } else {
            Vec::new()
        };
        forms.push(TxnForm {
            body,
            date,
            payee,
            note: None,
            tags,
        });
    }

    let mut deleted = 0;
    let none = HashSet::new();
    for chunk in forms.chunks(BATCH as usize) {
        let now = now_utc();
        let tx = ledger.conn.transaction()?;
        for form in chunk {
            let id = write_txn(&tx, None, &form_input(form)?, &none)?;
            if rng.chance(2) {
                txn::soft_delete(&tx, id, &now)?;
                deleted += 1;
            }
        }
        tx.commit()?;
    }

    // Valuation bulanan: nilai naik terus agar selisihnya tidak pernah 0.
    let mut value = 10_000_000;
    let mut period = first;
    for _ in 0..PERIODS {
        value += rng.amount(50_000, 500_000);
        let date = period.last_day(start_day).min(spec.end);
        ledger.record_valuation(invest, date, value)?;
        period = period.next();
    }

    seed_rules(ledger, &mut rng, &expense, main, spec.end)?;

    Ok(FixtureSummary {
        accounts,
        busiest_account: main,
        expense_category: expense[0],
        txns: spec.txns,
        deleted,
    })
}

fn line(category_id: Uuid, amount: i64) -> SplitLine {
    SplitLine {
        category_id,
        amount,
        memo: None,
    }
}

/// Budget 12 kategori expense di semua periode (3 dengan rollover), 6 bucket alokasi
/// (2 level, 1 nominal) beserta pemetaan akun.
fn seed_plans(
    ledger: &mut Ledger,
    rng: &mut Rng,
    expense: &[Uuid],
    first: nabungo_core::BudgetPeriod,
    [main, invest, savings]: &[Uuid; 3],
) -> Result<(), AppError> {
    let budgeted = &expense[..12.min(expense.len())];
    let amounts: Vec<_> = budgeted
        .iter()
        .map(|_| rng.amount(200_000, 3_000_000))
        .collect();
    let mut period = first;
    for _ in 0..PERIODS {
        for (i, (&cat, &amount)) in budgeted.iter().zip(&amounts).enumerate() {
            ledger.set_budget(cat, period, Some(amount))?;
            if i < 3 {
                ledger.set_budget_rollover(cat, period, true)?;
            }
        }
        period = period.next();
    }

    let mut bucket = |name: &str, parent_id, target| {
        ledger.create_bucket(&BucketFields {
            name: name.into(),
            parent_id,
            target,
            color: None,
        })
    };
    let emergency = bucket("Dana Darurat", None, Target::Percent(4_000))?;
    let growth = bucket("Investasi", None, Target::Percent(3_000))?;
    let holiday = bucket("Liburan", None, Target::Amount(5_000_000))?;
    let core = bucket("Utama", Some(emergency), Target::Percent(6_000))?;
    let spare = bucket("Cadangan", Some(emergency), Target::Percent(4_000))?;
    let stocks = bucket("Saham", Some(growth), Target::Percent(10_000))?;
    ledger.set_account_allocation(*savings, &[(core, 6_000), (spare, 4_000)])?;
    ledger.set_account_allocation(*invest, &[(stocks, 10_000)])?;
    ledger.set_account_allocation(*main, &[(holiday, 2_000)])?;
    Ok(())
}

/// 8 transaksi terjadwal tanpa auto-post, mulai setelah `end` agar tidak ada yang tertunda.
fn seed_rules(
    ledger: &mut Ledger,
    rng: &mut Rng,
    expense: &[Uuid],
    account_id: Uuid,
    end: Date,
) -> Result<(), AppError> {
    let freqs = [
        Freq::Monthly,
        Freq::Monthly,
        Freq::Monthly,
        Freq::Monthly,
        Freq::Monthly,
        Freq::Weekly,
        Freq::Yearly,
        Freq::Daily,
    ];
    for (i, freq) in (1..).zip(freqs) {
        let amount = rng.amount(20_000, 1_000_000);
        ledger.create_rule(&RuleInput {
            form: TxnForm {
                body: FormBody::Expense {
                    account_id,
                    amount,
                    splits: vec![line(*rng.pick(expense), amount)],
                },
                date: end + Duration::days(3 * i),
                payee: Some(rng.pick(&PAYEES).to_string()),
                note: None,
                tags: Vec::new(),
            },
            freq,
            interval: 1,
            until: None,
            auto_post: false,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    const END: Date = date!(2026 - 10 - 24);

    fn seeded(txns: u32) -> (Ledger, FixtureSummary) {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let summary = seed(
            &mut ledger,
            &FixtureSpec {
                txns,
                seed: 7,
                end: END,
            },
        )
        .unwrap();
        (ledger, summary)
    }

    fn count(ledger: &Ledger, sql: &str) -> i64 {
        ledger.conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    /// Saldo per nama akun + cashflow 12 periode: isi yang harus sama untuk seed yang sama.
    type Snapshot = (Vec<(String, i64)>, Vec<(i64, i64)>);

    fn snapshot(ledger: &Ledger) -> Snapshot {
        let balances = ledger
            .accounts_overview()
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| (a.name, a.balance))
            .collect();
        let end = ledger.period_of(END).unwrap();
        let flow = ledger
            .cashflow_report(end, 12)
            .unwrap()
            .into_iter()
            .map(|p| (p.income, p.expense))
            .collect();
        (balances, flow)
    }

    /// Titik terakhir riwayat (akhir periode berjalan, tidak ada transaksi sesudah `END`)
    /// sama dengan overview saat ini, yang menghitung saldo lewat jalur lain.
    #[test]
    fn history_end_matches_current_overview() {
        let (l, _) = seeded(2_000);
        let period = l.period_of(END).unwrap();
        let nw = l.net_worth_history(period, 12).unwrap();
        let overview = l.accounts_overview().unwrap();
        assert_eq!(nw.len(), 12);
        let last = nw.last().unwrap();
        assert_eq!(
            (last.assets, last.liabilities, last.net),
            (
                overview.net_worth.assets,
                overview.net_worth.liabilities,
                overview.net_worth.net
            )
        );
        let alloc = l.allocation_overview().unwrap();
        let hist = l.allocation_history(period, 12).unwrap();
        let point = hist.last().unwrap();
        assert_eq!(point.gross, alloc.report.gross);
        let roots: Vec<_> = alloc
            .buckets
            .iter()
            .zip(&alloc.report.buckets)
            .filter(|(b, _)| b.parent_id.is_none())
            .map(|(b, r)| (b.id, r.actual_bp))
            .collect();
        assert_eq!(point.buckets, roots);
    }

    #[test]
    fn fixture_is_deterministic() {
        let (a, _) = seeded(2_000);
        let (b, _) = seeded(2_000);
        assert_eq!(snapshot(&a), snapshot(&b));
    }

    #[test]
    fn fixture_counts_and_mix() {
        let (l, s) = seeded(2_000);
        assert_eq!(s.txns, 2_000);
        assert_eq!(
            count(
                &l,
                "SELECT COUNT(*) FROM txn WHERE kind NOT IN ('opening', 'valuation')"
            ),
            2_000
        );
        assert_eq!(
            count(&l, "SELECT COUNT(*) FROM txn WHERE deleted_at IS NOT NULL"),
            i64::from(s.deleted)
        );
        assert!(s.deleted > 0);
        let at_least_one = [
            // split
            "SELECT COUNT(*) FROM txn t WHERE t.kind = 'expense' AND
               (SELECT COUNT(*) FROM posting p WHERE p.txn_id = t.id AND p.category_id IS NOT NULL) > 1",
            // transfer berbiaya
            "SELECT COUNT(*) FROM txn t WHERE t.kind = 'transfer' AND
               EXISTS (SELECT 1 FROM posting p WHERE p.txn_id = t.id AND p.category_id IS NOT NULL)",
            // refund
            "SELECT COUNT(*) FROM txn t WHERE t.kind = 'expense' AND
               (SELECT SUM(p.amount) FROM posting p WHERE p.txn_id = t.id AND p.category_id IS NOT NULL) > 0",
            "SELECT COUNT(*) FROM txn WHERE kind = 'valuation'",
            "SELECT COUNT(*) FROM budget WHERE rollover = 1",
            "SELECT COUNT(*) FROM allocation_bucket WHERE target_amount IS NOT NULL",
            "SELECT COUNT(*) FROM allocation_bucket WHERE parent_id IS NOT NULL",
            "SELECT COUNT(*) FROM txn_tag",
            "SELECT COUNT(*) FROM account WHERE archived_at IS NOT NULL",
        ];
        for sql in at_least_one {
            assert!(count(&l, sql) > 0, "{sql}");
        }
        assert_eq!(count(&l, "SELECT COUNT(*) FROM recurring_rule"), 8);
        assert_eq!(count(&l, "SELECT COUNT(*) FROM account"), 10);
        assert!(s.accounts.contains(&s.busiest_account));
    }

    #[test]
    fn fixture_deleted_txns_excluded() {
        let (l, _) = seeded(2_000);
        let direct = count(
            &l,
            "SELECT SUM(p.amount) FROM posting p JOIN txn t ON t.id = p.txn_id
             JOIN account a ON a.id = p.account_id
             WHERE t.deleted_at IS NULL AND t.status = 'posted' AND a.include_in_net_worth = 1",
        );
        let with_deleted = count(
            &l,
            "SELECT SUM(p.amount) FROM posting p JOIN account a ON a.id = p.account_id
             WHERE a.include_in_net_worth = 1",
        );
        assert_ne!(
            direct, with_deleted,
            "fixture harus punya transaksi terhapus bernominal"
        );
        assert_eq!(l.accounts_overview().unwrap().net_worth.net, direct);
    }
}
