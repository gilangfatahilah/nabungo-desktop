//! Backup app-managed di folder `backups/`: penamaan, daftar, retensi, jadwal harian.

use std::path::{Path, PathBuf};

use nabungo_storage::rusqlite::Connection;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Duration, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::{AppError, Ledger};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackupKind {
    Auto,
    PreMigration,
    PreRestore,
}

impl BackupKind {
    // Urutan penting saat parsing: tidak ada prefiks yang menjadi awalan prefiks lain.
    const ALL: [BackupKind; 3] = [
        BackupKind::Auto,
        BackupKind::PreMigration,
        BackupKind::PreRestore,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            BackupKind::Auto => "auto",
            BackupKind::PreMigration => "pre-migration",
            BackupKind::PreRestore => "pre-restore",
        }
    }

    /// Jumlah file yang dipertahankan per jenis.
    fn keep(self) -> usize {
        match self {
            BackupKind::Auto => 14,
            BackupKind::PreMigration | BackupKind::PreRestore => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    pub file_name: String,
    pub kind: BackupKind,
    pub created_at: OffsetDateTime,
    pub size_bytes: u64,
}

const STAMP: &[FormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");

/// Backup otomatis jatuh tempo setelah jeda ini.
const AUTO_INTERVAL: Duration = Duration::hours(20);

/// `{kind}-{YYYYMMDD}T{HHMMSS}Z[-N].db` → (kind, waktu, N).
fn parse_name(name: &str) -> Option<(BackupKind, OffsetDateTime, u32)> {
    let stem = name.strip_suffix(".db")?;
    let (kind, rest) = BackupKind::ALL.iter().find_map(|k| {
        stem.strip_prefix(k.as_str())
            .and_then(|r| r.strip_prefix('-'))
            .map(|r| (*k, r))
    })?;
    let (stamp, seq) = match rest.split_once('-') {
        Some((stamp, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            (stamp, n.parse().ok()?)
        }
        Some(_) => return None,
        None => (rest, 0),
    };
    let at = PrimitiveDateTime::parse(stamp, STAMP).ok()?.assume_utc();
    Some((kind, at, seq))
}

pub fn parse_backup_name(name: &str) -> Option<(BackupKind, OffsetDateTime)> {
    parse_name(name).map(|(k, t, _)| (k, t))
}

/// Backup di `dir`, terbaru dulu. Folder yang belum ada = daftar kosong.
pub fn list_backups(dir: &Path) -> Result<Vec<BackupInfo>, AppError> {
    Ok(scan(dir)?.into_iter().map(|(info, _)| info).collect())
}

fn scan(dir: &Path) -> Result<Vec<(BackupInfo, u32)>, AppError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let Some((kind, created_at, seq)) = parse_name(&name) else {
            continue;
        };
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }
        out.push((
            BackupInfo {
                file_name: name,
                kind,
                created_at,
                size_bytes: meta.len(),
            },
            seq,
        ));
    }
    out.sort_by(|(a, sa), (b, sb)| (b.created_at, sb).cmp(&(a.created_at, sa)));
    Ok(out)
}

pub fn is_auto_due(last: Option<OffsetDateTime>, now: OffsetDateTime) -> bool {
    match last {
        None => true,
        // Jam sistem mundur: anggap jatuh tempo agar backup tetap berjalan.
        Some(last) if last > now => true,
        Some(last) => now - last >= AUTO_INTERVAL,
    }
}

/// Tulis backup baru di `dir` (dibuat bila perlu), lalu jalankan retensi jenisnya.
/// Retensi hanya berjalan bila backup sukses.
pub(crate) fn write_backup(
    conn: &Connection,
    dir: &Path,
    kind: BackupKind,
    now: OffsetDateTime,
) -> Result<BackupInfo, AppError> {
    let info = write_unpruned(conn, dir, kind, now)?;
    prune_logged(dir, kind);
    Ok(info)
}

