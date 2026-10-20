//! Exit criterion Fase 6a: proses yang di-kill di tengah tulis tidak merusak DB maupun saldo.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use nabungo_app::{AccountFields, Ledger, TxnInput, Uuid};
use nabungo_core::AccountKind;
use time::macros::date;

const ENV_DB: &str = "NABUNGO_CRASH_DB";
const TOTAL: i64 = 1_000_000;

fn account(ledger: &Ledger, name: &str) -> Uuid {
    ledger
        .accounts_overview()
        .unwrap()
        .accounts
        .into_iter()
        .find(|a| a.name == name)
        .unwrap()
        .id
}

/// Proses anak: transfer 1 rupiah A → B tanpa henti sampai di-kill.
#[test]
#[ignore = "dijalankan sebagai proses anak oleh kill_during_writes_keeps_balances"]
fn crash_child() {
    let Some(db) = std::env::var_os(ENV_DB) else {
        return;
    };
    let mut ledger = Ledger::open(Path::new(&db)).unwrap();
    let (a, b) = (account(&ledger, "A"), account(&ledger, "B"));
    let input = TxnInput::transfer(date!(2026 - 10 - 20), a, b, 1);
    loop {
        ledger.record(&input).unwrap();
    }
}

#[test]
fn kill_during_writes_keeps_balances() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("nabungo.db");
    let (a, b) = {
        let mut l = Ledger::open(&db).unwrap();
        let d = date!(2026 - 10 - 01);
        let a = l
            .create_account(AccountKind::Bank, &AccountFields::new("A", TOTAL, d))
            .unwrap();
        let b = l
            .create_account(AccountKind::Bank, &AccountFields::new("B", 0, d))
            .unwrap();
        (a, b)
    };

    let exe = std::env::current_exe().unwrap();
    let mut last_b = 0;
    for round in 0..5 {
        let mut child = Command::new(&exe)
            .args(["--exact", "crash_child", "--include-ignored", "--nocapture"])
            .env(ENV_DB, &db)
            .spawn()
            .unwrap();
        let reader = Ledger::open(&db).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while reader.balance(b).unwrap() <= last_b {
            assert!(
                Instant::now() < deadline,
                "round {round}: child made no progress"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(reader);
        child.kill().unwrap();
        child.wait().unwrap();

        let ledger = Ledger::open(&db).unwrap();
        assert!(
            ledger.integrity_check().unwrap().is_empty(),
            "round {round}"
        );
        let (bal_a, bal_b) = (ledger.balance(a).unwrap(), ledger.balance(b).unwrap());
        assert_eq!(bal_a + bal_b, TOTAL, "round {round}");
        assert!(bal_b > last_b, "round {round}: {bal_b} <= {last_b}");
        last_b = bal_b;
    }
}
