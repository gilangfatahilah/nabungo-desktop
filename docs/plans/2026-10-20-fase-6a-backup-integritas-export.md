# Fase 6a — Backup, Integritas, Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:**
- Data user tidak hilang:
  - backup otomatis harian (retensi 14), backup sebelum setiap migrasi, dan backup manual ke lokasi pilihan user;
  - restore dari daftar backup atau dari file, yang menghasilkan saldo identik;
  - `PRAGMA integrity_check` saat startup bila sesi sebelumnya tidak ditutup normal.
- User bisa mengekspor transaksi, akun, kategori, dan budget ke JSON atau CSV.
- Force-kill saat menulis tidak menghasilkan saldo korup (exit criterion pertama Fase 6).

**Architecture:**
- `nabungo-storage` mendapat primitif file database, satu-satunya tempat file DB dibuka:
  - `open_file`, `migrate`, `schema_version`;
  - `backup_into` / `restore_from` (SQLite online backup API, fitur rusqlite `backup`);
  - `inspect_backup`, `integrity_check`;
  - query ekspor.
- `nabungo-app` mendapat:
  - `backup.rs` — penamaan file, daftar, retensi, jatuh tempo backup harian, restore, backup pra-migrasi lewat `Ledger::open_with_backups`;
  - `export.rs` — JSON dan CSV.
- `src-tauri` mendapat `lifecycle.rs`:
  - penanda sesi untuk mendeteksi shutdown tidak normal;
  - laporan startup;
  - pengecekan backup otomatis tiap jam di thread latar.
- Command baru membuka dialog file native (tauri-plugin-dialog, dipanggil dari Rust) lalu meneruskan path ke `nabungo-app`.
- UI: tiga bagian di Pengaturan (Umum, Backup & pemulihan, Ekspor data) dan dialog peringatan integritas di `App.vue`.

**Tech Stack:**
- Rust:
  - rusqlite 0.40.2 dengan fitur `backup`;
  - `csv = "1.4.0"`, `serde`, `serde_json` di `nabungo-app`;
  - `tauri-plugin-dialog = "2.8.3"` di `src-tauri` (hanya API Rust, tanpa paket JS dan tanpa permission capability).
- Vue: komponen shadcn-vue yang sudah ada.
- Test: Vitest dan cargo test (termasuk integration test proses anak untuk force-kill).

**Spec:** `PRD.md`:
- §5: DAT-01, DAT-02; acceptance "Backup bisa dipulihkan"
- §8: baris Integritas dan Backup
- §9 Pengaturan ("backup & restore, import/export")
- §11 risiko "Kehilangan data"

Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 6 (poin DAT-01, integrity_check, DAT-02, dan exit criterion force-kill). Desain: `docs/design/ui-guidelines.md` §7.2, §8.1, §8.8, §8.9.

**Di luar plan ini — Fase 6b (plan terpisah setelah 6a di-merge):**
- SEC-01: `bundled-sqlcipher`, master password (Argon2id), recovery key, auto-lock, enkripsi opsional saat onboarding;
- exit criterion kedua: DB terenkripsi tidak terbaca tanpa password.

