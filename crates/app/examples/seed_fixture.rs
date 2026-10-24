//! Isi folder data baru dengan fixture benchmark, untuk mengukur app sungguhan.
//!
//! `cargo run -p nabungo-app --release --example seed_fixture -- <dir> [--txns N] [--password P]`
//!
//! Dengan `--password`, DB dienkripsi lewat jalur yang sama dengan app (`nabungo.key` valid
//! untuk layar kunci) dan recovery key dicetak.

use std::path::PathBuf;
use std::process::ExitCode;

use nabungo_app::fixture::{self, FixtureSpec};
use nabungo_app::vault::KdfParams;
use nabungo_app::{Ledger, local_today};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut dir = None;
    let mut spec = FixtureSpec::standard(local_today());
    let mut password = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--txns" => match args.next().and_then(|n| n.parse().ok()) {
                Some(n) => spec.txns = n,
                None => return usage(),
            },
            "--password" => match args.next() {
                Some(p) => password = Some(p),
                None => return usage(),
            },
            _ if dir.is_none() && !arg.starts_with("--") => dir = Some(PathBuf::from(arg)),
            _ => return usage(),
        }
    }
    let Some(dir) = dir else {
        return usage();
    };

    let db = dir.join("nabungo.db");
    if db.exists() {
        eprintln!("{} sudah ada; pakai folder kosong", db.display());
        return ExitCode::FAILURE;
    }
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("gagal membuat {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    let run = || -> Result<(), nabungo_app::AppError> {
        let mut ledger = Ledger::open(&db)?;
        let summary = fixture::seed(&mut ledger, &spec)?;
        println!(
            "{}: {} transaksi ({} terhapus)",
            db.display(),
            summary.txns,
            summary.deleted
        );
        if let Some(password) = &password {
            let recovery =
                ledger.enable_encryption(password, &dir.join("backups"), KdfParams::DEFAULT)?;
            println!("terenkripsi; recovery key: {recovery}");
        }
        Ok(())
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gagal: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("pakai: seed_fixture <dir> [--txns N] [--password P]");
    ExitCode::FAILURE
}
