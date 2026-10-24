//! Benchmark Fase 8a: skenario S1–S6 pada DB file 50.000 transaksi, plaintext dan SQLCipher.
//!
//! `cargo test -p nabungo-app --release --test perf -- --ignored --nocapture`

use std::path::Path;
use std::time::Instant;

use nabungo_app::fixture::{self, FixtureSpec, FixtureSummary};
use nabungo_app::{
    AccountKind, CategoryKind, DbKey, FormBody, Ledger, OffsetDateTime, SplitLine, TxnForm,
    TxnQuery, TxnSort, TxnSortField, local_today,
};
use time::Duration;

const RUNS: usize = 7;
const PERIODS: u8 = 12;

/// Budget (ms) per skenario, K4. `auto_backup` hanya dilaporkan.
const BUDGETS: &[(&str, Option<u128>)] = &[
    ("open", Some(150)),
    ("startup_backend", Some(700)),
    ("auto_backup", None),
    ("save", Some(100)),
    ("list_default", Some(200)),
    ("list_account", Some(200)),
    ("list_search", Some(200)),
    ("list_sort_amount", Some(200)),
    ("list_sort_payee_deep", Some(200)),
    ("cashflow_12", Some(500)),
    ("net_worth_12", Some(500)),
    ("breakdown", Some(500)),
    ("budget_overview", Some(500)),
    ("allocation_overview", Some(500)),
    ("allocation_history_12", Some(500)),
];

type Report<'a> = dyn Fn(&Ledger) + 'a;

#[derive(Debug, Clone, Copy)]
enum Variant {
    Plain,
    Encrypted,
}

/// Median `RUNS` putaran setelah 1 warm-up, dalam mikrodetik.
fn time<F: FnMut()>(mut f: F) -> u128 {
    f();
    let mut samples: Vec<u128> = (0..RUNS)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_micros()
        })
        .collect();
    samples.sort_unstable();
    samples[RUNS / 2]
}

fn open(path: &Path, backups: &Path, key: Option<&DbKey>) -> Ledger {
    Ledger::open_with_backups(path, backups, key, OffsetDateTime::now_utc()).unwrap()
}

/// Set dasbor yang dimuat UI saat startup (`DashboardView` + sidebar).
fn dashboard(ledger: &mut Ledger) {
    let today = local_today();
    ledger.post_auto_due(today);
    ledger.accounts_overview().unwrap();
    ledger.list_categories().unwrap();
    let period = ledger.period_of(today).unwrap();
    ledger
        .list_txns(&TxnQuery {
            limit: 8,
            ..TxnQuery::default()
        })
        .unwrap();
    ledger.budget_overview(period, today).unwrap();
    ledger
        .category_breakdown(period, CategoryKind::Expense)
        .unwrap();
    ledger.allocation_overview().unwrap();
    ledger.cashflow_report(period, PERIODS).unwrap();
    ledger.net_worth_history(period, PERIODS).unwrap();
    ledger.list_rules(today).unwrap();
}

fn page(q: TxnQuery) -> TxnQuery {
    TxnQuery { limit: 100, ..q }
}

