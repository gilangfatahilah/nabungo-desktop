//! Orkestrasi email capture (EML-01): connect/disconnect akun, satu putaran sync, dan jadwal
//! sync berkala. Ledger tidak pernah dipegang selama I/O jaringan atau keychain.

use std::time::{Duration, Instant};

use time::{OffsetDateTime, UtcOffset};

use crate::email_sync::{LedgerAccess, MailSource, SourceError, SyncError, SyncProgress};
use crate::secret::SecretStore;
use crate::{AppError, SyncSummary, normalize_address, normalize_mailbox, sync_mailbox_shared};

/// Membuka sumber email untuk `(address, password)`; di app = `ImapSource::connect` ke Gmail.
pub type Connect<'a> = &'a dyn Fn(&str, &str) -> Result<Box<dyn MailSource>, SourceError>;

pub const SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Gmail menampilkan App Password berkelompok (`abcd efgh ijkl mnop`); spasinya bukan bagian
/// password.
pub fn normalize_app_password(input: &str) -> String {
    input.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Verifikasi login + label, lalu simpan password ke keychain, lalu setting. Gagal di langkah
/// mana pun → keychain dan setting tidak berubah. `password: None` = pakai password tersimpan
/// untuk alamat itu (ganti label saja).
pub fn connect_email<L: LedgerAccess>(
    ledger: &L,
    secrets: &dyn SecretStore,
    connect: Connect,
    address: &str,
    mailbox: &str,
    password: Option<&str>,
) -> Result<(), SyncError> {
    let address = normalize_address(address)?;
    let mailbox = normalize_mailbox(mailbox)?;
    let previous = ledger.with_ledger(|l| l.email_settings())?.address;
    let entered = password
        .map(normalize_app_password)
        .filter(|p| !p.is_empty());
    let password = match &entered {
        Some(p) => p.clone(),
        None => secrets.get(&address)?.ok_or(AppError::EmailNotConfigured)?,
    };
    connect(&address, &password)?.open(&mailbox)?;
    if entered.is_some() {
        secrets.set(&address, &password)?;
    }
    if let Some(old) = previous.filter(|old| *old != address) {
        // Konfigurasi baru sudah valid; entry lama yang tertinggal hanya dicatat.
        if let Err(e) = secrets.delete(&old) {
            tracing::warn!(error = %e, "entry keychain alamat lama tidak terhapus");
        }
    }
    ledger.with_ledger(|l| l.save_email_account(&address, &mailbox))
}

/// Sync berhenti dan entry keychain dihapus; alamat, label, cursor, dan capture tetap.
pub fn disconnect_email<L: LedgerAccess>(
    ledger: &L,
    secrets: &dyn SecretStore,
) -> Result<(), SyncError> {
    // Nonaktifkan dulu: sync berhenti walau keychain gagal dihapus.
    let address = ledger.with_ledger(|l| {
        l.disable_email()?;
        Ok(l.email_settings()?.address)
    })?;
    if let Some(address) = address {
        secrets.delete(&address)?;
    }
    Ok(())
}

/// Satu putaran sync. Email belum dikonfigurasi → `EmailNotConfigured` tanpa menyentuh keychain
/// maupun jaringan.
pub fn run_email_sync<L: LedgerAccess>(
    ledger: &L,
    secrets: &dyn SecretStore,
    connect: Connect,
    local_offset: UtcOffset,
    on_batch: &mut dyn FnMut(SyncProgress),
) -> Result<SyncSummary, SyncError> {
    let settings = ledger.with_ledger(|l| l.email_settings())?;
    let address = match settings.address {
        Some(a) if settings.enabled => a,
        _ => return Err(AppError::EmailNotConfigured.into()),
    };
    let password = secrets.get(&address)?.ok_or(AppError::EmailNotConfigured)?;
    let mut source = connect(&address, &password)?;
    let summary = sync_mailbox_shared(
        ledger,
        source.as_mut(),
        &settings.mailbox,
        local_offset,
        on_batch,
    )?;
    ledger.with_ledger(|l| l.mark_email_synced(OffsetDateTime::now_utc()))?;
    Ok(summary)
}

/// Kapan sync otomatis berikutnya jatuh tempo. Login gagal menjeda sync otomatis sampai
/// `reset` (connect ulang), agar Google tidak memblokir akun karena login gagal berulang.
#[derive(Debug, Default)]
pub struct SyncSchedule {
    last_attempt: Option<Instant>,
    pub paused: bool,
}

impl SyncSchedule {
    pub fn due(&self, now: Instant) -> bool {
        !self.paused
            && self
                .last_attempt
                .is_none_or(|last| now.saturating_duration_since(last) >= SYNC_INTERVAL)
    }

    pub fn record(&mut self, now: Instant, result: &Result<SyncSummary, SyncError>) {
        self.last_attempt = Some(now);
        if matches!(result, Err(SyncError::Source(SourceError::Auth))) {
            self.paused = true;
        }
    }

    /// Putaran berikutnya segera (unlock, restore, connect).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::sync::Mutex;

    use super::*;
    use crate::email_sync::tests::FakeSource;
    use crate::secret::MemorySecretStore;
    use crate::{CaptureStatus, Ledger};
    use time::macros::offset;

    const WIB: UtcOffset = offset!(+7);
    const ADDR: &str = "me@gmail.com";

    fn ledger() -> Mutex<Option<Ledger>> {
        Mutex::new(Some(Ledger::open_in_memory().unwrap()))
    }

    fn settings(l: &Mutex<Option<Ledger>>) -> crate::EmailSettings {
        l.with_ledger(|l| l.email_settings()).unwrap()
    }

    /// Fake `Connect`: mencatat password yang diterima; `fail` dipakai sebagai error.
    struct FakeConnect {
        calls: Cell<u32>,
        passwords: RefCell<Vec<String>>,
        fail: RefCell<Option<SourceError>>,
        messages: u32,
    }

    impl FakeConnect {
        fn new(messages: u32) -> Self {
            FakeConnect {
                calls: Cell::new(0),
                passwords: RefCell::new(vec![]),
                fail: RefCell::new(None),
                messages,
            }
        }

        fn call(&self, _address: &str, password: &str) -> Result<Box<dyn MailSource>, SourceError> {
            self.calls.set(self.calls.get() + 1);
            self.passwords.borrow_mut().push(password.to_owned());
            match self.fail.borrow_mut().take() {
                Some(e) => Err(e),
                None => Ok(Box::new(FakeSource::new(self.messages))),
            }
        }
    }

    fn connect_ok(l: &Mutex<Option<Ledger>>, s: &MemorySecretStore, fc: &FakeConnect) {
        connect_email(l, s, &|a, p| fc.call(a, p), ADDR, "Finance", Some("pw")).unwrap();
    }

    #[test]
    fn connect_verifies_then_saves() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        connect_email(
            &l,
            &s,
            &|a, p| fc.call(a, p),
            " Me@Gmail.com ",
            " Bank ",
            Some("pw"),
        )
        .unwrap();
        assert_eq!(fc.calls.get(), 1);
        assert_eq!(s.get(ADDR).unwrap().as_deref(), Some("pw"));
        let st = settings(&l);
        assert!(st.enabled);
        assert_eq!(st.address.as_deref(), Some(ADDR));
        assert_eq!(st.mailbox, "Bank");
    }

    #[test]
    fn app_password_whitespace_is_stripped() {
        assert_eq!(
            normalize_app_password(" abcd efgh\tijkl\nmnop "),
            "abcdefghijklmnop"
        );
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        connect_email(
            &l,
            &s,
            &|a, p| fc.call(a, p),
            ADDR,
            "Finance",
            Some("abcd efgh ijkl mnop"),
        )
        .unwrap();
        assert_eq!(*fc.passwords.borrow(), vec!["abcdefghijklmnop".to_owned()]);
        assert_eq!(s.get(ADDR).unwrap().as_deref(), Some("abcdefghijklmnop"));
    }

    #[test]
    fn connect_rejects_invalid_input_before_network() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        let err =
            connect_email(&l, &s, &|a, p| fc.call(a, p), "me", "Finance", Some("pw")).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailAddressInvalid)),
            "{err:?}"
        );
        let err = connect_email(&l, &s, &|a, p| fc.call(a, p), ADDR, " ", Some("pw")).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailMailboxEmpty)),
            "{err:?}"
        );
        assert_eq!(fc.calls.get(), 0);
    }

    #[test]
    fn connect_auth_failure_saves_nothing() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        *fc.fail.borrow_mut() = Some(SourceError::Auth);
        let err =
            connect_email(&l, &s, &|a, p| fc.call(a, p), ADDR, "Finance", Some("pw")).unwrap_err();
        assert!(
            matches!(err, SyncError::Source(SourceError::Auth)),
            "{err:?}"
        );
        assert_eq!(s.get(ADDR).unwrap(), None);
        assert!(!settings(&l).enabled);
        assert_eq!(settings(&l).address, None);
    }

    #[test]
    fn connect_keychain_failure_saves_nothing() {
        let (l, s, fc) = (ledger(), MemorySecretStore::failing(), FakeConnect::new(0));
        let err =
            connect_email(&l, &s, &|a, p| fc.call(a, p), ADDR, "Finance", Some("pw")).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::KeychainUnavailable)),
            "{err:?}"
        );
        assert!(!settings(&l).enabled);
        assert_eq!(settings(&l).address, None);
    }

    #[test]
    fn connect_without_password_reuses_stored() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        let err = connect_email(&l, &s, &|a, p| fc.call(a, p), ADDR, "Finance", None).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailNotConfigured)),
            "{err:?}"
        );
        assert_eq!(fc.calls.get(), 0);

        connect_ok(&l, &s, &fc);
        connect_email(&l, &s, &|a, p| fc.call(a, p), ADDR, "Bank", None).unwrap();
        assert_eq!(
            *fc.passwords.borrow(),
            vec!["pw".to_owned(), "pw".to_owned()]
        );
        assert_eq!(settings(&l).mailbox, "Bank");
        assert_eq!(s.get(ADDR).unwrap().as_deref(), Some("pw"));
    }

    #[test]
    fn changing_address_deletes_old_secret() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        connect_ok(&l, &s, &fc);
        connect_email(
            &l,
            &s,
            &|a, p| fc.call(a, p),
            "lain@gmail.com",
            "Finance",
            Some("pw2"),
        )
        .unwrap();
        assert_eq!(s.get(ADDR).unwrap(), None);
        assert_eq!(s.get("lain@gmail.com").unwrap().as_deref(), Some("pw2"));
        assert_eq!(settings(&l).address.as_deref(), Some("lain@gmail.com"));
    }

    #[test]
    fn disconnect_removes_secret_keeps_data() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(2));
        connect_ok(&l, &s, &fc);
        run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| {}).unwrap();
        disconnect_email(&l, &s).unwrap();
        assert_eq!(s.get(ADDR).unwrap(), None);
        let st = settings(&l);
        assert!(!st.enabled);
        assert_eq!(st.address.as_deref(), Some(ADDR));
        let pending = l
            .with_ledger(|l| l.list_captures(CaptureStatus::Pending))
            .unwrap();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn disabled_never_touches_keychain_or_network() {
        let (l, s, fc) = (ledger(), MemorySecretStore::failing(), FakeConnect::new(0));
        let err = run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| {}).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailNotConfigured)),
            "{err:?}"
        );
        assert_eq!(fc.calls.get(), 0);

        // Pernah terhubung lalu diputus: tetap tidak menyentuh keychain/jaringan.
        l.with_ledger(|l| {
            l.save_email_account(ADDR, "Finance")?;
            l.disable_email()
        })
        .unwrap();
        let err = run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| {}).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailNotConfigured)),
            "{err:?}"
        );
        assert_eq!(fc.calls.get(), 0);
    }

    #[test]
    fn run_sync_marks_synced_on_success_only() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(2));
        connect_ok(&l, &s, &fc);
        *fc.fail.borrow_mut() = Some(SourceError::Connection("putus".into()));
        let err = run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| {}).unwrap_err();
        assert!(
            matches!(err, SyncError::Source(SourceError::Connection(_))),
            "{err:?}"
        );
        assert_eq!(settings(&l).last_sync_at, None);

        let mut batches = 0;
        let sum =
            run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| batches += 1).unwrap();
        assert_eq!(sum.new_items, 2);
        assert_eq!(batches, 1);
        assert!(settings(&l).last_sync_at.is_some());
    }

    #[test]
    fn run_sync_without_stored_password_is_not_configured() {
        let (l, s, fc) = (ledger(), MemorySecretStore::default(), FakeConnect::new(0));
        l.with_ledger(|l| l.save_email_account(ADDR, "Finance"))
            .unwrap();
        let err = run_email_sync(&l, &s, &|a, p| fc.call(a, p), WIB, &mut |_| {}).unwrap_err();
        assert!(
            matches!(err, SyncError::App(AppError::EmailNotConfigured)),
            "{err:?}"
        );
        assert_eq!(fc.calls.get(), 0);
    }

    #[test]
    fn auth_failure_pauses_schedule() {
        let t0 = Instant::now();
        let mut sch = SyncSchedule::default();
        sch.record(t0, &Err(SyncError::Source(SourceError::Auth)));
        assert!(sch.paused);
        assert!(!sch.due(t0 + SYNC_INTERVAL * 100));
        sch.reset();
        assert!(sch.due(t0));

        sch.record(
            t0,
            &Err(SyncError::Source(SourceError::Connection("x".into()))),
        );
        assert!(!sch.paused);
        assert!(!sch.due(t0 + Duration::from_secs(60)));
        assert!(sch.due(t0 + SYNC_INTERVAL));
    }

    #[test]
    fn schedule_due_rules() {
        let t0 = Instant::now();
        let mut sch = SyncSchedule::default();
        assert!(sch.due(t0));
        sch.record(t0, &Ok(SyncSummary::default()));
        assert!(!sch.due(t0 + SYNC_INTERVAL - Duration::from_secs(1)));
        assert!(sch.due(t0 + SYNC_INTERVAL));
    }
}
