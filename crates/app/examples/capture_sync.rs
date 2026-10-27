//! Sync satu label Gmail ke folder data lalu cetak antrean review (Fase 9a, belum ada UI).
//!
//! `cargo run -p nabungo-app --example capture_sync -- <data-dir> [--mailbox Finance]`
//!
//! Env: `NABUNGO_IMAP_USER`, `GOOGLE_APP_PASSWORD` (App Password Gmail), dan
//! `NABUNGO_DB_PASSWORD` bila DB terenkripsi. Akses read-only (EXAMINE + `BODY.PEEK[]`);
//! password tidak pernah dicetak.

use std::path::PathBuf;
use std::process::ExitCode;

use nabungo_app::mail::ImapSource;
use nabungo_app::{CaptureStatus, Ledger, OffsetDateTime, format_date, security, sync_mailbox};
use time::UtcOffset;

const HOST: &str = "imap.gmail.com";
const PORT: u16 = 993;
/// Fallback bila offset lokal tidak bisa dibaca (WIB).
const FALLBACK_OFFSET: i8 = 7;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut dir = None;
    let mut mailbox = "Finance".to_owned();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mailbox" => match args.next() {
                Some(m) => mailbox = m,
                None => return usage(),
            },
            _ if dir.is_none() && !arg.starts_with("--") => dir = Some(PathBuf::from(arg)),
            _ => return usage(),
        }
    }
    let Some(dir) = dir else {
        return usage();
    };
    let (Ok(user), Ok(password)) = (
        std::env::var("NABUNGO_IMAP_USER"),
        std::env::var("GOOGLE_APP_PASSWORD"),
    ) else {
        eprintln!("NABUNGO_IMAP_USER dan GOOGLE_APP_PASSWORD wajib diisi");
        return ExitCode::FAILURE;
    };
    match run(&dir, &mailbox, &user, &password) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gagal: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    dir: &std::path::Path,
    mailbox: &str,
    user: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let db = dir.join("nabungo.db");
    let key = if dir.join("nabungo.key").exists() {
        let pw = std::env::var("NABUNGO_DB_PASSWORD")
            .map_err(|_| "DB terenkripsi: isi NABUNGO_DB_PASSWORD")?;
        Some(security::unlock_key(&db, &pw)?)
    } else {
        None
    };
    let mut ledger = Ledger::open_with_backups(
        &db,
        &dir.join("backups"),
        key.as_ref(),
        OffsetDateTime::now_utc(),
    )?;
    let offset = UtcOffset::current_local_offset()
        .unwrap_or_else(|_| UtcOffset::from_hms(FALLBACK_OFFSET, 0, 0).expect("offset valid"));

    eprintln!("connect {HOST}…");
    let mut source = ImapSource::connect(HOST, PORT, user, password)?;
    let summary = sync_mailbox(&mut ledger, &mut source, mailbox, offset)?;
    drop(source);
    println!(
        "mailbox={mailbox} new_items={} unparsed={} seen={}",
        summary.new_items, summary.unparsed, summary.seen
    );

    let pending = ledger.list_captures(CaptureStatus::Pending)?;
    println!("pending: {}", pending.len());
    for c in &pending {
        let draft = ledger.capture_draft(c.id)?;
        println!(
            "{}  {:>12}  {:<40}  {}",
            format_date(c.date),
            c.amount,
            c.payee.as_deref().unwrap_or("-"),
            draft.note.as_deref().unwrap_or("-"),
        );
    }
    let unparsed = ledger.list_unparsed()?;
    println!("unparsed: {}", unparsed.len());
    for u in &unparsed {
        println!("{}  {}", u.sender, u.subject);
    }
    Ok(())
}

fn usage() -> ExitCode {
    eprintln!("pakai: capture_sync <data-dir> [--mailbox Finance]");
    ExitCode::FAILURE
}