fn measure(
    variant: Variant,
    path: &Path,
    key: Option<&DbKey>,
    dir: &Path,
    s: &FixtureSummary,
) -> Vec<(&'static str, u128)> {
    let backups = dir.join(format!("backups-{variant:?}"));
    let mut out = Vec::new();
    out.push(("open", time(|| drop(open(path, &backups, key)))));
    out.push((
        "startup_backend",
        time(|| dashboard(&mut open(path, &backups, key))),
    ));

    let mut ledger = open(path, &backups, key);
    let mut run = 0;
    out.push((
        "auto_backup",
        time(|| {
            run += 1;
            let fresh = dir.join(format!("auto-{variant:?}-{run}"));
            let made = ledger
                .auto_backup_if_due(&fresh, OffsetDateTime::now_utc())
                .unwrap();
            assert!(made.is_some());
        }),
    ));

    let today = local_today();
    let mut day = 0;
    out.push((
        "save",
        time(|| {
            // Tanggal unik per run: tidak ada baris yang identik dengan run sebelumnya.
            day += 1;
            ledger
                .create_txn_with_alerts(&TxnForm {
                    body: FormBody::Expense {
                        account_id: s.busiest_account,
                        amount: 25_000,
                        splits: vec![SplitLine {
                            category_id: s.expense_category,
                            amount: 25_000,
                            memo: None,
                        }],
                    },
                    date: today - Duration::days(day),
                    payee: Some("Kopi Kenangan".into()),
                    note: None,
                    tags: Vec::new(),
                })
                .unwrap();
        }),
    ));

    let lists = [
        ("list_default", page(TxnQuery::default())),
        (
            "list_account",
            page(TxnQuery {
                account_id: Some(s.busiest_account),
                ..TxnQuery::default()
            }),
        ),
        (
            "list_search",
            page(TxnQuery {
                search: Some("kopi".into()),
                ..TxnQuery::default()
            }),
        ),
        (
            "list_sort_amount",
            page(TxnQuery {
                sort: TxnSort {
                    field: TxnSortField::Amount,
                    desc: true,
                },
                ..TxnQuery::default()
            }),
        ),
        (
            "list_sort_payee_deep",
            page(TxnQuery {
                sort: TxnSort {
                    field: TxnSortField::Payee,
                    desc: false,
                },
                offset: 20_000,
                ..TxnQuery::default()
            }),
        ),
    ];
    for (name, q) in lists {
        out.push((name, time(|| drop(ledger.list_txns(&q).unwrap()))));
    }

    let period = ledger.period_of(today).unwrap();
    let reports: [(&str, &Report); 6] = [
        ("cashflow_12", &|l| {
            drop(l.cashflow_report(period, PERIODS).unwrap())
        }),
        ("net_worth_12", &|l| {
            drop(l.net_worth_history(period, PERIODS).unwrap())
        }),
        ("breakdown", &|l| {
            drop(l.category_breakdown(period, CategoryKind::Expense).unwrap())
        }),
        ("budget_overview", &|l| {
            drop(l.budget_overview(period, today).unwrap())
        }),
        ("allocation_overview", &|l| {
            drop(l.allocation_overview().unwrap())
        }),
        ("allocation_history_12", &|l| {
            drop(l.allocation_history(period, PERIODS).unwrap())
        }),
    ];
    for (name, f) in reports {
        out.push((name, time(|| f(&ledger))));
    }
    out
}

fn ms(us: u128) -> String {
    format!("{}.{}", us / 1000, us % 1000 / 100)
}

#[test]
#[ignore = "benchmark; jalankan dengan --release --ignored --nocapture"]
fn perf_50k() {
    let tmp = tempfile::tempdir().unwrap();
    let plain = tmp.path().join("plain.db");
    let enc = tmp.path().join("enc.db");

    let t = Instant::now();
    let summary = {
        let mut ledger = Ledger::open(&plain).unwrap();
        fixture::seed(&mut ledger, &FixtureSpec::standard(local_today())).unwrap()
    };
    let key = DbKey::generate();
    nabungo_storage::convert_file(&plain, None, &enc, Some(&key)).unwrap();
    eprintln!(
        "fixture: {} transaksi ({} terhapus), {} akun, {:?}",
        summary.txns,
        summary.deleted,
        summary.accounts.len(),
        t.elapsed()
    );
    // Akun tersibuk bukan liability, agar `list_account` mengukur saldo berjalan biasa.
    assert_ne!(
        Ledger::open(&plain)
            .unwrap()
            .accounts_overview()
            .unwrap()
            .accounts
            .iter()
            .find(|a| a.id == summary.busiest_account)
            .unwrap()
            .kind,
        AccountKind::CreditCard
    );

    let plain_ms = measure(Variant::Plain, &plain, None, tmp.path(), &summary);
    let enc_ms = measure(Variant::Encrypted, &enc, Some(&key), tmp.path(), &summary);

    println!("\n| skenario | plaintext ms | sqlcipher ms | budget ms |");
    println!("|---|---:|---:|---:|");
    let mut over = Vec::new();
    for (&(name, budget), (&(p_name, p), &(e_name, e))) in
        BUDGETS.iter().zip(plain_ms.iter().zip(&enc_ms))
    {
        assert_eq!((name, name), (p_name, e_name));
        let limit = budget.map_or("—".to_string(), |b| b.to_string());
        println!("| `{name}` | {} | {} | {limit} |", ms(p), ms(e));
        if let Some(b) = budget {
            for (variant, us) in [(Variant::Plain, p), (Variant::Encrypted, e)] {
                if us > b * 1000 {
                    over.push(format!("{name} {variant:?} {} ms > {b} ms", ms(us)));
                }
            }
        }
    }
    assert!(over.is_empty(), "melebihi budget: {over:?}");
}
