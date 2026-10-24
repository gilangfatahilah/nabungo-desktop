# Fase 8b: Pemulihan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** DB yang gagal dibuka tidak lagi membuat app crash. App masuk ke layar pemulihan, tempat user bisa restore backup atau mulai dengan data kosong. Backup terenkripsi dari instalasi lain bisa di-restore dengan recovery key-nya.

**Architecture:** `init()` tidak lagi gagal fatal saat DB tidak bisa dibuka. App mencatat `RecoveryState` di `AppState` (ledger tetap `None`), dan UI diarahkan ke `/recovery`. Operasi pemulihan berupa fungsi bebas di modul baru `nabungo-app::recovery`, karena tidak ada `Ledger`. Urutannya:

1. Bangun DB baru di `nabungo.db.recover`.
2. Pindahkan file rusak ke samping (tidak pernah dihapus).
3. Rename `nabungo.db.recover` ke `nabungo.db`.
4. Jalankan `lifecycle::open_unlocked`.

Restore backup asing memakai parameter `src_key` terpisah dari key DB aktif sampai ke `nabungo_storage::restore_from`, yang memang sudah bisa lintas key (`convert_file`).

**Tech Stack:** Rust (rusqlite bundled-sqlcipher, Tauri 2 + tauri-specta), Vue 3 + Pinia + vue-router, vitest, Playwright.

**Spec:** `PRD.md`, yang memuat:
- Acceptance: "File backup bisa di-restore ke instalasi baru dan menghasilkan saldo identik".
- §8: "Restore teruji di CI".
- Risiko: "Lupa master password → Recovery key yang dicetak".

Item yang ditunda dari fase sebelumnya:
- `docs/plans/2026-10-20-fase-6a-backup-integritas-export.md` K9: DB yang gagal dibuka masih fatal.
- `docs/plans/2026-10-20-fase-6b-enkripsi-sqlcipher.md` K9: backup terenkripsi instalasi lain ditolak `invalid_backup`.

## Kondisi saat ini (hasil riset)

1. **Plaintext gagal dibuka itu fatal.** Di `src-tauri/src/lib.rs` `init()`, error dari `security::db_state` atau `lifecycle::open_unlocked(None)` dikembalikan sebagai `Err` dari setup, lalu app crash di `.expect`. Restore tidak terjangkau.
2. **File sampah terbaca "terenkripsi".** `is_encrypted_file` hanya memeriksa 16 byte header, jadi file sampah atau header rusak dianggap terenkripsi. Unlock lalu gagal dengan `database_unreadable` (password benar, DB tidak cocok), dan user terjebak di `/unlock`.
3. **Backup terenkripsi asing selalu gagal.** `Ledger::restore_db` memanggil `inspect_backup(src, self.key)` dan `restore_from(conn, src, self.key)`, dan `restore_from` memakai `src_key = live_key`. Backup terenkripsi dengan DEK lain selalu gagal dan dipetakan ke `InvalidBackup`.
4. **DB dari versi app yang lebih baru** (`user_version > latest_version()`) gagal di `migrate` sebagai error storage generik.

## Keputusan

