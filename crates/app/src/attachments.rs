//! Lampiran struk (TRX-08): isi file di `<data>/attachments/<sha256>`, metadata di DB.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use nabungo_storage::attachment::{self, AttachmentRecord};
use nabungo_storage::{now_utc, txn};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ledger::{AppError, Ledger};

pub const MAX_ATTACHMENT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_FILE_NAME: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachmentRow {
    pub id: Uuid,
    pub file_name: String,
    pub mime: String,
    pub size_bytes: u64,
    /// File `<dir>/<sha256>` tidak ada (mis. setelah restore backup lama).
    pub missing: bool,
}

/// Jenis file dari magic bytes; `None` = tidak didukung.
pub(crate) fn sniff_mime(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.len() >= 12 && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else if head.starts_with(b"%PDF-") {
        Some("application/pdf")
    } else {
        None
    }
}

pub(crate) fn ext_for(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "pdf",
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Nama file yang sah di folder lampiran: sha256 hex huruf kecil.
pub(crate) fn is_sha256(name: &str) -> bool {
    name.len() == 64 && name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `<dir>/<sha>`, atau `None` bila `sha` (dari DB, bisa berasal dari backup orang lain)
/// bukan sha256 sah — mencegah path keluar dari folder lampiran.
pub(crate) fn blob_path(dir: &Path, sha: &str) -> Option<PathBuf> {
    is_sha256(sha).then(|| dir.join(sha))
}

/// sha256 isi file (hex huruf kecil), dibaca bertahap.
pub(crate) fn sha256_file(path: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

/// Nama tampilan: nama dasar sumber, maksimal 120 karakter dengan ekstensi dipertahankan.
fn display_name(source: &Path) -> String {
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().trim().to_owned())
        .unwrap_or_default();
    if name.is_empty() {
        return "lampiran".to_owned();
    }
    if name.chars().count() <= MAX_FILE_NAME {
        return name;
    }
    let ext = match name.rfind('.') {
        Some(i) if i > 0 && name[i..].chars().count() <= 10 => &name[i..],
        _ => "",
    };
    let keep = MAX_FILE_NAME - ext.chars().count();
    let stem: String = name[..name.len() - ext.len()].chars().take(keep).collect();
    format!("{stem}{ext}")
}

struct Checked {
    path: PathBuf,
    mime: &'static str,
    size: u64,
}

fn check_source(path: &Path) -> Result<Checked, AppError> {
    let size = fs::metadata(path)?.len();
    if size > MAX_ATTACHMENT_BYTES {
        return Err(AppError::AttachmentTooLarge);
    }
    let mut head = Vec::with_capacity(16);
    File::open(path)?.take(16).read_to_end(&mut head)?;
    let mime = sniff_mime(&head).ok_or(AppError::AttachmentType)?;
    Ok(Checked {
        path: path.to_owned(),
        mime,
        size,
    })
}

/// Salin ke `<dir>/<sha256>` lewat file `.partial`. `true` bila file baru dibuat
/// (konten yang sama sudah ada → tidak ditulis ulang).
fn store(source: &Path, dir: &Path) -> io::Result<(String, bool)> {
    let partial = dir.join(format!("{}.partial", Uuid::now_v7()));
    let result = (|| {
        let mut out = File::create(&partial)?;
        let mut hasher = Sha256::new();
        let mut input = File::open(source)?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            io::Write::write_all(&mut out, &buf[..n])?;
        }
        out.sync_all()?;
        Ok::<_, io::Error>(hex(&hasher.finalize()))
    })();
    let sha = match result {
        Ok(sha) => sha,
        Err(e) => {
            let _ = fs::remove_file(&partial);
            return Err(e);
        }
    };
    let target = dir.join(&sha);
    if target.exists() {
        fs::remove_file(&partial)?;
        return Ok((sha, false));
    }
    if let Err(e) = fs::rename(&partial, &target) {
        let _ = fs::remove_file(&partial);
        return Err(e);
    }
    Ok((sha, true))
}

fn row(rec: AttachmentRecord, dir: &Path) -> AttachmentRow {
    AttachmentRow {
        missing: !blob_path(dir, &rec.sha256).is_some_and(|p| p.is_file()),
        id: rec.id,
        file_name: rec.file_name,
        mime: rec.mime,
        size_bytes: rec.size_bytes,
    }
}

impl Ledger {
    /// Lampirkan file ke transaksi aktif. Semua sumber divalidasi dulu; satu yang tidak
    /// valid membatalkan semuanya. Mengembalikan baris yang baru ditambah.
    pub fn add_attachments(
        &mut self,
        txn_id: Uuid,
        sources: &[PathBuf],
        dir: &Path,
    ) -> Result<Vec<AttachmentRow>, AppError> {
        match txn::get_txn(&self.conn, txn_id)? {
            Some(t) if !t.deleted => {}
            _ => return Err(AppError::NotFound("transaction")),
        }
        let checked = sources
            .iter()
            .map(|p| check_source(p))
            .collect::<Result<Vec<_>, _>>()?;
        fs::create_dir_all(dir)?;
        let mut created = Vec::new();
        let mut records = Vec::new();
        let copied = (|| {
            for c in &checked {
                let (sha, new) = store(&c.path, dir)?;
                if new {
                    created.push(dir.join(&sha));
                }
                records.push(AttachmentRecord {
                    id: Uuid::now_v7(),
                    txn_id,
                    file_name: display_name(&c.path),
                    sha256: sha,
                    mime: c.mime.to_owned(),
                    size_bytes: c.size,
                    created_at: now_utc(),
                });
            }
            Ok::<_, AppError>(())
        })();
        let saved = copied.and_then(|()| {
            let tx = self.conn.transaction()?;
            for rec in &records {
                attachment::insert_attachment(&tx, rec)?;
            }
            tx.commit()?;
            Ok(())
        });
        if let Err(e) = saved {
            // File yang baru dibuat belum dirujuk baris mana pun.
            for path in &created {
                let _ = fs::remove_file(path);
            }
            return Err(e);
        }
        Ok(records.into_iter().map(|r| row(r, dir)).collect())
    }

    pub fn list_attachments(
        &self,
        txn_id: Uuid,
        dir: &Path,
    ) -> Result<Vec<AttachmentRow>, AppError> {
        Ok(attachment::list_for_txn(&self.conn, txn_id)?
            .into_iter()
            .map(|r| row(r, dir))
            .collect())
    }

    /// Hapus baris; file ikut dihapus bila tidak dirujuk baris lain (termasuk milik
    /// transaksi terhapus).
    pub fn remove_attachment(&mut self, id: Uuid, dir: &Path) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        let rec = attachment::get_attachment(&tx, id)?.ok_or(AppError::NotFound("attachment"))?;
        attachment::delete_attachment(&tx, id)?;
        let refs = attachment::sha_ref_count(&tx, &rec.sha256)?;
        tx.commit()?;
        if let Some(path) = blob_path(dir, &rec.sha256).filter(|_| refs == 0) {
            match fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => {
                    tracing::warn!(error = %e, "failed to remove attachment file");
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Salinan lampiran berekstensi (`<open_dir>/<sha256>.<ext>`) untuk dibuka viewer OS:
    /// blob di `dir` tanpa ekstensi, dan Windows memilih aplikasi dari ekstensi. Salinan, bukan
    /// hardlink, agar viewer yang menyimpan perubahan tidak mengubah blob. Salinan yang sudah ada
    /// dengan ukuran sama dipakai ulang (Windows mengunci file yang sedang dibuka viewer).
    pub fn attachment_open_path(
        &self,
        id: Uuid,
        dir: &Path,
        open_dir: &Path,
    ) -> Result<PathBuf, AppError> {
        let rec =
            attachment::get_attachment(&self.conn, id)?.ok_or(AppError::NotFound("attachment"))?;
        let blob = blob_path(dir, &rec.sha256)
            .filter(|p| p.is_file())
            .ok_or(AppError::AttachmentMissing)?;
        let dest = open_dir.join(format!("{}.{}", rec.sha256, ext_for(&rec.mime)));
        let size = fs::metadata(&blob)?.len();
        if fs::metadata(&dest).is_ok_and(|m| m.len() == size) {
            return Ok(dest);
        }
        fs::create_dir_all(open_dir)?;
        fs::copy(&blob, &dest)?;
        Ok(dest)
    }
}

/// Kosongkan folder salinan lampiran (best-effort, dipanggil saat start).
pub fn clear_open_cache(open_dir: &Path) {
    match fs::remove_dir_all(open_dir) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => {
            tracing::warn!(error = %e, "failed to clear attachment open cache");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::Kit;
    use std::fs;
    use time::macros::date;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const PDF: &[u8] = b"%PDF-1.4\n%%EOF\n";

    struct Fx {
        kit: Kit,
        txn: Uuid,
        src: tempfile::TempDir,
        dir: tempfile::TempDir,
    }

    fn fx() -> Fx {
        let mut kit = Kit::new(1, date!(2026 - 10 - 01));
        let txn = kit.expense(date!(2026 - 10 - 05), "Kopi & jajan", 10_000);
        Fx {
            kit,
            txn,
            src: tempfile::tempdir().unwrap(),
            dir: tempfile::tempdir().unwrap(),
        }
    }

    impl Fx {
        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let p = self.src.path().join(name);
            fs::write(&p, bytes).unwrap();
            p
        }

        fn add(&mut self, txn: Uuid, files: &[PathBuf]) -> Result<Vec<AttachmentRow>, AppError> {
            let dir = self.dir.path().to_owned();
            self.kit.ledger.add_attachments(txn, files, &dir)
        }

        fn stored(&self) -> Vec<String> {
            let mut names: Vec<_> = fs::read_dir(self.dir.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }
    }

    #[test]
    fn sniff_detects_supported_types() {
        assert_eq!(sniff_mime(b"\xff\xd8\xff\xe0abc"), Some("image/jpeg"));
        assert_eq!(sniff_mime(PNG), Some("image/png"));
        assert_eq!(sniff_mime(b"RIFF\x10\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff_mime(PDF), Some("application/pdf"));
        assert_eq!(sniff_mime(b"hello"), None);
        assert_eq!(sniff_mime(b""), None);
        assert_eq!(sniff_mime(b"RIFF\0\0\0\0WAVE"), None);
        assert_eq!(ext_for("image/jpeg"), "jpg");
        assert_eq!(ext_for("application/pdf"), "pdf");
    }

    #[test]
    fn add_copies_content_addressed() {
        let mut f = fx();
        let p = f.file("struk.png", PNG);
        let rows = f.add(f.txn, std::slice::from_ref(&p)).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.file_name, "struk.png");
        assert_eq!(r.mime, "image/png");
        assert_eq!(r.size_bytes, PNG.len() as u64);
        assert!(!r.missing);
        let sha = sha256_file(&p).unwrap();
        assert_eq!(sha.len(), 64);
        assert_eq!(f.stored(), vec![sha.clone()]);
        assert_eq!(fs::read(f.dir.path().join(&sha)).unwrap(), PNG);
        assert_eq!(
            f.kit.ledger.list_attachments(f.txn, f.dir.path()).unwrap(),
            rows
        );
    }

    #[test]
    fn add_rejects_type_size_and_deleted_txn() {
        let mut f = fx();
        let txt = f.file("a.txt", b"hello");
        assert!(matches!(
            f.add(f.txn, std::slice::from_ref(&txt)),
            Err(AppError::AttachmentType)
        ));

        let mut big = PDF.to_vec();
        big.resize(usize::try_from(MAX_ATTACHMENT_BYTES).unwrap() + 1, b'x');
        let big = f.file("big.pdf", &big);
        assert!(matches!(
            f.add(f.txn, &[big]),
            Err(AppError::AttachmentTooLarge)
        ));

        let png = f.file("s.png", PNG);
        assert!(matches!(
            f.add(f.txn, &[png.clone(), txt]),
            Err(AppError::AttachmentType)
        ));
        assert!(f.stored().is_empty());
        assert!(
            f.kit
                .ledger
                .list_attachments(f.txn, f.dir.path())
                .unwrap()
                .is_empty()
        );

        f.kit.ledger.delete_txn(f.txn).unwrap();
        assert!(matches!(
            f.add(f.txn, &[png]),
            Err(AppError::NotFound("transaction"))
        ));
        assert!(f.stored().is_empty());
    }

    #[test]
    fn long_file_name_truncated_keeping_extension() {
        let mut f = fx();
        let name = format!("{}.pdf", "a".repeat(200));
        let p = f.file(&name, PDF);
        let row = &f.add(f.txn, &[p]).unwrap()[0];
        assert!(row.file_name.chars().count() <= 120);
        assert!(row.file_name.ends_with(".pdf"));
        assert!(row.file_name.starts_with("aaa"));
    }

    #[test]
    fn shared_content_survives_first_removal() {
        let mut f = fx();
        let other = f.kit.expense(date!(2026 - 10 - 06), "Kopi & jajan", 5_000);
        let p = f.file("s.png", PNG);
        let a = f.add(f.txn, std::slice::from_ref(&p)).unwrap()[0].id;
        let b = f.add(other, &[p]).unwrap()[0].id;
        assert_eq!(f.stored().len(), 1);
        f.kit.ledger.remove_attachment(a, f.dir.path()).unwrap();
        assert_eq!(f.stored().len(), 1);
        f.kit.ledger.remove_attachment(b, f.dir.path()).unwrap();
        assert!(f.stored().is_empty());
        assert!(matches!(
            f.kit.ledger.remove_attachment(b, f.dir.path()),
            Err(AppError::NotFound("attachment"))
        ));
    }

    #[test]
    fn missing_file_is_flagged_and_open_fails() {
        let mut f = fx();
        let p = f.file("s.pdf", PDF);
        let row = f.add(f.txn, std::slice::from_ref(&p)).unwrap().remove(0);
        let open = tempfile::tempdir().unwrap();
        fs::remove_file(f.dir.path().join(sha256_file(&p).unwrap())).unwrap();
        let rows = f.kit.ledger.list_attachments(f.txn, f.dir.path()).unwrap();
        assert!(rows[0].missing);
        assert!(matches!(
            f.kit
                .ledger
                .attachment_open_path(row.id, f.dir.path(), open.path()),
            Err(AppError::AttachmentMissing)
        ));
    }

    #[test]
    fn open_copy_has_extension_and_same_bytes() {
        let mut f = fx();
        let open = tempfile::tempdir().unwrap();
        let p = f.file("s.pdf", PDF);
        let row = f.add(f.txn, std::slice::from_ref(&p)).unwrap().remove(0);
        let sha = sha256_file(&p).unwrap();
        let got = f
            .kit
            .ledger
            .attachment_open_path(row.id, f.dir.path(), open.path())
            .unwrap();
        assert_eq!(got, open.path().join(format!("{sha}.pdf")));
        assert_eq!(fs::read(&got).unwrap(), PDF);
        assert!(f.dir.path().join(&sha).is_file());
    }

    #[test]
    fn open_copy_reused_when_present() {
        // Windows mengunci file yang sedang dibuka viewer; salinan yang ada dipakai ulang.
        let mut f = fx();
        let open = tempfile::tempdir().unwrap();
        let p = f.file("s.png", PNG);
        let row = f.add(f.txn, &[p]).unwrap().remove(0);
        let first = f
            .kit
            .ledger
            .attachment_open_path(row.id, f.dir.path(), open.path())
            .unwrap();
        assert!(first.extension().is_some_and(|e| e == "png"));
        let mut perms = fs::metadata(&first).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&first, perms).unwrap();
        let second = f
            .kit
            .ledger
            .attachment_open_path(row.id, f.dir.path(), open.path())
            .unwrap();
        assert_eq!(second, first);
        assert_eq!(fs::read(&second).unwrap(), PNG);
    }

    #[test]
    fn open_cache_created_when_missing() {
        let mut f = fx();
        let root = tempfile::tempdir().unwrap();
        let open = root.path().join("cache").join("open");
        let p = f.file("s.pdf", PDF);
        let row = f.add(f.txn, &[p]).unwrap().remove(0);
        let got = f
            .kit
            .ledger
            .attachment_open_path(row.id, f.dir.path(), &open)
            .unwrap();
        assert!(got.starts_with(&open) && got.is_file());
    }

    #[test]
    fn clear_open_cache_removes_files_and_ignores_missing_dir() {
        let open = tempfile::tempdir().unwrap();
        fs::write(open.path().join("a.pdf"), PDF).unwrap();
        let ro = open.path().join("b.png");
        fs::write(&ro, PNG).unwrap();
        let mut perms = fs::metadata(&ro).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&ro, perms).unwrap();
        clear_open_cache(open.path());
        assert!(!open.path().exists());
        clear_open_cache(&open.path().join("tidak-ada"));
    }

    #[test]
    fn tampered_sha_never_escapes_attachments_dir() {
        // Backup .db dari pihak lain bisa berisi sha256 berupa path; jangan pernah disentuh.
        let mut f = fx();
        let p = f.file("s.png", PNG);
        let row = f.add(f.txn, &[p]).unwrap().remove(0);
        let outside = tempfile::tempdir().unwrap();
        let open = tempfile::tempdir().unwrap();
        let victim = outside.path().join("dokumen.txt");
        fs::write(&victim, b"penting").unwrap();
        for sha in [
            victim.to_str().unwrap().to_owned(),
            "../dokumen.txt".to_owned(),
        ] {
            f.kit
                .ledger
                .conn
                .execute("UPDATE attachment SET sha256 = ?1", [&sha])
                .unwrap();
            let rows = f.kit.ledger.list_attachments(f.txn, f.dir.path()).unwrap();
            assert!(rows[0].missing, "{sha}");
            assert!(matches!(
                f.kit
                    .ledger
                    .attachment_open_path(row.id, f.dir.path(), open.path()),
                Err(AppError::AttachmentMissing)
            ));
        }
        assert_eq!(fs::read_dir(open.path()).unwrap().count(), 0);
        fs::copy(&victim, f.dir.path().join("../dokumen.txt")).unwrap();
        f.kit
            .ledger
            .remove_attachment(row.id, f.dir.path())
            .unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"penting");
        assert!(f.dir.path().join("../dokumen.txt").exists());
    }

    #[test]
    fn soft_delete_keeps_attachments() {
        let mut f = fx();
        let p = f.file("s.png", PNG);
        f.add(f.txn, &[p]).unwrap();
        f.kit.ledger.delete_txn(f.txn).unwrap();
        f.kit.ledger.undo_delete_txn(f.txn).unwrap();
        let rows = f.kit.ledger.list_attachments(f.txn, f.dir.path()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].missing);
        assert_eq!(f.stored().len(), 1);
    }
}