fn write_unpruned(
    conn: &Connection,
    dir: &Path,
    kind: BackupKind,
    now: OffsetDateTime,
) -> Result<BackupInfo, AppError> {
    std::fs::create_dir_all(dir)?;
    let now = now
        .to_offset(UtcOffset::UTC)
        .replace_nanosecond(0)
        .unwrap_or(now);
    let stamp = now
        .format(STAMP)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let base = format!("{}-{stamp}", kind.as_str());
    let mut seq = 0u32;
    let file_name = loop {
        let name = if seq == 0 {
            format!("{base}.db")
        } else {
            format!("{base}-{seq}.db")
        };
        if !dir.join(&name).exists() {
            break name;
        }
        seq += 1;
    };
    let path = dir.join(&file_name);
    // Pra-restore menyalin DB apa adanya, termasuk yang rusak: justru saat itu restore dibutuhkan.
    // Jatahnya terpisah, jadi salinan rusak tidak menggeser backup otomatis.
    if kind == BackupKind::PreRestore {
        nabungo_storage::backup_into_unverified(conn, &path)?;
    } else {
        nabungo_storage::backup_into(conn, &path)?;
    }
    let size_bytes = std::fs::metadata(&path)?.len();
    Ok(BackupInfo {
        file_name,
        kind,
        created_at: now,
        size_bytes,
    })
}

/// Hapus backup terlama jenis `kind` melebihi jatah retensi. File lain tidak disentuh.
pub(crate) fn prune(dir: &Path, kind: BackupKind) -> Result<(), AppError> {
    let of_kind = scan(dir)?.into_iter().filter(|(b, _)| b.kind == kind);
    for (old, _) in of_kind.skip(kind.keep()) {
        std::fs::remove_file(dir.join(&old.file_name))?;
    }
    Ok(())
}

/// Retensi yang gagal (mis. file terkunci di Windows) hanya dicatat: backup baru sudah
/// tersimpan, dan setelah restore DB sudah berganti, jadi jangan dilaporkan sebagai gagal.
fn prune_logged(dir: &Path, kind: BackupKind) {
    if let Err(e) = prune(dir, kind) {
        tracing::warn!(error = %e, kind = kind.as_str(), "retensi backup gagal");
    }
}

/// `path` setelah symlink dan `.`/`..` di-resolve; file boleh belum ada.
fn resolve(path: &Path) -> Result<PathBuf, AppError> {
    if let Ok(p) = std::fs::canonicalize(path) {
        return Ok(p);
    }
    let name = path.file_name().ok_or(AppError::BackupTargetInvalid)?;
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    Ok(std::fs::canonicalize(parent)?.join(name))
}

impl Ledger {
    /// Buka DB; bila skemanya lama (bukan DB baru), simpan backup pra-migrasi dulu (K6).
    pub fn open_with_backups(
        path: &Path,
        backup_dir: &Path,
        now: OffsetDateTime,
    ) -> Result<Self, AppError> {
        let mut conn = nabungo_storage::open_file(path)?;
        let version = nabungo_storage::schema_version(&conn)?;
        if version > 0 && version < nabungo_storage::latest_version() {
            write_backup(&conn, backup_dir, BackupKind::PreMigration, now)?;
        }
        nabungo_storage::migrate(&mut conn)?;
        Ok(Self {
            conn,
            path: Some(path.to_path_buf()),
        })
    }

    /// Tolak `target` bila menunjuk DB utama atau file pendampingnya (K10).
    pub(crate) fn ensure_not_live(&self, target: &Path) -> Result<(), AppError> {
        let Some(live) = &self.path else {
            return Ok(());
        };
        let live = resolve(live)?;
        let target = resolve(target)?;
        let mut forbidden = live.clone().into_os_string();
        let base = forbidden.clone();
        if target == live {
            return Err(AppError::BackupTargetInvalid);
        }
        for suffix in ["-wal", "-shm", "-journal"] {
            forbidden.clone_from(&base);
            forbidden.push(suffix);
            if target.as_os_str() == forbidden {
                return Err(AppError::BackupTargetInvalid);
            }
        }
        Ok(())
    }

