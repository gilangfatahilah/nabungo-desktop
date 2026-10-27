//! Sumber email: trait untuk loop sync + implementasi IMAP read-only.

use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("login failed")]
    Auth,
    #[error("mailbox not found")]
    Mailbox,
    #[error("connection: {0}")]
    Connection(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub uid: u32,
    pub raw: Vec<u8>,
}

pub trait MailSource {
    /// Read-only (EXAMINE). Mengembalikan UIDVALIDITY.
    fn open(&mut self, mailbox: &str) -> Result<u32, SourceError>;
    /// UID > `after`, naik, maksimal `limit`. Memakai BODY.PEEK[] (tidak menandai dibaca).
    fn fetch_after(&mut self, after: u32, limit: usize) -> Result<Vec<Fetched>, SourceError>;
}

type TlsSession = imap::Session<native_tls::TlsStream<TcpStream>>;

/// IMAP over TLS, read-only: EXAMINE + `BODY.PEEK[]`, tidak pernah menulis/menghapus.
pub struct ImapSource {
    session: TlsSession,
}

/// Batas tunggu baca/tulis socket agar sync tidak menggantung selamanya.
const IO_TIMEOUT: Duration = Duration::from_secs(60);

fn connection(e: impl std::fmt::Display) -> SourceError {
    SourceError::Connection(e.to_string())
}

impl ImapSource {
    pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Self, SourceError> {
        let tcp = TcpStream::connect((host, port)).map_err(connection)?;
        tcp.set_read_timeout(Some(IO_TIMEOUT)).map_err(connection)?;
        tcp.set_write_timeout(Some(IO_TIMEOUT))
            .map_err(connection)?;
        let tls = native_tls::TlsConnector::new().map_err(connection)?;
        let stream = tls.connect(host, tcp).map_err(connection)?;
        let mut client = imap::Client::new(stream);
        client.read_greeting().map_err(connection)?;
        let session = client.login(user, password).map_err(|(e, _)| match e {
            imap::Error::No(_) | imap::Error::Bad(_) => SourceError::Auth,
            e => connection(e),
        })?;
        Ok(Self { session })
    }
}

impl MailSource for ImapSource {
    fn open(&mut self, mailbox: &str) -> Result<u32, SourceError> {
        let mb = self.session.examine(mailbox).map_err(|e| match e {
            imap::Error::No(_) => SourceError::Mailbox,
            e => connection(e),
        })?;
        mb.uid_validity
            .ok_or_else(|| SourceError::Connection("server tidak mengirim UIDVALIDITY".into()))
    }

    fn fetch_after(&mut self, after: u32, limit: usize) -> Result<Vec<Fetched>, SourceError> {
        // `n:*` selalu menyertakan pesan terakhir walau UID-nya < n: saring ulang.
        let mut uids: Vec<u32> = self
            .session
            .uid_search(format!("UID {}:*", after.saturating_add(1)))
            .map_err(connection)?
            .into_iter()
            .filter(|&u| u > after)
            .collect();
        uids.sort_unstable();
        uids.truncate(limit);
        if uids.is_empty() {
            return Ok(Vec::new());
        }
        let set = uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let fetches = self
            .session
            .uid_fetch(set, "BODY.PEEK[]")
            .map_err(connection)?;
        let mut out: Vec<Fetched> = fetches
            .iter()
            .filter_map(|f| {
                Some(Fetched {
                    uid: f.uid?,
                    raw: f.body()?.to_vec(),
                })
            })
            .filter(|f| f.uid > after)
            .collect();
        out.sort_by_key(|f| f.uid);
        Ok(out)
    }
}

impl Drop for ImapSource {
    fn drop(&mut self) {
        let _ = self.session.logout();
    }
}