| ID | Keputusan | Alasan |
|---|---|---|
| P1 | **Mode pemulihan.** `init()` menangkap error `db_state` dan `open_unlocked` (plaintext). Error itu di-log, lalu `AppState.recovery = Some(RecoveryState { reason })` diisi dan app tetap jalan. `reason`: `Unreadable` \| `TooNew`. Command yang butuh ledger mengembalikan code baru `recovery_required` (bukan `locked`) selama mode ini aktif. | App harus selalu bisa start agar restore terjangkau (6a K9). `locked` tidak dipakai karena memicu layar unlock yang salah. |
| P2 | `Ledger::open_with_backups` mengembalikan `AppError::DatabaseTooNew` bila `schema_version > latest_version()`, sebelum migrasi. Error ini dipetakan ke `reason = TooNew`. | Pesannya beda ("perbarui aplikasi"). Restore tetap ditawarkan, tetapi dengan peringatan bahwa data versi baru akan dipindahkan ke samping. |
| P3 | **File rusak tidak pernah dihapus.** `nabungo.db` (+ `-wal`/`-shm`/`-journal`) di-rename ke `nabungo.db.broken-<YYYYMMDDTHHMMSSZ>` (+ sufiks yang sama). Bila key file tidak lagi cocok dengan DB baru, `nabungo.key` di-rename ke `nabungo.key.broken-<ts>`. Key file tidak cocok bila sumber plaintext, sumber dibuka dengan recovery key, atau user memilih mulai kosong. | CLAUDE.md: jangan hapus `nabungo.key`. Tanpa rename, `db_state` akan menghapus key file "nyasar" di samping DB plaintext baru. Data rusak tetap ada untuk diperbaiki manual. |
| P4 | **Urutan crash-safe:** restore ke `nabungo.db.recover` sampai selesai (backup API + migrasi, koneksi ditutup) → pindahkan file rusak ke samping (P3) → rename `.recover` → `nabungo.db`. Ada dua kasus sisa crash yang ditangani `db_state`. Bila `nabungo.db` tidak ada tetapi `.recover` ada, rename diselesaikan. Bila keduanya ada, `.recover` dihapus (restore belum selesai, DB lama masih di tempat). | Crash di tengah jalan tidak boleh berakhir di DB kosong, karena app akan menampilkan onboarding seolah data hilang. Polanya sama dengan sisa `.convert*` di 6b. |
| P5 | **DB hasil pemulihan memakai status enkripsi backup-nya,** karena tidak ada key DB lama yang bisa dipakai. Kunci sumber berasal dari `BackupSecret`: (a) `None`: sumber plaintext. (b) `Password`: membuka `nabungo.key` yang ada. Ini kasus backup dari instalasi yang sama; key file dipertahankan. (c) `RecoveryKey { key, new_password }`: DEK dari recovery key, lalu key file baru ditulis dengan password baru (sama seperti `recover_key`). | Satu jalur yang mencakup semua kasus, termasuk "lupa password" dan backup asing di layar pemulihan. |
| P6 | **Restore normal lintas key.** `Ledger::restore_file`/`restore_named` menerima `src_recovery: Option<&DbKey>`, dan `src_key = src_recovery.or(self.key)`. DB aktif tetap pada status enkripsi dan key-nya sendiri (aturan 6b dipertahankan). Recovery key yang dimasukkan adalah milik **file backup**, bukan instalasi ini. | Item 6b K9. `convert_file` sudah mendukung `src_key ≠ dest_key`. |
| P7 | **Error baru untuk backup yang butuh key.** `inspect_backup` dengan header bukan "SQLite format 3" dan key yang gagal: tanpa `src_recovery` hasilnya `AppError::BackupKeyRequired` (code `backup_key_required`); dengan `src_recovery` hasilnya `InvalidRecoveryKey`. Header SQLite yang rusak tetap `backup_invalid`. | File sampah dan file terenkripsi asing tidak bisa dibedakan dari header. Copy UI menyebut keduanya: "File ini terenkripsi (mis. dari instalasi lain) atau bukan backup Nabungo. Masukkan recovery key file tersebut." |
| P8 | **Command baru dan yang diubah** (`src-tauri`): `recovery_status`, `inspect_backup_source`, `pick_backup_file`, `restore_backup(source, src_recovery_key)` (menggantikan `restore_backup` + `restore_backup_from_file`), `recover_from_backup(source, secret)`, `recovery_start_fresh`. Command pemulihan hanya jalan bila `ledger` `None` (mode pemulihan **atau** terkunci). Selain itu hasilnya `recovery_unavailable`. | Dari `/unlock` user bisa masuk pemulihan (kasus 2 riset, atau lupa password tanpa recovery key). DB terenkripsi asli tetap aman karena dipindah, bukan dihapus. |
| P9 | `BackupSourceDto` = `{ kind: 'named', fileName }` \| `{ kind: 'path', path }`. Path hanya berasal dari `pick_backup_file` (dialog native). Backend tetap memvalidasi `ensure_not_live` (`backup_target_invalid`) dan `inspect_backup`. Nama `named` tetap divalidasi `parse_backup_name`. | Restore asing butuh dua langkah (pilih → periksa butuh key → masukkan key → restore) tanpa membuka dialog dua kali. |
| P10 | **UI.** Route `/recovery` (`meta.bare`). `recoveryGuard` didaftarkan **sebelum** `securityGuard`. Saat pemulihan aktif semua route diarahkan ke `recovery`. Saat terkunci, `recovery` boleh diakses. Saat normal, `recovery` dialihkan ke dasbor. Code `recovery_required` dari command apa pun berpindah ke `/recovery` (`setRecoveryHandler`, pola sama dengan `setLockedHandler`). UnlockView menampilkan tautan "Pulihkan dari backup" yang selalu terlihat, dan menonjolkannya setelah error `database_unreadable`/`key_file_invalid`. | Konsisten dengan pola lock 6b. |
| P11 | **Mode browser** (`browser-ledger.ts`): `recoveryStatus` aktif hanya lewat hook test `__enterRecovery(reason)`. `recoverFromBackup`/`recoveryStartFresh` memakai snapshot in-memory lalu keluar dari mode pemulihan. `inspectBackupSource` → `needsKey: false`. Error mengikuti Rust: command ledger saat pemulihan → `recovery_required`; command pemulihan saat normal → `recovery_unavailable`; `named` yang tidak ada → `backup_invalid`. | Aturan turunan harus sama dengan Rust (CLAUDE.md), dan e2e butuh cara masuk ke layar ini. |

## Global Constraints