    /// Backup manual ke path pilihan user (K3), tanpa retensi.
    pub fn backup_to(&self, dest: &Path) -> Result<(), AppError> {
        self.ensure_not_live(dest)?;
        nabungo_storage::backup_into(&self.conn, dest)?;
        Ok(())
    }

    /// Backup otomatis bila jatuh tempo (K5); `None` bila belum waktunya.
    pub fn auto_backup_if_due(
        &self,
        dir: &Path,
        now: OffsetDateTime,
    ) -> Result<Option<BackupInfo>, AppError> {
        // Backup bertanggal masa depan (jam pernah maju) diabaikan; kalau tidak, setiap
        // putaran jatuh tempo dan retensi menghapus riwayat harian.
        let last = list_backups(dir)?
            .into_iter()
            .find(|b| b.kind == BackupKind::Auto && b.created_at <= now)
            .map(|b| b.created_at);
        if !is_auto_due(last, now) {
            return Ok(None);
        }
        write_backup(&self.conn, dir, BackupKind::Auto, now).map(Some)
    }

    /// Pulihkan dari `src` (K7): periksa → backup pra-restore → salin → migrasi.
    /// File tidak valid tidak mengubah apa pun.
    pub fn restore_file(
        &mut self,
        src: &Path,
        backup_dir: &Path,
        now: OffsetDateTime,
    ) -> Result<(), AppError> {
        self.ensure_not_live(src)?;
        nabungo_storage::inspect_backup(src).map_err(AppError::from_backup)?;
        write_unpruned(&self.conn, backup_dir, BackupKind::PreRestore, now)?;
        nabungo_storage::restore_from(&mut self.conn, src).map_err(AppError::from_backup)?;
        nabungo_storage::migrate(&mut self.conn)?;
        // Retensi setelah restore: sumbernya bisa saja pre-restore tertua.
        prune_logged(backup_dir, BackupKind::PreRestore);
        Ok(())
    }

    /// Pulihkan backup app-managed berdasarkan nama file (K11).
    pub fn restore_named(
        &mut self,
        backup_dir: &Path,
        file_name: &str,
        now: OffsetDateTime,
    ) -> Result<(), AppError> {
        if file_name.contains(['/', '\\']) || parse_backup_name(file_name).is_none() {
            return Err(AppError::InvalidBackup);
        }
        self.restore_file(&backup_dir.join(file_name), backup_dir, now)
    }

