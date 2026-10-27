//! Spike IMAP (Fase 9a Task 1): login, EXAMINE label, cetak header 10 email terakhir.
//! Env: NABUNGO_IMAP_USER, GOOGLE_APP_PASSWORD, NABUNGO_IMAP_MAILBOX (default "Finance").

use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user = env::var("NABUNGO_IMAP_USER")?;
    let password = env::var("GOOGLE_APP_PASSWORD")?;
    let mailbox = env::var("NABUNGO_IMAP_MAILBOX").unwrap_or_else(|_| "Finance".into());

    let tls = native_tls::TlsConnector::builder().build()?;
    eprintln!("connect…");
    let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)?;
    eprintln!("login…");
    let mut session = client.login(user, password).map_err(|(e, _)| e)?;
    if env::var("NABUNGO_IMAP_LIST").is_ok() {
        for n in session.list(None, Some("*"))?.iter() {
            println!("{}", n.name());
        }
    }
    eprintln!("examine…");
    let mb = session.examine(&mailbox)?;
    println!(
        "mailbox={mailbox} exists={} uid_validity={:?}",
        mb.exists, mb.uid_validity
    );
    if mb.exists > 0 {
        let start = mb.exists.saturating_sub(9).max(1);
        eprintln!("fetch…");
        let fetches = session.fetch(
            format!("{start}:{}", mb.exists),
            "(UID BODY.PEEK[HEADER.FIELDS (SUBJECT FROM DATE MESSAGE-ID)])",
        )?;
        for f in fetches.iter() {
            println!("--- uid={:?}", f.uid);
            if let Some(h) = f.header() {
                print!("{}", String::from_utf8_lossy(h));
            }
        }
    }
    session.logout()?;
    Ok(())
}