- Offline total: tidak ada network call, aset remote, atau telemetry.
- **Jangan pernah menghapus** `nabungo.db` lama atau `nabungo.key`. Keduanya hanya di-rename (P3).
- Semua open DB lewat `nabungo_storage` (`open_file`, `open_file_read_only`, `restore_from`); jangan `Connection::open` di luar `file.rs`. Restore memakai backup API / `convert_file`, bukan salin file.
- Migrasi rilis tidak diubah; fase ini tidak menambah migrasi.
- `ui/src/ipc/bindings.ts` di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit, tidak diedit manual.
- Code error baru (`recovery_required`, `recovery_unavailable`, `backup_key_required`, `database_too_new`) wajib ada di `errors` `ui/src/i18n/id.ts` dan `en.ts`.
- Command Argon2 (`recover_from_backup` dengan password atau recovery key) berjalan di `spawn_blocking` (helper `blocking`). Dialog lewat `pick_path`.
- UI mengikuti `docs/design/design.md` (layar bare seperti `UnlockView`; dialog destruktif memakai `ConfirmDialog destructive`).

## Review Focus

1. **Crash di setiap titik `restore_into_fresh`/`start_fresh`** (sebelum rename samping, di antara rename samping dan rename final): startup berikutnya tidak boleh berakhir di DB kosong/onboarding saat data masih ada di `.recover` atau `.broken-*`. Ini dikunci oleh test `db_state_finishes_interrupted_recovery` dan `db_state_discards_incomplete_recover` di Task 3.
2. **Restore backup plaintext ke DB yang dulunya terenkripsi:** `nabungo.key` harus berpindah ke `nabungo.key.broken-*`, bukan dihapus oleh pembersihan key file nyasar di `db_state`. Dikunci oleh `plaintext_backup_moves_keyfile_aside`.
3. **Recovery key salah, password salah, atau file sampah di layar pemulihan:** tidak boleh ada file yang dipindah atau dibuat (`.recover` dibersihkan), dan error-nya spesifik. Dikunci oleh `failed_recovery_leaves_files_untouched`.
4. **Sumber restore = DB aktif atau file `.broken-*` miliknya sendiri lewat dialog:** DB aktif ditolak `backup_target_invalid`, sedangkan `.broken-*` lama boleh dicoba dan gagal secara wajar (`backup_invalid` atau `backup_key_required`), tanpa panic. Dikunci oleh `recovery_rejects_live_db_as_source`.
5. **Command biasa yang dipanggil UI saat mode pemulihan** (sidebar, palette, auto-lock idle) tidak boleh memicu loop navigasi `/recovery` ↔ `/unlock`. Dikunci oleh test router `recovery_wins_over_locked` di Task 5.

---

### Task 1: Storage — restore lintas key dan error backup terenkripsi (P6, P7)

**Files:**
- Modify: `crates/storage/src/file.rs` (`inspect_backup`, `restore_from`)
- Modify: `crates/storage/src/lib.rs` (`StorageError`)
- Modify: pemanggil `restore_from` di `crates/app/src/backup.rs` (sementara dengan `src_key = self.key`)

**Interfaces:**
- Produces: `pub fn restore_from(conn: &mut Connection, src: &Path, src_key: Option<&DbKey>, live_key: Option<&DbKey>) -> Result<()>`.
- Produces: `inspect_backup(path, key) -> Result<usize>`, yang sekarang mengembalikan `StorageError::BackupKeyRequired` bila header file bukan SQLite plaintext dan `open_raw` gagal dengan `KeyRequired`/`WrongKey`. File ber-header SQLite yang tidak bisa dibaca tetap `InvalidBackup`.
- Produces: varian baru `StorageError::BackupKeyRequired` (`#[error("backup is encrypted with another key")]`).

- [ ] **Step 1: Test** di `file.rs`:
  - `restore_from_foreign_encrypted_into_plain`: buat DB terenkripsi key A berisi satu tabel/baris (pakai `open` + `migrate` + insert akun lewat SQL), lalu `restore_from(&mut plain_conn, src, Some(&a), None)`. Baris ada dan file tujuan tetap plaintext (`!is_encrypted_file`).
  - `restore_from_foreign_encrypted_into_other_key`: sumber key A, DB aktif key B. Setelah restore, DB aktif bisa dibuka dengan B dan berisi baris tersebut.
  - `inspect_backup_encrypted_without_key_requires_key`: sumber key A, `inspect_backup(src, None)` → `BackupKeyRequired`; `inspect_backup(src, Some(&b))` → `BackupKeyRequired`.
  - `inspect_backup_garbage_header_requires_key`: file 4096 byte acak → `BackupKeyRequired` (P7).
  - `inspect_backup_corrupt_sqlite_is_invalid`: header "SQLite format 3\0" + sisa sampah → `InvalidBackup`.