    /// Pesan `PRAGMA integrity_check`; kosong = sehat.
    pub fn integrity_check(&self) -> Result<Vec<String>, AppError> {
        Ok(nabungo_storage::integrity_check(&self.conn)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn conn() -> nabungo_storage::rusqlite::Connection {
        nabungo_storage::open_in_memory().unwrap()
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        v.sort();
        v
    }

    const T: OffsetDateTime = datetime!(2026-10-20 03:15:00 UTC);

    #[test]
    fn name_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let c = conn();
        let a = write_backup(&c, dir.path(), BackupKind::Auto, T).unwrap();
        assert_eq!(a.file_name, "auto-20261020T031500Z.db");
        assert_eq!(a.kind, BackupKind::Auto);
        assert_eq!(a.created_at, T);
        let b = write_backup(&c, dir.path(), BackupKind::Auto, T).unwrap();
        assert_eq!(b.file_name, "auto-20261020T031500Z-1.db");
        assert_eq!(
            parse_backup_name("pre-migration-20261020T031500Z-2.db"),
            Some((BackupKind::PreMigration, T))
        );
        assert_eq!(
            parse_backup_name("pre-restore-20261020T031500Z.db"),
            Some((BackupKind::PreRestore, T))
        );
        for bad in [
            "auto-2026.db",
            "../auto-20261020T031500Z.db",
            "manual-20261020T031500Z.db",
            "auto-20261020T031500Z.sqlite",
            "auto-20261020T031500Z-.db",
            "auto-20261020T031500Z-x.db",
            "auto-20261320T031500Z.db",
        ] {
            assert!(parse_backup_name(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn auto_due_rules() {
        use time::Duration;
        assert!(is_auto_due(None, T));
        assert!(!is_auto_due(Some(T - Duration::hours(19)), T));
        assert!(is_auto_due(Some(T - Duration::hours(20)), T));
        assert!(is_auto_due(Some(T + Duration::hours(1)), T));
    }

    #[test]
    fn prune_keeps_newest_per_kind() {
        use time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let c = conn();
        for i in 0..16 {
            write_backup(&c, dir.path(), BackupKind::Auto, T + Duration::days(i)).unwrap();
        }
        for i in 0..6 {
            write_backup(
                &c,
                dir.path(),
                BackupKind::PreRestore,
                T + Duration::days(i),
            )
            .unwrap();
        }
        let list = list_backups(dir.path()).unwrap();
        let autos: Vec<_> = list.iter().filter(|b| b.kind == BackupKind::Auto).collect();
        assert_eq!(autos.len(), 14);
        assert_eq!(autos.last().unwrap().created_at, T + Duration::days(2));
        let pre: Vec<_> = list
            .iter()
            .filter(|b| b.kind == BackupKind::PreRestore)
            .collect();
        assert_eq!(pre.len(), 5);
        assert_eq!(pre.last().unwrap().created_at, T + Duration::days(1));
    }

    #[test]
    fn prune_keeps_newest_suffix_within_same_second() {
        let dir = tempfile::tempdir().unwrap();
        let c = conn();
        for _ in 0..15 {
            write_backup(&c, dir.path(), BackupKind::Auto, T).unwrap();
        }
        let n = names(dir.path());
        assert_eq!(n.len(), 14);
        assert!(!n.contains(&"auto-20261020T031500Z.db".to_owned()));
        assert!(n.contains(&"auto-20261020T031500Z-14.db".to_owned()));
    }

    #[test]
    fn prune_ignores_foreign_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("catatan.txt"), "x").unwrap();
        std::fs::write(dir.path().join("nabungo-backup-2026-10-01.db"), "x").unwrap();
        let c = conn();
        for i in 0..15 {
            write_backup(
                &c,
                dir.path(),
                BackupKind::Auto,
                T + time::Duration::days(i),
            )
            .unwrap();
        }
        let n = names(dir.path());
        assert!(n.contains(&"catatan.txt".to_owned()));
        assert!(n.contains(&"nabungo-backup-2026-10-01.db".to_owned()));
        assert_eq!(n.len(), 16);
    }

    #[test]
    fn list_is_newest_first_with_sizes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_backups(&dir.path().join("none")).unwrap().is_empty());
        let c = conn();
        write_backup(&c, dir.path(), BackupKind::Auto, T).unwrap();
        write_backup(
            &c,
            dir.path(),
            BackupKind::PreMigration,
            T + time::Duration::hours(1),
        )
        .unwrap();
        let list = list_backups(dir.path()).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, BackupKind::PreMigration);
        assert!(list[0].created_at > list[1].created_at);
        assert!(list.iter().all(|b| b.size_bytes > 0));
    }

    #[test]
    fn write_creates_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("backups");
        write_backup(&conn(), &sub, BackupKind::Auto, T).unwrap();
        assert_eq!(names(&sub).len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn failed_check_keeps_existing_backups() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let c = conn();
        for i in 0..14 {
            write_backup(
                &c,
                dir.path(),
                BackupKind::Auto,
                T + time::Duration::days(i),
            )
            .unwrap();
        }
        let before = names(dir.path());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let r = write_backup(
            &c,
            dir.path(),
            BackupKind::Auto,
            T + time::Duration::days(30),
        );
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(r.is_err());
        assert_eq!(names(dir.path()), before);
    }

    mod ledger_ops {
        use std::path::{Path, PathBuf};