6a menyiapkan satu titik masuk (`nabungo_storage::open_file`) tempat key akan dipasang, termasuk untuk file backup.

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (core, storage)`. `src-tauri` hanya bergantung pada `nabungo-app` dan crate Tauri. Tipe yang dibutuhkan DTO di-re-export dari `nabungo_app`.
- Semua pembukaan file database (DB utama, file backup tujuan, file backup sumber) lewat `nabungo_storage::open_file` / `open_file_read_only`. Tidak ada `Connection::open` di tempat lain.
- File DB user tidak pernah dihapus, di-rename, atau ditimpa. Restore menyalin isi backup **ke dalam** koneksi yang hidup lewat backup API (atomik di sisi tujuan), setelah membuat backup `pre-restore`.
- Retensi hanya menghapus file di `<data_dir>/backups/` yang namanya cocok dengan pola backup Nabungo. File lain tidak disentuh.
- Migrasi `0001`–`0005` tidak diubah. Fase 6a tidak menambah migrasi.
- Offline: tidak ada network call. Dialog file memakai dialog native OS.
- Satu use case tulis = satu SQLite transaction. Backup dan restore memegang `Mutex<Ledger>` selama prosesnya. Lock tidak boleh ditahan melintasi `.await`: dialog dibuka dulu, baru lock diambil.
- Waktu masuk sebagai parameter (`now: OffsetDateTime`, `today: Date`), tidak dibaca di dalam `nabungo-app`, agar bisa dites.
- Semua command baru didaftarkan di `collect_commands!`. `ui/src/ipc/bindings.ts` di-regenerate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- Semua teks UI ada di `id.ts` dan `en.ts`. Setiap `ErrorCode` baru masuk `errors` di kedua file.
- Komponen hanya memanggil `ui/src/ipc/index.ts`. `browser-ledger.ts` mengimplementasikan setiap command baru dengan aturan yang sama (backup = snapshot state in-memory).
- UI mengikuti `docs/design/ui-guidelines.md`.
- Setiap task diakhiri dengan perintah berikut, semuanya hijau:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `pnpm --filter @nabungo/ui typecheck`
  - `pnpm --filter @nabungo/ui test`

## Keputusan desain (berlaku untuk semua task)

| # | Keputusan | Alasan |
|---|-----------|--------|
| K1 | Backup app-managed disimpan di `<data_dir>/backups/` dengan nama `{kind}-{YYYYMMDD}T{HHMMSS}Z.db`. `kind` ∈ `auto`, `pre-migration`, `pre-restore`. Bila nama sudah ada, ditambah sufiks `-1`, `-2`, … sebelum `.db`. Waktu dalam UTC; UI menampilkannya dalam waktu lokal. | Nama bisa diurutkan dan di-parse tanpa metadata terpisah. |
| K2 | Retensi per kind: `auto` 14, `pre-migration` 5, `pre-restore` 5. Yang dihapus selalu yang terlama. | PRD: retensi 14 untuk backup harian; jenis lain jarang terjadi tetapi tetap tidak boleh menumpuk. |
| K3 | "Backup sekarang" menyimpan ke path pilihan user lewat dialog simpan (nama default `nabungo-backup-YYYY-MM-DD.db`) dan tidak dikenai retensi. Daftar di Pengaturan hanya menampilkan isi folder `backups/`. | Backup manual dimaksudkan untuk disalin keluar perangkat; app tidak mengelola file milik user. |
| K4 | Backup otomatis jatuh tempo bila belum ada backup `auto`, bila ≥ 20 jam sejak yang terbaru, atau bila yang terbaru tercatat di masa depan (jam sistem mundur). Dicek saat startup, lalu tiap 1 jam di thread latar. | "Harian" tetap terpenuhi untuk user yang membuka app pada jam yang sedikit berbeda setiap hari. |
| K5 | Setiap backup ditulis ke `<dest>.partial`, diperiksa dengan `PRAGMA quick_check`, lalu di-rename ke `dest`. Bila gagal, partial dihapus dan retensi tidak berjalan. | Backup rusak tidak pernah menggeser backup baik keluar dari retensi. |
| K6 | Backup pra-migrasi dibuat di `Ledger::open_with_backups` bila `0 < schema_version < latest`. DB baru (versi 0) tidak di-backup. | PRD: "backup sebelum setiap migrasi". |
| K7 | Restore: (1) `inspect_backup` (bisa dibuka read-only, `quick_check` ok, punya tabel `posting`, versi ≤ latest); (2) backup `pre-restore`; (3) `restore_from` ke koneksi hidup; (4) `migrate`. Gagal di langkah 1 → tidak ada yang berubah dan tidak ada file baru. Setelah sukses, UI memuat ulang halaman (`location.reload()`). | Semua store UI kembali konsisten tanpa invalidasi manual; user selalu bisa membatalkan restore dengan memulihkan backup `pre-restore`. |
| K8 | Penanda sesi `<data_dir>/session.lock` dibuat setelah DB terbuka dan dihapus di `RunEvent::Exit`. Bila penanda sudah ada saat startup, sesi sebelumnya dianggap tidak normal dan `PRAGMA integrity_check` (penuh) dijalankan. | Sesuai PRD §8; integrity check penuh mahal, jadi hanya dijalankan setelah crash. |
| K9 | Integrity gagal → masalah dicatat di log, backup otomatis **tidak** dijalankan selama sesi itu, dan UI menampilkan dialog yang mengarahkan ke Pengaturan → Backup & pemulihan. DB yang gagal dibuka sama sekali tetap fatal seperti sekarang (dicatat di log), dan layar pemulihan untuk kasus itu ditunda ke Fase 8. | Backup dari DB rusak akan menggeser backup sehat keluar dari retensi. Mode tanpa `Ledger` menyentuh setiap command. |
| K10 | Backup atau restore ke path DB utama atau file `-wal`/`-shm`-nya ditolak (`backup_target_invalid`). `Ledger` menyimpan path-nya (`None` untuk in-memory). | Dialog simpan bisa menunjuk `nabungo.db`, dan rename di atas DB yang sedang terbuka merusaknya. |
| K11 | Restore dari daftar menerima `file_name`, bukan path. Nama dengan pemisah path, `..`, atau yang tidak cocok pola K1 ditolak (`backup_invalid`). | UI tidak bisa membuat backend membaca file di luar `backups/`. |
| K12 | Ekspor JSON: satu file dengan `{ "format": "nabungo-export", "version": 1, "exported_at", "app_version", "accounts", "categories", "budgets", "transactions" }`. Transaksi memuat `postings` (`account_id`, `category_id`, `amount`, `currency`, `memo`). Hanya transaksi `deleted_at IS NULL`. Nominal dalam satuan minor (integer). | Lengkap dan stabil untuk import v2 (DAT-03). |
| K13 | Ekspor CSV ditulis ke subfolder baru `nabungo-export-YYYY-MM-DD` (sufiks `-2`, `-3`, … bila sudah ada) di folder pilihan user, berisi `transactions.csv` (satu baris per posting), `accounts.csv`, `categories.csv`, `budgets.csv`. Format: UTF-8 dengan BOM, koma, CRLF. Kategori child ditulis `Parent / Child`. | Tidak pernah menimpa file user; bisa langsung dibuka Excel/LibreOffice dengan karakter non-ASCII benar. |
| K14 | Sel teks CSV yang diawali `=`, `+`, `-`, `@`, tab, atau CR diberi awalan `'`. Kolom angka tidak diubah. | Mencegah formula injection lewat payee/catatan saat file dibuka di spreadsheet. |
| K15 | Saldo di `accounts.csv`/JSON dihitung saat ekspor dari postings (kolom `balance`), tidak dibaca dari kolom tersimpan. | Model data: saldo selalu diturunkan. |