- [ ] **Step 2: Jalankan** `cargo test -p nabungo-storage file::` → gagal kompilasi (signature dan varian belum ada).
- [ ] **Step 3: Implementasi.** `restore_from` memakai `src_key` untuk sumber terenkripsi. Untuk sumber plaintext `src_key` diabaikan. Cabang status sama vs beda tetap ditentukan header sumber vs `live_key`. Di `inspect_backup`, petakan `KeyRequired | WrongKey` ke `BackupKeyRequired` hanya bila `is_encrypted_file(path)`. Perbarui pemanggil di `backup.rs` menjadi `restore_from(&mut self.conn, src, self.key.as_ref(), self.key.as_ref())`.
- [ ] **Step 4: Jalankan** `cargo test -p nabungo-storage && cargo test -p nabungo-app` → semua lulus (test restore lama tidak berubah perilaku).
- [ ] **Step 5: Commit** `feat(storage): restore lintas key dan error backup terenkripsi`.

### Task 2: App — restore backup instalasi lain dengan recovery key (P6, P7, P2)

**Files:**
- Modify: `crates/app/src/backup.rs` (`restore_file`, `restore_named`, `restore_db`, `open_with_backups`), `crates/app/src/ledger.rs` (`AppError` + `from_backup`)
- Test: `crates/app/src/backup.rs` (mod tests)

**Interfaces:**
- Consumes: `restore_from(conn, src, src_key, live_key)` dan `StorageError::BackupKeyRequired` (Task 1).
- Produces:
  - `pub fn restore_file(&mut self, src: &Path, src_recovery: Option<&DbKey>, backup_dir: &Path, attachments_dir: &Path, now: OffsetDateTime) -> Result<(), AppError>`
  - `pub fn restore_named(&mut self, backup_dir: &Path, file_name: &str, src_recovery: Option<&DbKey>, now: OffsetDateTime) -> Result<(), AppError>`
- Produces:
  - `pub fn backup_needs_key(src: &Path, live_key: Option<&DbKey>) -> Result<bool, AppError>`: fungsi bebas, `true` bila `inspect_backup(src, live_key)` → `BackupKeyRequired`; error lain diteruskan.
  - `impl Ledger { pub fn key(&self) -> Option<&DbKey> }` bila belum ada.
- Produces: `AppError::BackupKeyRequired` dan `AppError::DatabaseTooNew`. `from_backup` memetakan `BackupKeyRequired` ke `InvalidRecoveryKey` bila `src_recovery.is_some()`. Mapping ini ada di `restore_db`, bukan di `from_backup`.

- [ ] **Step 1: Test:**
  - `restore_foreign_encrypted_backup_matches_balances` (acceptance PRD): instalasi A terenkripsi (`fixture::seed` kecil, `FixtureSpec` dengan sedikit transaksi bila ada konstruktor kecil, bila tidak pakai helper ledger test yang ada) → `backup_to`. Instalasi B plaintext baru → `restore_file(src, Some(&key_a), …)`. Hasil `accounts_overview()` B == A (saldo per akun dan net worth), dan `B.is_encrypted() == false`.
  - `restore_foreign_backup_into_encrypted_keeps_live_key`: B terenkripsi key B. Setelah restore, buka ulang B dengan key B berhasil dan saldonya identik.
  - `restore_foreign_backup_without_key_is_key_required`: `restore_file(src, None, …)` → `Err(BackupKeyRequired)`; data B tidak berubah dan tidak ada file pre-restore baru.
  - `restore_foreign_backup_wrong_key_is_invalid_recovery_key`: key acak → `Err(InvalidRecoveryKey)`.
  - `backup_needs_key_reports_foreign`: `backup_needs_key(src_a, Some(&key_b)) == true`, `backup_needs_key(src_a, Some(&key_a)) == false`, `backup_needs_key(plain, None) == false`.
  - `open_newer_schema_is_too_new`: DB plaintext dengan `PRAGMA user_version = latest_version() + 1` → `open_with_backups` → `Err(DatabaseTooNew)`, tanpa file pre-migration.
- [ ] **Step 2: Jalankan** `cargo test -p nabungo-app backup::` → gagal kompilasi.
- [ ] **Step 3: Implementasi.** Signature di atas. Di `restore_db`, `let src_key = src_recovery.or(self.key.as_ref())`, dipakai untuk `inspect_backup` dan `restore_from`. Di `open_with_backups`, periksa versi > latest sebelum `migrate`. Perbarui pemanggil di `src-tauri/src/commands.rs` dengan `None` agar workspace tetap kompilasi (command baru di Task 4).
- [ ] **Step 4: Jalankan** `cargo test --workspace` → lulus; `cargo clippy --workspace --all-targets -- -D warnings` → bersih.
- [ ] **Step 5: Commit** `feat(app): restore backup instalasi lain dengan recovery key`.

### Task 3: App — modul pemulihan tanpa Ledger (P3, P4, P5)

**Files:**
- Create: `crates/app/src/recovery.rs` (daftarkan di `crates/app/src/lib.rs`)
- Modify: `crates/app/src/security.rs` (`db_state`: sisa `.recover`; ekstrak helper penulisan key file dari `recover_key`)
- Modify: `crates/app/src/backup.rs` (`restore_sidecar` jadi `pub(crate)`)