        use nabungo_core::{AccountKind, FormBody, PeriodStartDay, SplitLine};
        use time::macros::{date, datetime};
        use time::{Duration, OffsetDateTime};

        use crate::{AccountFields, AppError, Ledger, Locale, OnboardingInput, TxnForm};

        const NOW: OffsetDateTime = datetime!(2026-10-20 03:15:00 UTC);

        type Snapshot = (Vec<(String, i64)>, crate::NetWorth);

        fn snapshot(l: &Ledger) -> Snapshot {
            let o = l.accounts_overview().unwrap();
            let mut v: Vec<_> = o
                .accounts
                .into_iter()
                .map(|a| (a.name, a.balance))
                .collect();
            v.sort();
            (v, o.net_worth)
        }

        fn cat(l: &Ledger, name: &str) -> crate::Uuid {
            l.list_categories()
                .unwrap()
                .into_iter()
                .find(|c| c.name == name)
                .unwrap()
                .id
        }

        fn form(l: &mut Ledger, body: FormBody) {
            l.create_txn(&TxnForm {
                body,
                date: date!(2026 - 10 - 05),
                payee: None,
                note: None,
                tags: vec![],
            })
            .unwrap();
        }

        fn expense(l: &mut Ledger, account_id: crate::Uuid, amount: i64) {
            let category_id = cat(l, "Makan & Minum");
            form(
                l,
                FormBody::Expense {
                    account_id,
                    amount,
                    splits: vec![SplitLine {
                        category_id,
                        amount,
                        memo: None,
                    }],
                },
            );
        }

        /// Onboarding + Dompet + investasi; kembalikan id BCA.
        fn seed(l: &mut Ledger) -> crate::Uuid {
            let opening = date!(2026 - 10 - 01);
            let bca = l
                .complete_onboarding(&OnboardingInput {
                    locale: Locale::Id,
                    period_start_day: PeriodStartDay::new(1).unwrap(),
                    account_kind: AccountKind::Bank,
                    account: AccountFields::new("BCA", 1_000_000, opening),
                })
                .unwrap();
            let dompet = l
                .create_account(AccountKind::Cash, &AccountFields::new("Dompet", 0, opening))
                .unwrap();
            let reksa = l
                .create_account(
                    AccountKind::Investment,
                    &AccountFields::new("Reksa dana", 500_000, opening),
                )
                .unwrap();
            expense(l, bca, 25_000);
            let fee = Some(nabungo_core::Fee {
                amount: 2_500,
                category_id: cat(l, "Biaya admin"),
            });
            form(
                l,
                FormBody::Transfer {
                    from: bca,
                    to: dompet,
                    amount: 100_000,
                    fee,
                },
            );
            l.record_valuation(reksa, date!(2026 - 10 - 10), 530_000)
                .unwrap();
            bca
        }

        struct Fixture {
            _dir: tempfile::TempDir,
            root: PathBuf,
            ledger: Ledger,
            bca: crate::Uuid,
        }

        impl Fixture {
            fn new() -> Self {
                let dir = tempfile::tempdir().unwrap();
                let root = dir.path().to_path_buf();
                let mut ledger = Ledger::open(&root.join("nabungo.db")).unwrap();
                let bca = seed(&mut ledger);
                Fixture {
                    _dir: dir,
                    root,
                    ledger,
                    bca,
                }
            }
            fn db(&self) -> PathBuf {
                self.root.join("nabungo.db")
            }
            fn backups(&self) -> PathBuf {
                self.root.join("backups")
            }
        }

        fn files(dir: &Path) -> Vec<String> {
            super::names(dir)
        }

        fn backup_files(dir: &Path) -> Vec<String> {
            if !dir.exists() {
                return vec![];
            }
            files(dir)
        }

