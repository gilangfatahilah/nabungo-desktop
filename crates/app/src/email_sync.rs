//! Loop sync satu mailbox (EML-01): sumber → `ingest_email` per pesan, cursor per pesan (K4).

pub use nabungo_mail::source::{Fetched, MailSource, SourceError};
use std::cell::RefCell;
use std::sync::Mutex;
use time::UtcOffset;

use crate::Cursor;
use crate::capture::IngestOutcome;
use crate::ledger::{AppError, Ledger};

/// Pesan per `fetch_after`.
pub const SYNC_BATCH: usize = 50;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncSummary {
    /// Item baru di staging.
    pub new_items: u32,
    pub unparsed: u32,
    /// Email yang sudah pernah diproses (sync ulang / UIDVALIDITY berubah).
    pub seen: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    App(#[from] AppError),
    /// App terkunci (atau ledger ditutup) di tengah sync.
    #[error("ledger is locked")]
    Locked,
    /// DB diganti (restore/recovery) di tengah sync.
    #[error("sync interrupted: database changed")]
    Interrupted,
}

/// Akses ledger bersama untuk sync yang berjalan di thread lain; ledger hanya dipegang selama
/// `f`. `Err(SyncError::Locked)` bila app terkunci / ledger tidak ada.
pub trait LedgerAccess {
    fn with_ledger<R>(
        &self,
        f: impl FnOnce(&mut Ledger) -> Result<R, AppError>,
    ) -> Result<R, SyncError>;
}

impl LedgerAccess for Mutex<Option<Ledger>> {
    fn with_ledger<R>(
        &self,
        f: impl FnOnce(&mut Ledger) -> Result<R, AppError>,
    ) -> Result<R, SyncError> {
        let mut guard = self.lock().unwrap_or_else(|e| e.into_inner());
        let ledger = guard.as_mut().ok_or(SyncError::Locked)?;
        Ok(f(ledger)?)
    }
}

/// Adapter untuk [`sync_mailbox`] (pemanggil sudah memegang `&mut Ledger`).
struct Exclusive<'a>(RefCell<&'a mut Ledger>);

impl LedgerAccess for Exclusive<'_> {
    fn with_ledger<R>(
        &self,
        f: impl FnOnce(&mut Ledger) -> Result<R, AppError>,
    ) -> Result<R, SyncError> {
        Ok(f(&mut self.0.borrow_mut())?)
    }
}

/// Kemajuan kumulatif satu putaran, dilaporkan setelah tiap batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncProgress {
    pub processed: u32,
    pub new_items: u32,
    pub unparsed: u32,
}

/// Seperti [`sync_mailbox`], tetapi ledger hanya dikunci per email (fetch jaringan di luar
/// lock). Cursor di DB dicek sebelum tiap ingest: berbeda dari yang terakhir ditulis (DB
/// diganti restore/recovery) → `Interrupted` tanpa menulis.
pub fn sync_mailbox_shared<L: LedgerAccess>(
    ledger: &L,
    source: &mut dyn MailSource,
    mailbox: &str,
    local_offset: UtcOffset,
    on_batch: &mut dyn FnMut(SyncProgress),
) -> Result<SyncSummary, SyncError> {
    let (rules, stored) = ledger.with_ledger(|l| {
        l.seed_email_rules()?;
        Ok((l.email_rules()?, l.email_cursor(mailbox)?))
    })?;
    let uid_validity = source.open(mailbox)?;
    let mut last_uid = match stored {
        Some(c) if c.uid_validity == uid_validity => c.last_uid,
        _ => 0,
    };
    // Cursor yang seharusnya ada di DB; ingest selalu menulis cursor pesan (K4).
    let mut expected = stored;
    let mut summary = SyncSummary::default();
    let mut progress = SyncProgress::default();
    loop {
        let batch = source.fetch_after(last_uid, SYNC_BATCH)?;
        if batch.is_empty() {
            return Ok(summary);
        }
        for msg in batch {
            let cursor = Cursor {
                uid_validity,
                last_uid: msg.uid,
            };
            let outcome = ledger
                .with_ledger(|l| {
                    if l.email_cursor(mailbox)? != expected {
                        return Ok(None);
                    }
                    l.ingest_email(mailbox, cursor, &msg.raw, &rules, local_offset)
                        .map(Some)
                })?
                .ok_or(SyncError::Interrupted)?;
            expected = Some(cursor);
            match outcome {
                IngestOutcome::Captured(n) => summary.new_items += n,
                IngestOutcome::Unparsed => summary.unparsed += 1,
                IngestOutcome::AlreadySeen => summary.seen += 1,
            }
            progress.processed += 1;
            last_uid = last_uid.max(msg.uid);
        }
        progress.new_items = summary.new_items;
        progress.unparsed = summary.unparsed;
        on_batch(progress);
    }
}