**Interfaces:**
- Consumes: `restore_from` (Task 1), `backup_needs_key`, `AppError::BackupKeyRequired` (Task 2), `vault::{parse_recovery_key, KeyFile, keyfile_path}`, `security::unlock_key`.
- Produces:

```rust
pub enum BackupSecret {
    None,
    Password(String),
    RecoveryKey { key: String, new_password: String },
}

pub struct Recovered {
    pub key: Option<DbKey>,
    /// File lama yang dipindah ke samping (P3), untuk ditampilkan dan di-log.
    pub moved_aside: Vec<PathBuf>,
}

pub fn restore_into_fresh(db: &Path, src: &Path, attachments_dir: &Path,
    secret: &BackupSecret, params: KdfParams, now: OffsetDateTime) -> Result<Recovered, AppError>;
pub fn start_fresh(db: &Path, now: OffsetDateTime) -> Result<Vec<PathBuf>, AppError>;
pub fn broken_suffix(now: OffsetDateTime) -> String; // ".broken-20261024T101500Z"
```

Aturan `restore_into_fresh`:
- Validasi dulu tanpa menyentuh file:
  - `src` tidak boleh `db` atau sibling `-wal`/`-shm`/`-journal`-nya (`BackupTargetInvalid`).
  - Kunci sumber diturunkan dari `secret` (P5). `Password` tanpa key file → `KeyFileInvalid`; password salah → `WrongPassword`; recovery key tidak bisa di-parse atau tidak cocok → `InvalidRecoveryKey`. `new_password` < minimum → `PasswordTooShort`.
  - Sumber terenkripsi + `BackupSecret::None` → `BackupKeyRequired`; sumber plaintext + secret apa pun → secret diabaikan.
  - Lalu `inspect_backup`.
- Kemudian urutan P4. Key file dipindah kecuali `secret` = `Password` dan sumber terenkripsi. `RecoveryKey` menulis key file baru (`write_atomic`) setelah rename final. Lampiran: `restore_sidecar(src, attachments_dir)`.
- `start_fresh` memindahkan DB dan key file ke samping. DB baru dibuat oleh `open_unlocked` yang berjalan setelahnya (plaintext, onboarding).

- [ ] **Step 1: Test** di `recovery.rs` (tempdir + `KdfParams` test cepat yang sudah dipakai `security.rs`):
  - `restore_into_fresh_replaces_garbage_db`: `nabungo.db` = 4096 byte acak; sumber backup plaintext dari ledger berisi data. Hasilnya `Recovered.key == None`, `Ledger::open(db)` berisi saldo identik, dan `moved_aside` berisi `nabungo.db.broken-…` yang isinya byte-identik dengan sampah awal.
  - `plaintext_backup_moves_keyfile_aside`: ada `nabungo.key` + DB sampah. Setelah restore plaintext, `nabungo.key` tidak ada, `nabungo.key.broken-…` ada dengan isi identik, lalu `security::db_state(db) == Plain` (tidak ada file yang dihapus).
  - `password_restores_same_installation_backup`: instalasi terenkripsi (key file + password) dengan backup. DB lalu ditimpa sampah. `restore_into_fresh(…, Password(pw))` → `Recovered.key == Some(dek)`, key file asli tetap di tempat dan `unlock_key(db, pw)` membuka DB baru.
  - `recovery_key_restores_foreign_backup_and_writes_keyfile`: backup terenkripsi instalasi lain + `RecoveryKey { key, new_password }` → `unlock_key(db, new_password)` berhasil, dan key file lama (bila ada) ada di `nabungo.key.broken-…`.
  - `failed_recovery_leaves_files_untouched`: kasus password salah, recovery key salah, sumber sampah, dan sumber terenkripsi + `None`. Masing-masing error spesifik (`WrongPassword`, `InvalidRecoveryKey`, `BackupKeyRequired`, `BackupKeyRequired`), daftar file di folder identik sebelum/sesudah, dan tidak ada `.recover`.
  - `recovery_rejects_live_db_as_source` → `BackupTargetInvalid`.
  - `start_fresh_moves_db_and_keyfile_aside`: keduanya pindah, `db` tidak ada lagi.
  - Di `security.rs`:
    - `db_state_finishes_interrupted_recovery`: hanya `nabungo.db.recover` (DB plaintext valid). `db_state` → `Plain`, `nabungo.db` ada, `.recover` hilang.
    - `db_state_discards_incomplete_recover`: `nabungo.db` dan `.recover` sama-sama ada. `.recover` dihapus, `nabungo.db` tidak berubah.