        #[test]
        fn restore_gives_identical_balances() {
            let f = Fixture::new();
            let before = snapshot(&f.ledger);
            let x = f.root.join("x.db");
            f.ledger.backup_to(&x).unwrap();

            let dir2 = tempfile::tempdir().unwrap();
            let mut fresh = Ledger::open(&dir2.path().join("nabungo.db")).unwrap();
            fresh
                .restore_file(&x, &dir2.path().join("backups"), NOW)
                .unwrap();
            assert_eq!(snapshot(&fresh), before);
            let mode: String = fresh
                .conn
                .pragma_query_value(None, "journal_mode", |r| r.get(0))
                .unwrap();
            assert_eq!(mode, "wal");
        }

        #[test]
        fn restore_overwrites_newer_changes_and_keeps_pre_restore() {
            let mut f = Fixture::new();
            let x = f.root.join("x.db");
            f.ledger.backup_to(&x).unwrap();
            let at_backup = snapshot(&f.ledger);
            let bca = f.bca;
            expense(&mut f.ledger, bca, 40_000);
            let with_new = snapshot(&f.ledger);
            assert_ne!(with_new, at_backup);

            let backups = f.backups();
            f.ledger.restore_file(&x, &backups, NOW).unwrap();
            assert_eq!(snapshot(&f.ledger), at_backup);

            let pre: Vec<_> = backup_files(&backups)
                .into_iter()
                .filter(|n| n.starts_with("pre-restore-"))
                .collect();
            assert_eq!(pre.len(), 1);
            f.ledger
                .restore_named(&backups, &pre[0], NOW + Duration::minutes(1))
                .unwrap();
            assert_eq!(snapshot(&f.ledger), with_new);
        }

        #[test]
        fn invalid_restore_changes_nothing() {
            let mut f = Fixture::new();
            let before = snapshot(&f.ledger);
            let txt = f.root.join("catatan.db");
            std::fs::write(&txt, "bukan database sqlite, hanya teks biasa").unwrap();
            let backups = f.backups();
            let err = f.ledger.restore_file(&txt, &backups, NOW).unwrap_err();
            assert!(matches!(err, AppError::InvalidBackup), "{err:?}");
            assert_eq!(snapshot(&f.ledger), before);
            assert!(backup_files(&backups).is_empty());
        }

        #[test]
        fn restore_old_schema_backup_migrates() {
            let dir = tempfile::tempdir().unwrap();
            let old = dir.path().join("old.db");
            let expected = {
                let mut conn = nabungo_storage::open_file(&old).unwrap();
                nabungo_storage::migrate_to(&mut conn, 4).unwrap();
                // 0005 hanya menambah index, jadi use case jalan di skema 4.
                let mut l = Ledger { conn, path: None };
                seed(&mut l);
                snapshot(&l)
            };
            assert_eq!(
                nabungo_storage::schema_version(&nabungo_storage::open_file(&old).unwrap())
                    .unwrap(),
                4
            );

            let f = Fixture::new();
            let mut ledger = f.ledger;
            ledger
                .restore_file(&old, &f.root.join("backups"), NOW)
                .unwrap();
            assert_eq!(
                nabungo_storage::schema_version(&ledger.conn).unwrap(),
                nabungo_storage::latest_version()
            );
            assert_eq!(snapshot(&ledger), expected);
        }

        #[test]
        fn restore_named_rejects_foreign_names() {
            let mut f = Fixture::new();
            let backups = f.backups();
            std::fs::create_dir_all(&backups).unwrap();
            std::fs::write(backups.join("catatan.txt"), "x").unwrap();
            let abs = f.db().to_string_lossy().into_owned();
            for name in ["../nabungo.db", abs.as_str(), "catatan.txt", "auto-x.db"] {
                let err = f.ledger.restore_named(&backups, name, NOW).unwrap_err();
                assert!(matches!(err, AppError::InvalidBackup), "{name}: {err:?}");
            }
            assert_eq!(backup_files(&backups), ["catatan.txt"]);
        }