## Error code baru

Ditambahkan ke `ErrorCode` (src-tauri), ke `errors` di `id.ts` dan `en.ts`, dan ke `browser-ledger.ts` bila relevan:

| Code | id | en |
|------|----|----|
| `backup_invalid` | File ini bukan backup Nabungo yang valid | This file is not a valid Nabungo backup |
| `backup_too_new` | Backup ini dibuat oleh versi Nabungo yang lebih baru | This backup was made by a newer version of Nabungo |
| `backup_target_invalid` | Tidak bisa memakai file database yang sedang dibuka | Can't use the database file that is currently open |
| `file_io` | Gagal membaca atau menulis file | Could not read or write the file |

## Review Focus

1. **Backup dari DB rusak menggeser backup sehat.** Bila integrity check startup gagal, atau `quick_check` pada file backup gagal, tidak ada file baru yang disimpan dan retensi tidak menghapus apa pun.
   → test `failed_check_keeps_existing_backups` (Task 2) dan `auto_backup_not_allowed_after_failed_integrity` (Task 5).
2. **Dialog simpan menunjuk DB utama.** User memilih `nabungo.db` (atau path yang sama lewat symlink) sebagai tujuan backup manual: harus ditolak tanpa menyentuh DB.
   → test `backup_to_live_db_path_is_rejected` (Task 3).
3. **Nama file restore yang direkayasa.** `../nabungo.db`, path absolut, `auto-x.db`, atau nama file non-backup di folder `backups/` ditolak `backup_invalid`.
   → test `restore_named_rejects_foreign_names` (Task 3).
4. **Restore backup skema lama.** Backup dari versi skema lebih rendah (mis. sebelum `0005`) dipulihkan lalu dimigrasi ke versi terbaru, dengan saldo identik.
   → test `restore_old_schema_backup_migrates` (Task 3).
5. **Payee/catatan berisi koma, kutip, baris baru, atau `=HYPERLINK(...)`.** CSV tetap satu record per baris logis dan formula dinetralkan, sementara nominal negatif tetap `-15000`.
   → test `csv_escapes_and_neutralizes_formulas` (Task 6).

---

## Bagian A — Primitif & use case backup (DAT-01)

### Task 1: Storage — file DB, backup API, inspeksi, integrity

**Files:**
- Modify: `crates/storage/Cargo.toml` (rusqlite features `["bundled", "backup"]`; dev `tempfile = "3"`)
- Modify: `crates/storage/src/lib.rs`
- Create: `crates/storage/src/file.rs` (re-export dari `lib.rs`)

**Interfaces:**
- Produces (semua di-re-export dari root `nabungo_storage`):
  - `pub fn open_file(path: &Path) -> Result<Connection>`: `Connection::open` lalu `configure` (PRAGMA yang sudah ada), **tanpa** migrasi.
  - `pub fn open_file_read_only(path: &Path) -> Result<Connection>`: `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`, tanpa PRAGMA tulis.
  - `pub fn open(path)` tetap ada, sekarang `open_file` + `migrate`.
  - `pub fn migrate(conn: &mut Connection) -> Result<()>`
  - `pub fn migrate_to(conn: &mut Connection, version: usize) -> Result<()>`: untuk test dan tool (`Migrations::to_version`).
  - `pub fn schema_version(conn: &Connection) -> Result<usize>`: membaca `user_version`.
  - `pub fn latest_version() -> usize`: jumlah migrasi di `migrations()`.
  - `pub fn backup_into(conn: &Connection, dest: &Path) -> Result<()>`: aturan K5. Memakai `Backup::new(conn, &mut dst).run_to_completion(-1, Duration::ZERO, None)`; `dst` dibuka dengan `open_file`.
  - `pub fn restore_from(conn: &mut Connection, src: &Path) -> Result<()>`: `src` lewat `open_file_read_only`, satu step `-1`.
  - `pub fn inspect_backup(path: &Path) -> Result<usize>`: mengembalikan versi skema. Error:
    - `StorageError::InvalidBackup` bila file tidak bisa dibuka atau bukan SQLite, `quick_check` ≠ `ok`, atau tidak ada tabel `posting`;
    - `StorageError::BackupTooNew(usize)` bila versi > latest.
  - `pub fn integrity_check(conn: &Connection) -> Result<Vec<String>>`: baris hasil `PRAGMA integrity_check`; kosong bila hasilnya tepat `["ok"]`.
  - Varian baru `StorageError::InvalidBackup`, `StorageError::BackupTooNew(usize)`, `StorageError::Io(#[from] std::io::Error)`.

- [ ] **Step 1: Write the failing tests** (di `file.rs`, memakai `tempfile` sebagai dev-dependency storage; seed lewat SQL langsung ke tabel `setting` cukup)