- [ ] **Step 2: Jalankan** `cargo test -p nabungo-app recovery:: security::db_state` → gagal kompilasi.
- [ ] **Step 3: Implementasi** sesuai Interfaces. Gunakan `nabungo_storage::open_file(recover_path, key)` → `restore_from(&mut conn, src, key, key)` (status sama → backup API) → `migrate` → tutup koneksi. Lalu rename samping (DB + sibling yang ada), lalu rename `.recover` → `db`. Helper key file yang diekstrak dari `recover_key` dipakai bersama, dan perilaku `recover_key` tidak berubah.
- [ ] **Step 4: Jalankan** `cargo test --workspace` → lulus; clippy bersih.
- [ ] **Step 5: Commit** `feat(app): pemulihan DB tanpa Ledger`.

### Task 4: Desktop — mode pemulihan dan command (P1, P8, P9)

**Files:**
- Modify: `src-tauri/src/state.rs` (`recovery: Mutex<Option<RecoveryState>>`)
- Modify: `src-tauri/src/lib.rs` (`init`, `collect_commands!`)
- Modify: `src-tauri/src/lifecycle.rs` (`startup_mode`, keluar dari pemulihan)
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/error.rs`
- Modify: `ui/src/ipc/bindings.ts` (generate), `ui/src/i18n/id.ts` + `en.ts` (kunci `errors.*`)

**Interfaces:**
- Consumes: Task 2 (`restore_file`/`restore_named` dengan `src_recovery`, `backup_needs_key`, `DatabaseTooNew`) dan Task 3 (`recovery::*`).
- Produces (Rust):
  - `pub enum RecoveryReason { Unreadable, TooNew }`, `pub struct RecoveryState { pub reason: RecoveryReason }`.
  - `fn startup_mode(r: Result<(), AppError>) -> Option<RecoveryState>` di `lifecycle.rs`: murni dan bisa di-test. `DatabaseTooNew` → `TooNew`, error lain → `Unreadable`, `Ok` → `None`.
  - `ledger(&state)` mengembalikan `recovery_required` bila `state.recovery` `Some`.
- Produces (command / TS via bindings):
  - `recovery_status() -> RecoveryStatusDto { active: bool, reason: Option<RecoveryReasonDto>, dataDir: String, hasKeyfile: bool }` (`hasKeyfile` = `nabungo.key` ada; tidak lewat `db_state`, yang bisa gagal di mode ini)
  - `pick_backup_file() -> Option<String>` (async, dialog `Open`)
  - `inspect_backup_source(source: BackupSourceDto) -> BackupSourceInfoDto { needsKey: bool }`. Key aktif = `ledger.key()`. Saat ledger `None`, `needsKey` = sumber terenkripsi dan tidak ada key aktif.
  - `restore_backup(source: BackupSourceDto, srcRecoveryKey: Option<String>) -> ()` menggantikan `restore_backup(fileName)` + `restore_backup_from_file`. Recovery key di-parse dengan `vault::parse_recovery_key`; gagal parse → `invalid_recovery_key`.
  - `recover_from_backup(source: BackupSourceDto, secret: BackupSecretDto) -> ()` (async, `blocking`). `BackupSecretDto` = `{ kind: 'none' } | { kind: 'password', password } | { kind: 'recoveryKey', recoveryKey, newPassword }`. Sesudah sukses: `recovery = None`, `open_unlocked(app, key)`, `startup.after_restore()`.
  - `recovery_start_fresh() -> ()`: `recovery::start_fresh` → `recovery = None` → `open_unlocked(app, None)`.
- Produces: `ErrorCode::{RecoveryRequired, RecoveryUnavailable, BackupKeyRequired, DatabaseTooNew}`.

- [ ] **Step 1: Test** (unit di `lifecycle.rs` / `commands.rs`, pola test yang sudah ada):
  - `startup_mode_maps_errors`: `Ok` → `None`; `DatabaseTooNew` → `TooNew`; `DatabaseUnreadable` dan error storage → `Unreadable`.
  - `backup_source_named_rejects_paths`: `BackupSourceDto::Named { file_name: "../x.db" }` → `backup_invalid`. Ini fungsi resolver `fn resolve_source(dir: &Path, s: BackupSourceDto) -> Result<PathBuf, IpcError>`.
  - `error_codes_serialize_snake_case`: memperluas test serialisasi yang ada dengan 4 code baru.
- [ ] **Step 2: Jalankan** `cargo test -p nabungo-desktop` → gagal kompilasi.
- [ ] **Step 3: Implementasi.**
  - `init()`: `db_state` Err → `recovery = Some(Unreadable)`. `DbState::Plain` + `open_unlocked` Err → `startup_mode(Err)`. Error tetap di-log (`tracing::error!`), tanpa `eprintln` + `return Err`.
  - Command pemulihan memeriksa `ledger` `None`, selain itu `recovery_unavailable`.
  - `recovery_start_fresh` dan `recover_from_backup` memakai `confirm` di UI, bukan di backend.
  - Scheduler backup tidak jalan selama pemulihan, karena `spawn_backup_scheduler` hanya dipanggil dari `open_unlocked`.
  - Generate binding: `cargo test -p nabungo-desktop export_bindings`.
  - Tambah 4 kunci `errors.*` di `id.ts` dan `en.ts`. Copy `id`:
    - `recovery_required`: "Database tidak bisa dibuka. Pulihkan dari backup terlebih dahulu."
    - `recovery_unavailable`: "Pemulihan hanya tersedia saat database tidak terbuka."
    - `backup_key_required`: "File ini terenkripsi (mis. dari instalasi lain) atau bukan backup Nabungo. Masukkan recovery key file tersebut."
    - `database_too_new`: "Database dibuat oleh versi Nabungo yang lebih baru. Perbarui aplikasi."
  - Perbarui pemakaian `restoreBackup`/`restoreBackupFromFile` di `BackupSection.vue` secukupnya agar typecheck lulus. UX penuhnya di Task 5.
- [ ] **Step 4: Jalankan:**
  - `cargo test --workspace` → lulus;
  - `cargo clippy --workspace --all-targets -- -D warnings` → bersih;
  - `pnpm --filter @nabungo/ui typecheck` → lulus;
  - `git diff --stat ui/src/ipc/bindings.ts` → berisi command baru.
- [ ] **Step 5: Commit** `feat(desktop): mode pemulihan saat DB gagal dibuka`.

### Task 5: UI — ipc, mirror browser, router, dan restore dengan recovery key (P9, P10, P11)

**Files:**
- Modify: `ui/src/ipc/index.ts` (`setRecoveryHandler`; re-export command baru)
- Modify: `ui/src/ipc/browser-ledger.ts` (+ test)
- Modify: `ui/src/router.ts` (+ `router.test.ts`)
- Modify: `ui/src/components/settings/BackupSection.vue` (+ test di `SettingsView.test.ts` atau file test baru `BackupSection.test.ts`)
- Modify: `ui/src/i18n/id.ts`, `en.ts` (`settings.backup.*` baru)

**Interfaces:**
- Consumes: binding Task 4 (`recoveryStatus`, `pickBackupFile`, `inspectBackupSource`, `restoreBackup(source, srcRecoveryKey)`, `recoverFromBackup`, `recoveryStartFresh`).
- Produces:
  - `export function setRecoveryHandler(fn: (() => void) | null): void` (dipanggil saat code `recovery_required`).
  - `export function recoveryGuard(isRecovery: () => Promise<boolean>): NavigationGuard`.
  - Route `{ path: '/recovery', name: 'recovery', meta: { bare: true } }`.
  - `securityGuard` dan `onboardingGuard` melewatkan `recovery`.
  - Hook test browser: `__enterRecovery(reason: 'unreadable' | 'too_new')`.

- [ ] **Step 1: Test:**
  - `router.test.ts`:
    - `recovery_redirects_all_routes`: aktif → `/transactions` menjadi `recovery`.
    - `recovery_route_leaves_when_inactive`: tidak aktif + tidak terkunci → `recovery` menjadi `dashboard`.
    - `recovery_allowed_while_locked`: terkunci → `recovery` boleh diakses (securityGuard melewatkannya).
    - `recovery_wins_over_locked`: pemulihan aktif + terkunci → `unlock` menjadi `recovery`.
  - `browser-ledger.test.ts`:
    - normal → `recoveryStatus().active === false`, dan `recoverFromBackup` → `recovery_unavailable`;
    - `__enterRecovery('unreadable')` → `listAccounts` → `recovery_required`;
    - `recoverFromBackup({kind:'named', fileName}, {kind:'none'})` → aktif `false` dan data snapshot kembali;
    - `recoveryStartFresh` → onboarding belum selesai.
  - `index.test.ts`: code `recovery_required` memanggil handler yang di-set.
  - BackupSection:
    - restore `named` dengan `inspectBackupSource` → `needsKey: true` menampilkan field recovery key di dialog konfirmasi;
    - tombol konfirmasi mengirim `restoreBackup(source, key)`;
    - error `invalid_recovery_key` menempel di field tersebut.
- [ ] **Step 2: Jalankan** `pnpm --filter @nabungo/ui exec vitest run src/router.test.ts src/ipc` → gagal.
- [ ] **Step 3: Implementasi.**
  - Daftarkan `recoveryGuard` sebelum `securityGuard`.
  - `main.ts`/`App.vue` memasang `setRecoveryHandler(() => router.replace({ name: 'recovery' }))`, tanpa reload. Ini beda dengan lock: di mode pemulihan tidak ada data store yang perlu dibuang.
  - Alur BackupSection "Pulihkan dari file": `pickBackupFile` → batal = selesai → `inspectBackupSource` → dialog konfirmasi (dengan field recovery key bila `needsKey`) → `restoreBackup`.
  - Mirror browser mengikuti P11. Simulasi restore lintas key tidak ada.
- [ ] **Step 4: Jalankan** `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck` → lulus.
- [ ] **Step 5: Commit** `feat(ui): restore backup dengan recovery key dan guard pemulihan`.

### Task 6: UI — layar pemulihan dan tautan dari layar kunci (P10)

**Files:**
- Create: `ui/src/views/RecoveryView.vue`, `ui/src/views/RecoveryView.test.ts`
- Modify: `ui/src/views/UnlockView.vue` (+ `UnlockView.test.ts`), `ui/src/i18n/id.ts` + `en.ts` (`recovery.*`)
- Modify: `ui/e2e/` (satu spec baru `recovery.spec.ts`)

**Interfaces:**
- Consumes: Task 5 (route, ipc, hook `__enterRecovery`).

**Isi layar** (bare, pola `UnlockView`: logo, judul, deskripsi per `reason`):
- Pesan alasan dan path folder data (`dataDir`), dengan catatan bahwa file lama disimpan sebagai `nabungo.db.broken-…` dan tidak dihapus.
- Daftar backup dari `listBackups()` (waktu + jenis, terbaru dulu) dengan tombol "Pulihkan" per baris, plus tombol "Pilih file backup…".
- Setelah memilih sumber, `inspectBackupSource`: bila terenkripsi muncul pilihan "Password" (bila `recoveryStatus().hasKeyfile`) atau "Recovery key + password baru".
- Konfirmasi destruktif lalu `recoverFromBackup` → `router.replace('/')` → `reloadApp()` di Tauri.
- "Mulai dengan data kosong" di bagian bawah, konfirmasi destruktif → `recoveryStartFresh` → onboarding.
- `reason === 'too_new'`: peringatan tambahan di atas daftar ("Perbarui aplikasi bila memungkinkan; memulihkan akan memindahkan database versi baru ke samping").

- [ ] **Step 1: Test** `RecoveryView.test.ts` (browser-ledger + `__enterRecovery`):
  - `shows_reason_and_backups`: judul sesuai reason dan baris backup tampil.
  - `restore_named_backup_leaves_recovery`: klik Pulihkan → konfirmasi → `recoveryStatus().active === false` dan router di `/`.
  - `start_fresh_goes_to_onboarding`.
  - `too_new_shows_update_warning`.
  - Di `UnlockView.test.ts`: `shows_recovery_link_after_unreadable`. Ini memakai mock `unlock` yang melempar `database_unreadable` sesuai pola test yang ada. Tautan `/recovery` ditonjolkan, dan tautan biasa selalu ada.
- [ ] **Step 2: Jalankan** `pnpm --filter @nabungo/ui exec vitest run src/views/RecoveryView.test.ts src/views/UnlockView.test.ts` → gagal.
- [ ] **Step 3: Implementasi** sesuai isi layar. Komponen shadcn yang sudah ada (`Button`, `Input`, `Label`, `ConfirmDialog`). Error menempel di field (design.md §7.3), sama dengan UnlockView.
- [ ] **Step 4: Test e2e** `recovery.spec.ts`: buka app → buat data via onboarding → `backupNow`/snapshot browser → `__enterRecovery('unreadable')` lewat `page.evaluate` → navigasi ke halaman mana pun menjadi layar pemulihan → pulihkan → dasbor tampil dengan saldo yang sama. Jalankan `pnpm --filter @nabungo/ui e2e` → lulus.
- [ ] **Step 5: Jalankan** `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck` → lulus.
- [ ] **Step 6: Commit** `feat(ui): layar pemulihan`.

### Task 7: Dokumentasi dan QA manual

**Files:** `CLAUDE.md` (subbagian "Backup & file DB": mode pemulihan, `.recover`, `.broken-*`), `docs/plans/2026-10-12-v1-roadmap.md` (baris 8, 8b selesai), plan ini (Hasil).

- [ ] **Step 1: Tulis catatan di CLAUDE.md**, maksimal 6 baris:
  - mode pemulihan (`AppState.recovery`, `recovery_required`);
  - urutan `.recover` → `.broken-*` → rename;
  - command pemulihan hanya saat ledger `None`;
  - restore lintas key memakai recovery key milik file backup.
- [ ] **Step 2: Checklist QA manual** (diisi user, release build portable):
  1. Timpa `data/nabungo.db` dengan sampah → app start ke layar pemulihan, lalu pulihkan dari backup otomatis.
  2. Ulangi pada instalasi terenkripsi dengan password.
  3. Restore backup terenkripsi dari folder data lain dengan recovery key-nya, lewat Pengaturan.
  4. "Mulai dengan data kosong" → onboarding, dan file `.broken-*` ada di folder data.
- [ ] **Step 3: Jalankan** suite penuh (perintah di CLAUDE.md: fmt, clippy, `cargo test --workspace`, UI test/typecheck/e2e) → semua lulus.
- [ ] **Step 4: Commit** `docs: Fase 8b selesai — pemulihan`.

## Hasil

_(diisi setelah eksekusi)_