        #[test]
        fn restore_named_oldest_pre_restore_survives_retention() {
            let mut f = Fixture::new();
            let backups = f.backups();
            let first = snapshot(&f.ledger);
            let x = f.root.join("x.db");
            f.ledger.backup_to(&x).unwrap();
            let bca = f.bca;
            for i in 0..5 {
                expense(&mut f.ledger, bca, 1_000);
                f.ledger
                    .restore_file(&x, &backups, NOW + Duration::minutes(i))
                    .unwrap();
            }
            // Pre-restore tertua = state awal; harus tetap bisa dipulihkan walau jatah penuh.
            let list = backup_files(&backups);
            assert_eq!(list.len(), 5, "{list:?}");
            let oldest = list[0].clone();
            f.ledger
                .restore_named(&backups, &oldest, NOW + Duration::hours(1))
                .unwrap();
            let mut expected = first;
            expected.0.iter_mut().find(|(n, _)| n == "BCA").unwrap().1 -= 1_000;
            expected.1.assets -= 1_000;
            expected.1.net -= 1_000;
            assert_eq!(snapshot(&f.ledger), expected);
            assert_eq!(backup_files(&backups).len(), 5);
        }

        #[test]
        fn backup_to_live_db_path_is_rejected() {
            let f = Fixture::new();
            let before = snapshot(&f.ledger);
            let db = f.db();
            let mut targets = vec![
                db.clone(),
                f.root.join("nabungo.db-wal"),
                f.root.join("nabungo.db-shm"),
                f.root.join(".").join("nabungo.db"),
            ];
            #[cfg(unix)]
            {
                let link = f.root.join("link.db");
                std::os::unix::fs::symlink(&db, &link).unwrap();
                targets.push(link);
            }
            for t in targets {
                let err = f.ledger.backup_to(&t).unwrap_err();
                assert!(
                    matches!(err, AppError::BackupTargetInvalid),
                    "{t:?}: {err:?}"
                );
            }
            assert_eq!(snapshot(&f.ledger), before);
            assert_eq!(snapshot(&Ledger::open(&db).unwrap()), before);
        }

        #[test]
        fn restore_from_live_db_is_rejected() {
            let mut f = Fixture::new();
            let (db, backups) = (f.db(), f.backups());
            let err = f.ledger.restore_file(&db, &backups, NOW).unwrap_err();
            assert!(matches!(err, AppError::BackupTargetInvalid), "{err:?}");
            assert!(backup_files(&backups).is_empty());
        }

        #[test]
        fn pre_migration_backup_only_for_existing_old_db() {
            let dir = tempfile::tempdir().unwrap();
            let backups = dir.path().join("backups");
            let fresh = dir.path().join("fresh.db");
            Ledger::open_with_backups(&fresh, &backups, NOW).unwrap();
            assert!(backup_files(&backups).is_empty());

            let old = dir.path().join("old.db");
            {
                let mut conn = nabungo_storage::open_file(&old).unwrap();
                nabungo_storage::migrate_to(&mut conn, 4).unwrap();
            }
            let l = Ledger::open_with_backups(&old, &backups, NOW).unwrap();
            let list = backup_files(&backups);
            assert_eq!(list.len(), 1);
            assert!(list[0].starts_with("pre-migration-"), "{list:?}");
            let b = nabungo_storage::open_file_read_only(&backups.join(&list[0])).unwrap();
            assert_eq!(nabungo_storage::schema_version(&b).unwrap(), 4);
            assert_eq!(
                nabungo_storage::schema_version(&l.conn).unwrap(),
                nabungo_storage::latest_version()
            );

            // Buka ulang di versi terbaru: tidak ada backup pra-migrasi baru.
            drop(l);
            Ledger::open_with_backups(&old, &backups, NOW + Duration::hours(1)).unwrap();
            assert_eq!(
                backup_files(&backups).len(),
                1,
                "{:?}",
                backup_files(&backups)
            );
        }