/// Ambil semua email baru dari `mailbox`. Error di tengah jalan tidak membuang yang sudah
/// masuk: cursor maju per pesan, sync berikutnya lanjut dari situ. UIDVALIDITY berubah →
/// scan ulang dari awal; dedup Message-ID mencegah dobel.
pub fn sync_mailbox(
    ledger: &mut Ledger,
    source: &mut dyn MailSource,
    mailbox: &str,
    local_offset: UtcOffset,
) -> Result<SyncSummary, SyncError> {
    let access = Exclusive(RefCell::new(ledger));
    sync_mailbox_shared(&access, source, mailbox, local_offset, &mut |_| {})
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{CaptureStatus, Ledger};
    use time::macros::offset;

    const WIB: UtcOffset = offset!(+7);
    const MAILBOX: &str = "Finance";

    pub(crate) struct FakeSource {
        validity: u32,
        messages: Vec<Fetched>,
        /// Dipanggil di awal tiap `fetch_after` dengan indeks panggilan (0 = pertama).
        on_fetch: Option<Box<dyn FnMut(usize)>>,
        /// Gagal setelah sejumlah pesan terkirim (total sejak dibuat).
        fail_after: Option<usize>,
        delivered: usize,
        calls: Vec<u32>,
    }

    impl FakeSource {
        pub(crate) fn new(count: u32) -> Self {
            FakeSource {
                validity: 1,
                messages: (0..count).map(mail).collect(),
                on_fetch: None,
                fail_after: None,
                delivered: 0,
                calls: vec![],
            }
        }
    }

    impl MailSource for FakeSource {
        fn open(&mut self, _mailbox: &str) -> Result<u32, SourceError> {
            Ok(self.validity)
        }

        fn fetch_after(&mut self, after: u32, limit: usize) -> Result<Vec<Fetched>, SourceError> {
            if let Some(hook) = self.on_fetch.as_mut() {
                hook(self.calls.len());
            }
            self.calls.push(after);
            let mut limit = limit;
            if let Some(n) = self.fail_after {
                if self.delivered >= n {
                    return Err(SourceError::Connection("putus".into()));
                }
                limit = limit.min(n - self.delivered);
            }
            let out: Vec<Fetched> = self
                .messages
                .iter()
                .filter(|m| m.uid > after)
                .take(limit)
                .cloned()
                .collect();
            self.delivered += out.len();
            Ok(out)
        }
    }

    /// Email tarik tunai BCA sintetis ber-UID `100 + i`.
    fn mail(i: u32) -> Fetched {
        let raw = format!(
            "Message-ID: <m{i}@bca.co.id>\r\nFrom: BCA <bca@bca.co.id>\r\n\
             Subject: Cash Withdrawal Successful\r\nDate: Mon, 26 Oct 2026 10:00:00 +0700\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\nAmount : IDR {},000.00\r\n",
            i + 1
        );
        Fetched {
            uid: 100 + i,
            raw: raw.into_bytes(),
        }
    }

    fn pending(ledger: &Ledger) -> usize {
        ledger.list_captures(CaptureStatus::Pending).unwrap().len()
    }

    #[test]
    fn first_sync_ingests_all() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(3);
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(
            sum,
            SyncSummary {
                new_items: 3,
                unparsed: 0,
                seen: 0
            }
        );
        assert_eq!(pending(&ledger), 3);
        let cursor = ledger.email_cursor(MAILBOX).unwrap().unwrap();
        assert_eq!((cursor.uid_validity, cursor.last_uid), (1, 102));
    }

    #[test]
    fn second_sync_fetches_nothing_new() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(3);
        sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        src.calls.clear();
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(sum, SyncSummary::default());
        assert_eq!(src.calls, vec![102]);
    }

    #[test]
    fn sync_resumes_after_source_error() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(5);
        src.fail_after = Some(2);
        let err = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap_err();
        assert!(matches!(err, SyncError::Source(_)), "{err:?}");
        assert_eq!(pending(&ledger), 2);
        src.fail_after = None;
        src.calls.clear();
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(sum.new_items, 3);
        assert_eq!(src.calls[0], 101);
        assert_eq!(pending(&ledger), 5);
    }

    #[test]
    fn uidvalidity_change_rescans_without_duplicates() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(3);
        sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        let first = ledger.list_captures(CaptureStatus::Pending).unwrap()[0].id;
        ledger.ignore_capture(first).unwrap();
        src.validity = 2;
        src.calls.clear();
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(src.calls[0], 0);
        assert_eq!(
            sum,
            SyncSummary {
                new_items: 0,
                unparsed: 0,
                seen: 3
            }
        );
        assert_eq!(pending(&ledger), 2);
        let ignored = ledger.list_captures(CaptureStatus::Ignored).unwrap();
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].id, first);
        let cursor = ledger.email_cursor(MAILBOX).unwrap().unwrap();
        assert_eq!((cursor.uid_validity, cursor.last_uid), (2, 102));
    }

    #[test]
    fn batches_until_empty() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(120);
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(sum.new_items, 120);
        assert_eq!(src.calls, vec![0, 149, 199, 219]);
    }

    #[test]
    fn unparsed_mail_is_counted() {
        let mut ledger = Ledger::open_in_memory().unwrap();
        let mut src = FakeSource::new(1);
        src.messages.push(Fetched {
            uid: 500,
            raw: b"Message-ID: <x@spam.example>\r\nFrom: a@spam.example\r\nSubject: Promo\r\n\r\nhalo\r\n"
                .to_vec(),
        });
        let sum = sync_mailbox(&mut ledger, &mut src, MAILBOX, WIB).unwrap();
        assert_eq!(
            sum,
            SyncSummary {
                new_items: 1,
                unparsed: 1,
                seen: 0
            }
        );
    }

    fn shared(ledger: Ledger) -> std::sync::Arc<Mutex<Option<Ledger>>> {
        std::sync::Arc::new(Mutex::new(Some(ledger)))
    }

    fn pending_shared(shared: &Mutex<Option<Ledger>>) -> usize {
        pending(shared.lock().unwrap().as_ref().unwrap())
    }

    #[test]
    fn shared_sync_matches_direct() {
        let shared = shared(Ledger::open_in_memory().unwrap());
        let mut src = FakeSource::new(3);
        let mut batches = vec![];
        let sum = sync_mailbox_shared(&*shared, &mut src, MAILBOX, WIB, &mut |p| batches.push(p))
            .unwrap();
        assert_eq!(
            sum,
            SyncSummary {
                new_items: 3,
                unparsed: 0,
                seen: 0
            }
        );
        assert_eq!(
            batches,
            vec![SyncProgress {
                processed: 3,
                new_items: 3,
                unparsed: 0
            }]
        );
        assert_eq!(pending_shared(&shared), 3);
    }

    #[test]
    fn locked_ledger_fails_before_fetch() {
        let shared: Mutex<Option<Ledger>> = Mutex::new(None);
        let mut src = FakeSource::new(3);
        let err = sync_mailbox_shared(&shared, &mut src, MAILBOX, WIB, &mut |_| {}).unwrap_err();
        assert!(matches!(err, SyncError::Locked), "{err:?}");
        assert!(src.calls.is_empty());
    }

    #[test]
    fn lock_mid_sync_stops_with_locked() {
        let shared = shared(Ledger::open_in_memory().unwrap());
        let stash = std::sync::Arc::new(Mutex::new(None));
        let mut src = FakeSource::new(120);
        let (s, st) = (shared.clone(), stash.clone());
        src.on_fetch = Some(Box::new(move |i| {
            if i == 1 {
                *st.lock().unwrap() = s.lock().unwrap().take();
            }
        }));
        let err = sync_mailbox_shared(&*shared, &mut src, MAILBOX, WIB, &mut |_| {}).unwrap_err();
        assert!(matches!(err, SyncError::Locked), "{err:?}");

        *shared.lock().unwrap() = stash.lock().unwrap().take();
        assert_eq!(pending_shared(&shared), 50);
        let mut src = FakeSource::new(120);
        let sum = sync_mailbox_shared(&*shared, &mut src, MAILBOX, WIB, &mut |_| {}).unwrap();
        assert_eq!(sum.new_items, 70);
        assert_eq!(src.calls[0], 149);
        assert_eq!(pending_shared(&shared), 120);
    }

    #[test]
    fn cursor_changed_mid_sync_is_interrupted() {
        let shared = shared(Ledger::open_in_memory().unwrap());
        let mut src = FakeSource::new(120);
        let s = shared.clone();
        src.on_fetch = Some(Box::new(move |i| {
            if i == 1 {
                *s.lock().unwrap() = Some(Ledger::open_in_memory().unwrap());
            }
        }));
        let err = sync_mailbox_shared(&*shared, &mut src, MAILBOX, WIB, &mut |_| {}).unwrap_err();
        assert!(matches!(err, SyncError::Interrupted), "{err:?}");
        let guard = shared.lock().unwrap();
        let fresh = guard.as_ref().unwrap();
        assert_eq!(pending(fresh), 0);
        assert_eq!(fresh.email_cursor(MAILBOX).unwrap(), None);
    }
}