```rust
#[test]
fn backup_roundtrip_preserves_rows() // open → insert setting 'k'='v' → backup_into(dir/b.db) → open(b.db) membaca 'v'
#[test]
fn backup_includes_committed_wal_frames() // tulis tanpa checkpoint, backup, isi ada di backup
#[test]
fn backup_failure_leaves_no_file() // dest di folder yang tidak ada → Err; tidak ada dest maupun dest.partial
#[test]
fn restore_replaces_contents() // A berisi 'k'='a', B berisi 'k'='b'; restore_from(A, B) → A membaca 'b'
#[test]
fn inspect_rejects_non_backups() {
    // file teks, 0 byte, SQLite kosong tanpa tabel posting → InvalidBackup
    // user_version = latest_version()+1 → BackupTooNew(latest+1)
    // DB hasil open() → Ok(latest_version())
}
#[test]
fn integrity_check_ok_on_fresh_db() { assert!(integrity_check(&open_in_memory().unwrap()).unwrap().is_empty()) }
#[test]
fn migrate_to_then_migrate_reaches_latest() // migrate_to(3) → schema_version 3 → migrate → latest_version()
```

- [ ] **Step 2: Run** `cargo test -p nabungo-storage file::` → FAIL (fungsi belum ada).
- [ ] **Step 3: Implement** sesuai Interfaces. `open_in_memory()` tetap memakai `configure` + `migrate`.
- [ ] **Step 4: Run** `cargo test -p nabungo-storage` → PASS.
- [ ] **Step 5: Commit** `feat(storage): primitif backup, restore, dan integrity check`

### Task 2: App — penamaan, daftar, retensi, jatuh tempo backup

**Files:**
- Create: `crates/app/src/backup.rs` (modul, re-export tipe publik dari `lib.rs`)
- Modify: `crates/app/Cargo.toml` (`time` features `formatting`, `parsing`; dev `tempfile = "3"`)