        #[test]
        fn auto_backup_runs_once_per_window() {
            let f = Fixture::new();
            let dir = f.backups();
            assert!(f.ledger.auto_backup_if_due(&dir, NOW).unwrap().is_some());
            assert!(
                f.ledger
                    .auto_backup_if_due(&dir, NOW + Duration::hours(1))
                    .unwrap()
                    .is_none()
            );
            let b = f
                .ledger
                .auto_backup_if_due(&dir, NOW + Duration::hours(20))
                .unwrap()
                .unwrap();
            assert_eq!(b.kind, super::BackupKind::Auto);
            assert_eq!(backup_files(&dir).len(), 2);
        }

        /// Rusak satu halaman b-tree `posting` (jumlah sel mustahil); DB harus tertutup.
        fn corrupt_posting_page(db: &Path) {
            let (root, page_size): (i64, i64) = {
                let c = nabungo_storage::open_file_read_only(db).unwrap();
                let root = c
                    .query_row(
                        "SELECT rootpage FROM sqlite_schema WHERE name = 'posting'",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                let size = c
                    .pragma_query_value(None, "page_size", |r| r.get(0))
                    .unwrap();
                (root, size)
            };
            let mut bytes = std::fs::read(db).unwrap();
            let at = usize::try_from((root - 1) * page_size).unwrap() + 3;
            bytes[at..at + 2].copy_from_slice(&[0xFF, 0xFF]);
            std::fs::write(db, bytes).unwrap();
        }

        #[test]
        fn restore_works_when_live_db_is_corrupt() {
            let f = Fixture::new();
            let before = snapshot(&f.ledger);
            let x = f.root.join("x.db");
            f.ledger.backup_to(&x).unwrap();
            let (db, backups) = (f.db(), f.backups());
            let Fixture { _dir, ledger, .. } = f;
            drop(ledger);
            corrupt_posting_page(&db);

            let mut ledger = Ledger::open(&db).unwrap();
            assert!(!matches!(ledger.integrity_check(), Ok(p) if p.is_empty()));
            ledger.restore_file(&x, &backups, NOW).unwrap();
            assert_eq!(snapshot(&ledger), before);
            assert!(ledger.integrity_check().unwrap().is_empty());
            let list = backup_files(&backups);
            assert_eq!(list.len(), 1);
            assert!(list[0].starts_with("pre-restore-"), "{list:?}");
        }

        #[test]
        fn future_dated_auto_does_not_force_hourly_backups() {
            let f = Fixture::new();
            let dir = f.backups();
            // Jam sempat maju sebulan saat app terbuka.
            f.ledger
                .auto_backup_if_due(&dir, NOW + Duration::days(30))
                .unwrap()
                .unwrap();
            assert!(f.ledger.auto_backup_if_due(&dir, NOW).unwrap().is_some());
            assert!(
                f.ledger
                    .auto_backup_if_due(&dir, NOW + Duration::hours(1))
                    .unwrap()
                    .is_none()
            );
            assert_eq!(backup_files(&dir).len(), 2);
        }

        #[test]
        fn export_json_file_rejects_live_db() {
            let f = Fixture::new();
            let before = snapshot(&f.ledger);
            for target in [f.db(), f.root.join("nabungo.db-wal")] {
                let err = f.ledger.export_json_file(&target, NOW).unwrap_err();
                assert!(matches!(err, AppError::BackupTargetInvalid), "{err:?}");
            }
            assert_eq!(snapshot(&f.ledger), before);
            assert_eq!(snapshot(&Ledger::open(&f.db()).unwrap()), before);
        }

        #[test]
        fn export_json_file_writes_complete_file() {
            let f = Fixture::new();
            let dest = f.root.join("out.json");
            std::fs::write(&dest, "lama").unwrap();
            f.ledger.export_json_file(&dest, NOW).unwrap();
            let v: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&dest).unwrap()).unwrap();
            assert_eq!(v["format"], "nabungo-export");
            assert_eq!(
                files(&f.root)
                    .iter()
                    .filter(|n| n.ends_with(".partial"))
                    .count(),
                0
            );
        }

        #[test]
        fn integrity_check_ok_on_seeded_db() {
            assert!(Fixture::new().ledger.integrity_check().unwrap().is_empty());
        }
    }
}
