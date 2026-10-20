//! Backup app-managed di folder `backups/`: penamaan, daftar, retensi, jadwal harian.

use std::path::Path;

use nabungo_storage::rusqlite::Connection;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Duration, OffsetDateTime, PrimitiveDateTime, UtcOffset};

use crate::AppError;

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
    nabungo_storage::backup_into(conn, &path)?;
    let size_bytes = std::fs::metadata(&path)?.len();
    prune(dir, kind)?;
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
}