**Interfaces:**
- Consumes: `nabungo_storage::backup_into` (Task 1).
- Produces:
  - `pub enum BackupKind { Auto, PreMigration, PreRestore }` dengan `as_str()` → `auto`/`pre-migration`/`pre-restore`.
  - `pub struct BackupInfo { pub file_name: String, pub kind: BackupKind, pub created_at: OffsetDateTime, pub size_bytes: u64 }`
  - `pub fn parse_backup_name(name: &str) -> Option<(BackupKind, OffsetDateTime)>`: pola K1, termasuk sufiks `-N`.
  - `pub fn list_backups(dir: &Path) -> Result<Vec<BackupInfo>, AppError>`: terbaru dulu. Folder belum ada → `[]`. File yang tidak cocok pola diabaikan.
  - `pub fn is_auto_due(last: Option<OffsetDateTime>, now: OffsetDateTime) -> bool`: aturan K4.
  - `pub(crate) fn write_backup(conn: &Connection, dir: &Path, kind: BackupKind, now: OffsetDateTime) -> Result<BackupInfo, AppError>`: membuat `dir` bila perlu, memilih nama unik (K1), `backup_into`, lalu `prune(dir, kind)` hanya bila sukses.
  - `pub(crate) fn prune(dir: &Path, kind: BackupKind) -> Result<(), AppError>`: retensi K2.
  - `AppError::Io(#[from] std::io::Error)`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn name_roundtrip() {
    // write_backup pada 2026-10-20 03:15:00 UTC → "auto-20261020T031500Z.db"
    // panggilan kedua di detik yang sama → "auto-20261020T031500Z-1.db"
    assert!(parse_backup_name("pre-migration-20261020T031500Z-2.db").is_some());
    for bad in ["auto-2026.db", "../auto-20261020T031500Z.db", "manual-20261020T031500Z.db", "auto-20261020T031500Z.sqlite"] {
        assert!(parse_backup_name(bad).is_none(), "{bad}");
    }
}
#[test]
fn auto_due_rules() {
    // None → true; 19 jam lalu → false; 20 jam lalu → true; 1 jam di masa depan → true
}
#[test]
fn prune_keeps_newest_per_kind() // 16 auto + 6 pre-restore → tersisa 14 auto terbaru + 5 pre-restore terbaru
#[test]
fn prune_ignores_foreign_files() // "catatan.txt" dan "nabungo-backup-2026-10-01.db" di dir tetap ada
#[test]
fn list_is_newest_first_with_sizes() // size_bytes > 0, urutan created_at menurun, dir tidak ada → []
#[test]
fn failed_check_keeps_existing_backups() {
    // 14 auto sudah ada; write_backup ke dir yang dibuat read-only (atau dest yang gagal dibuka)
    // → Err; tetap 14 file yang sama
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app backup::` → FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces. Format waktu memakai `time::format_description` statis `[year][month][day]T[hour][minute][second]Z`.
- [ ] **Step 4: Run** `cargo test -p nabungo-app backup::` → PASS.
- [ ] **Step 5: Commit** `feat(app): penamaan, retensi, dan jadwal backup`

### Task 3: App — operasi backup/restore pada `Ledger`

**Files:**
- Modify: `crates/app/src/ledger.rs` (field `path: Option<PathBuf>`, `open_with_backups`, varian error)
- Modify: `crates/app/src/backup.rs` (blok `impl Ledger`)

**Interfaces:**
- Consumes: Task 1 dan Task 2.
- Produces:
  - `Ledger::open_with_backups(path: &Path, backup_dir: &Path, now: OffsetDateTime) -> Result<Self, AppError>`: `open_file` → backup `PreMigration` bila `0 < version < latest` (K6) → `migrate`. `Ledger::open(path)` tetap ada (tanpa backup) untuk test.
  - `Ledger::backup_to(&self, dest: &Path) -> Result<(), AppError>`: backup manual K3; K10 → `AppError::BackupTargetInvalid`.
  - `Ledger::auto_backup_if_due(&self, dir: &Path, now: OffsetDateTime) -> Result<Option<BackupInfo>, AppError>`
  - `Ledger::restore_file(&mut self, src: &Path, backup_dir: &Path, now: OffsetDateTime) -> Result<(), AppError>`: aturan K7; K10 untuk `src` = DB utama.
  - `Ledger::restore_named(&mut self, backup_dir: &Path, file_name: &str, now: OffsetDateTime) -> Result<(), AppError>`: aturan K11, lalu `restore_file`.
  - `Ledger::integrity_check(&self) -> Result<Vec<String>, AppError>`
  - `nabungo_app` me-re-export `time::OffsetDateTime` (seperti `Date`) agar `src-tauri` tidak butuh dependensi `time`.
  - `AppError::InvalidBackup`, `AppError::BackupTooNew`, `AppError::BackupTargetInvalid`. `StorageError::InvalidBackup` / `BackupTooNew(_)` dipetakan ke dua varian pertama (pola `duplicate_as`).

- [ ] **Step 1: Write the failing tests** (file DB di `tempfile::tempdir()`; seed lewat `complete_onboarding`, `create_account`, `record` seperti `testkit`; "saldo" = `accounts_overview()` per akun + net worth)

```rust
#[test]
fn restore_gives_identical_balances() {
    // Acceptance PRD "Backup bisa dipulihkan": seed 2 akun, expense, transfer+biaya, valuation;
    // snapshot = saldo semua akun + net worth; backup_to(x.db);
    // instalasi baru: Ledger::open(dir2/nabungo.db) + restore_file(x.db) → snapshot identik
}
#[test]
fn restore_overwrites_newer_changes_and_keeps_pre_restore() {
    // backup; record expense baru; restore_file → expense hilang;
    // ada tepat satu file "pre-restore-*.db", dan restore_named(itu) mengembalikan expense
}
#[test]
fn invalid_restore_changes_nothing() // restore_file(teks) → InvalidBackup; saldo sama; folder backups kosong
#[test]
fn restore_old_schema_backup_migrates() {
    // buat file via storage open_file + migrate_to(4) + seed SQL minimal (1 akun + txn opening)
    // restore_file → schema_version == latest; saldo akun sama dengan seed
}
#[test]
fn restore_named_rejects_foreign_names() {
    // "../nabungo.db", path absolut, "catatan.txt", "auto-x.db" → InvalidBackup, tanpa file pre-restore
}
#[test]
fn backup_to_live_db_path_is_rejected() {
    // backup_to(path DB), backup_to(path DB + "-wal"), backup_to(symlink ke DB; cfg(unix)) → BackupTargetInvalid; DB tetap terbuka & saldo sama
}
#[test]
fn pre_migration_backup_only_for_existing_old_db() {
    // DB baru → open_with_backups tanpa file backup;
    // DB di versi 4 → satu "pre-migration-*.db" berversi 4, DB utama di versi latest
}
#[test]
fn auto_backup_runs_once_per_window() // now → Some; now+1 jam → None; now+20 jam → Some
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app backup::` → FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces. Perbandingan K10 memakai `std::fs::canonicalize` pada parent dir + nama file (tujuan belum tentu ada), lalu cek juga sufiks `-wal`/`-shm`.
- [ ] **Step 4: Run** `cargo test -p nabungo-app` → PASS.
- [ ] **Step 5: Commit** `feat(app): backup manual/otomatis, pra-migrasi, dan restore`

### Task 4: Konsistensi saat force-kill (exit criterion)

**Files:**
- Create: `crates/app/tests/crash_consistency.rs`

**Interfaces:**
- Consumes: API publik `Ledger` (`open`, `complete_onboarding`, `create_account`, `record`, `balance`, `integrity_check`).

- [ ] **Step 1: Write the test**

```rust
// Child: dipanggil ulang lewat current_exe() dengan env NABUNGO_CRASH_DB=<path>
// dan argumen `--exact crash_child --include-ignored --nocapture`.
// Child membuka DB dan melakukan loop tanpa henti: transfer 1 rupiah A → B.
#[test]
#[ignore = "dijalankan sebagai proses anak oleh kill_during_writes_keeps_balances"]
fn crash_child() { /* return bila env tidak ada */ }

#[test]
fn kill_during_writes_keeps_balances() {
    // Parent: seed A = 1_000_000, B = 0, lalu spawn child.
    // Tunggu sampai saldo B > 0 terlihat lewat koneksi baca (maks 10 s), lalu child.kill().
    // Ulangi 5 kali pada DB yang sama. Setelah setiap kill:
    //   integrity_check() kosong; balance(A) + balance(B) == 1_000_000; balance(B) naik dari putaran sebelumnya.
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app --test crash_consistency` → PASS. Bila gagal, itu bug nyata di jalur tulis: debug dengan `superpowers:systematic-debugging`, jangan melemahkan assert.
- [ ] **Step 3: Commit** `test(app): force-kill saat menulis tidak merusak saldo`

---

## Bagian B — Integrasi Tauri

### Task 5: Lifecycle — penanda sesi, laporan startup, backup terjadwal

**Files:**
- Create: `src-tauri/src/lifecycle.rs`
- Modify: `src-tauri/src/paths.rs` (`DataDir::backups_path()` → `path/backups`, `DataDir::session_marker_path()` → `path/session.lock`)
- Modify: `src-tauri/src/state.rs` (`AppState.startup: StartupReport`)
- Modify: `src-tauri/src/lib.rs` (`init` + `.build(..).run(|app, ev| …)` untuk `RunEvent::Exit`)
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/dto.rs`

**Interfaces:**
- Consumes: `Ledger::open_with_backups`, `integrity_check`, `auto_backup_if_due` (Task 3).
- Produces:
  - `pub struct SessionMarker` dengan `acquire(path: &Path) -> io::Result<bool>` (true = penanda sudah ada, yaitu sesi sebelumnya tidak normal; penanda ditulis/ditimpa) dan `release(path: &Path)`.
  - `pub enum IntegrityStatus { Skipped, Ok, Failed }`; `pub struct StartupReport { pub unclean_shutdown: bool, pub integrity: IntegrityStatus }`.
  - `pub fn auto_backup_allowed(r: &StartupReport) -> bool`: false bila `Failed`.
  - `pub fn spawn_backup_scheduler(app: AppHandle)`: thread `std::thread` yang tidur 1 jam lalu memanggil `auto_backup_if_due` (mutex poisoned → dilewati, error → `tracing::warn`).
  - Urutan `init`: resolve data dir → logging → `SessionMarker::acquire` → `open_with_backups` → integrity bila unclean → `auto_backup_if_due` bila diizinkan → manage state → scheduler bila diizinkan.
  - Command `startup_report() -> StartupReportDto { uncleanShutdown: bool, integrity: "skipped" | "ok" | "failed" }`.

- [ ] **Step 1: Write the failing tests** (`lifecycle.rs`)

```rust
#[test]
fn marker_detects_unclean_shutdown() // acquire → false; acquire lagi tanpa release → true; release lalu acquire → false
#[test]
fn auto_backup_not_allowed_after_failed_integrity() // Failed → false; Ok / Skipped → true
```

- [ ] **Step 2: Run** `cargo test -p nabungo-desktop lifecycle::` → FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces. Daftarkan `startup_report` lalu regenerate bindings.
- [ ] **Step 4: Run** semua perintah Global Constraints → PASS.
- [ ] **Step 5: Commit** `feat(desktop): integrity check setelah crash dan backup harian terjadwal`

### Task 6: App — ekspor JSON & CSV (DAT-02)

**Files:**
- Create: `crates/storage/src/export.rs` (`pub fn for_each_txn(conn: &Connection, f: impl FnMut(ExportTxn)) -> Result<()>`: transaksi aktif urut `date, created_at`, dengan postings dan tag)
- Modify: `crates/storage/src/budget.rs` (`pub fn list_all_budgets(conn: &Connection) -> Result<Vec<BudgetRow>>`, urut `period, category`)
- Create: `crates/app/src/export.rs`
- Modify: `crates/app/Cargo.toml` (`csv = "1.4.0"`, `serde` derive, `serde_json`)

**Interfaces:**
- Produces:
  - `Ledger::export_json(&self, w: &mut impl std::io::Write, now: OffsetDateTime) -> Result<(), AppError>`: skema K12. `app_version` = `crate::version()`.
  - `Ledger::export_csv(&self, parent: &Path, today: Date) -> Result<PathBuf, AppError>`: aturan K13–K15; mengembalikan folder yang dibuat.
  - Kolom CSV (header persis):
    - `transactions.csv`: `txn_id,date,kind,status,payee,note,tags,account,category,amount,currency,memo` (`tags` dipisah `;`).
    - `accounts.csv`: `id,name,kind,group,currency,archived,include_in_net_worth,balance`.
    - `categories.csv`: `id,name,kind,parent,archived`.
    - `budgets.csv`: `period,category,amount`.
- Ekspor tidak menulis ke DB, jadi tidak membutuhkan transaction.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn json_export_has_all_collections() {
    // seed seperti restore_gives_identical_balances; parse hasil dengan serde_json::Value:
    // format == "nabungo-export", version == 1, jumlah transactions == transaksi aktif (yang soft-deleted tidak ikut),
    // transfer punya 2 posting dengan category_id null, balance akun == ledger.balance()
}
#[test]
fn csv_export_creates_new_folder_each_time() // panggilan kedua di hari sama → "nabungo-export-2026-10-20-2"; isi folder pertama tidak berubah
#[test]
fn csv_has_bom_headers_and_one_row_per_posting() // 3 byte pertama EF BB BF; header persis; split 2 kategori → 2 baris
#[test]
fn csv_escapes_and_neutralizes_formulas() {
    // payee "=HYPERLINK(\"x\")" → sel "'=HYPERLINK(""x"")"; note "a, \"b\"\nc" ter-quote utuh;
    // amount -15000 tetap "-15000"; dibaca ulang dengan csv::Reader menghasilkan jumlah record yang benar
}
#[test]
fn child_category_written_with_parent() // "Makan / Restoran"
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app export::` → FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces. Netralisasi K14 diterapkan pada setiap kolom teks sebelum `csv::Writer::write_record`.
- [ ] **Step 4: Run** `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(app): ekspor JSON dan CSV`

### Task 7: Command backup, restore, ekspor + dialog native

**Files:**
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-dialog = "2.8.3"`), `src-tauri/src/lib.rs` (`.plugin(tauri_plugin_dialog::init())`, daftar command)
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/dto.rs`, `src-tauri/src/error.rs`
- Regenerate: `ui/src/ipc/bindings.ts`

**Interfaces:**
- Consumes: Task 3 dan Task 6.
- Produces:
  - `list_backups() -> Vec<BackupDto { fileName, kind: "auto" | "pre_migration" | "pre_restore", createdAt: string /* ISO-8601 UTC */, sizeBytes: number }>`
  - `backup_now() -> Option<String>`: dialog simpan (filter `*.db`, nama default K3); `None` bila dibatalkan; path hasil bila sukses.
  - `restore_backup(fileName: String) -> ()`
  - `restore_backup_from_file() -> bool`: dialog buka (filter `*.db`); `false` bila dibatalkan.
  - `export_json() -> Option<String>`: dialog simpan, nama default `nabungo-export-YYYY-MM-DD.json`.
  - `export_csv() -> Option<String>`: dialog pilih folder; mengembalikan folder yang dibuat.
  - Command dengan dialog berbentuk `async fn`. Dialog dipanggil dengan `blocking_*` di dalam `tauri::async_runtime::spawn_blocking`, lalu `Mutex<Ledger>` dikunci setelah dialog selesai.
  - Pemetaan error:
    - `AppError::InvalidBackup` → `backup_invalid`;
    - `AppError::BackupTooNew` → `backup_too_new`;
    - `AppError::BackupTargetInvalid` → `backup_target_invalid`;
    - `AppError::Io` → `file_io` (dicatat `tracing::error`).

- [ ] **Step 1: Implement** command, DTO, dan pemetaan error (match tanpa wildcard, sesuai pola `error.rs`). Daftarkan semua command di `collect_commands!`.
- [ ] **Step 2: Run** `cargo test -p nabungo-desktop export_bindings` lalu semua perintah Global Constraints → PASS. Typecheck UI akan gagal sampai `errors.*` ditambah: lakukan itu di task ini (`id.ts`/`en.ts`, empat code dari tabel).
- [ ] **Step 3: Commit** `feat(desktop): command backup, restore, dan ekspor dengan dialog native`

---

## Bagian C — UI

### Task 8: Facade IPC + browser-ledger

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/ipc/browser-ledger.ts`
- Test: `ui/src/ipc/browser-ledger.test.ts`

**Interfaces:**
- Produces (diekspor dari `@/ipc`): `startupReport()`, `listBackups()`, `backupNow()`, `restoreBackup(fileName)`, `restoreBackupFromFile()`, `exportJson()`, `exportCsv()`, beserta tipe `BackupDto`, `StartupReportDto`.
- Perilaku browser-ledger:
  - `backupNow`: menyimpan snapshot `structuredClone(state)` sebagai backup `auto` dengan `createdAt` = waktu sekarang. Mengembalikan `'browser://backup'`.
  - `restoreBackup(name)`: snapshot state saat ini disimpan dulu sebagai `pre_restore`, lalu state diganti snapshot itu. Nama tak dikenal → `backup_invalid`.
  - `restoreBackupFromFile`: mengembalikan `false` (tidak ada file di browser).
  - `startupReport`: `{ uncleanShutdown: false, integrity: 'skipped' }`.
  - `exportJson` / `exportCsv`: mengembalikan path palsu (`'browser://export.json'`, `'browser://nabungo-export'`).

- [ ] **Step 1: Write the failing tests** di `browser-ledger.test.ts`:
  - `restore mengembalikan saldo dan membuat pre_restore`: backup → catat expense → restore → saldo kembali; `listBackups()` terbaru dulu dan berisi satu `pre_restore`.
  - `restore nama tak dikenal ditolak`: kode `backup_invalid`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts` → FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** typecheck + test UI → PASS.
- [ ] **Step 5: Commit** `feat(ui): facade IPC backup dan ekspor`

### Task 9: Pengaturan — Backup & pemulihan, Ekspor data

**Files:**
- Modify: `ui/src/views/SettingsView.vue`
- Create: `ui/src/components/settings/BackupSection.vue`, `ui/src/components/settings/ExportSection.vue`
- Modify: `ui/src/lib/date.ts` (`formatTimestamp(iso: string, locale: LocaleDto): string`, tanggal + jam lokal, mis. `20 Okt 2026 10.15`)
- Modify: `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts` (`settings.sections.*`, `settings.backup.*`, `settings.export.*`)
- Test: `ui/src/views/SettingsView.test.ts` (baru), `ui/src/lib/date.test.ts`

**Interfaces:**
- Consumes: Task 8.
- Layout:
  - `SettingsView` menjadi tiga `<section>` berjudul (`h2`, gaya judul bagian per §5 ui-guidelines), dipisah `Separator`: **Umum** (isi yang sudah ada), **Backup & pemulihan**, **Ekspor data**. Lebar tetap `max-w-md` untuk form; daftar backup boleh `max-w-2xl`.
  - Backup & pemulihan:
    - teks penjelas: "Backup otomatis dibuat setiap hari; 14 terakhir disimpan di folder data." / "A backup is made automatically every day; the latest 14 are kept in the data folder.";
    - tombol [Backup sekarang…] (primer) dan [Pulihkan dari file…] (outline);
    - daftar backup: kolom waktu (`formatTimestamp`), jenis (`Badge`: Otomatis / Sebelum migrasi / Sebelum pemulihan), ukuran (KB/MB, `num` rata kanan), dan tombol baris [Pulihkan];
    - daftar kosong → `EmptyState`.
  - Konfirmasi restore memakai `ConfirmDialog` dengan `destructive`:
    - judul "Pulihkan backup {waktu}?";
    - isi "Data saat ini akan diganti dengan isi backup. Salinan data saat ini disimpan dulu sebagai backup sebelum pemulihan.";
    - tombol "Pulihkan".
  - Setelah sukses: toast "Data dipulihkan", lalu `window.location.reload()`. Aksi reload dibungkus fungsi yang bisa di-stub di test (`reloadApp` di `ui/src/lib/reload.ts`).
  - Ekspor data: tombol [Ekspor JSON…] dan [Ekspor CSV…]. Sukses → toast "Diekspor ke {path}". Batal (`null`) → tanpa toast.
  - Semua aksi lewat `withErrorToast`. Tombol nonaktif selama aksi berjalan.

- [ ] **Step 1: Write the failing tests** (`SettingsView.test.ts`, browser-ledger asli, fake Date `new Date(2026, 9, 17)`):
  - `backup sekarang menambah baris di daftar`.
  - `pulihkan meminta konfirmasi lalu memuat ulang`: klik [Pulihkan] → dialog dengan judul berisi waktu → konfirmasi → `reloadApp` dipanggil sekali; batal → tidak dipanggil.
  - `ekspor JSON menampilkan toast path`.
  - `daftar kosong menampilkan empty state`.
  - `date.test.ts`: `formatTimestamp('2026-10-20T12:00:00Z', 'id')` mengandung `20 Okt 2026` dan `'en'` mengandung `Oct 20, 2026` (jam siang UTC agar aman di zona waktu mana pun).
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/views/SettingsView.test.ts` → FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** typecheck + test UI → PASS.
- [ ] **Step 5: Commit** `feat(ui): bagian backup, pemulihan, dan ekspor di Pengaturan`

### Task 10: Dialog integritas saat startup + penutup fase

**Files:**
- Create: `ui/src/components/IntegrityAlert.vue`
- Modify: `ui/src/App.vue` (mount `IntegrityAlert` di layout non-bare)
- Modify: `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`
- Test: `ui/src/components/IntegrityAlert.test.ts`
- Modify: `docs/plans/2026-10-12-v1-roadmap.md` (tandai 6a selesai, catat K9 "layar pemulihan saat DB gagal dibuka → Fase 8"), `CLAUDE.md` (folder `backups/`, `session.lock`, aturan "semua open DB lewat `open_file`")

**Interfaces:**
- Consumes: `startupReport()` (Task 8).
- Perilaku:
  - `IntegrityAlert` memanggil `startupReport()` sekali saat mount.
  - Bila `integrity === 'failed'`, tampil `AlertDialog` (bukan destruktif):
    - judul "Database mungkin rusak";
    - isi "Nabungo tidak ditutup dengan normal dan pemeriksaan database menemukan masalah. Pulihkan dari backup terakhir sebelum mencatat transaksi baru.";
    - tombol [Nanti] dan [Buka backup] (navigasi ke `settings`).
  - Selain `failed`, tidak ada yang tampil.

- [ ] **Step 1: Write the failing tests**: `startupReport` di-mock lewat `vi.spyOn` pada modul `@/ipc`:
  - `failed menampilkan dialog dan tombol membuka pengaturan`
  - `ok/skipped tidak menampilkan apa pun`
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** semua perintah Global Constraints dan `pnpm --filter @nabungo/ui e2e` → PASS.
- [ ] **Step 5: Verifikasi manual di Tauri** (`pnpm dev`), hasilnya dicatat di ledger progres:
  1. **Backup sekarang**: menyimpan file; file itu bisa dipulihkan lewat [Pulihkan dari file…].
  2. **Restore**: setelah restore, app reload dan saldo kembali.
  3. **Folder backup**: `~/.local/share/nabungo/backups/` berisi `auto-*.db` setelah start.
  4. **Ekspor CSV**: file terbuka benar di LibreOffice.
  5. **Integrity setelah crash**: `kill -9` pada proses → start ulang → log mencatat integrity check dan hasilnya `ok`, tanpa dialog.
  6. **Tujuan backup = DB aktif**: memilih `nabungo.db` sebagai tujuan backup menampilkan toast `backup_target_invalid`.
- [ ] **Step 6: Commit** `feat(ui): peringatan integritas database saat startup` dan `docs: Fase 6a selesai`
