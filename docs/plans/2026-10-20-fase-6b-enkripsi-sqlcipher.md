# Fase 6b — Enkripsi Database (SEC-01) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enkripsi opsional file DB dengan SQLCipher + master password (Argon2id), recovery key yang bisa dicetak, layar kunci, dan auto-lock saat idle — sesuai SEC-01/D8.

**Architecture:** File DB dienkripsi dengan *data key* (DEK) acak 32 byte yang dipasang sebagai raw key SQLCipher. DEK dibungkus (XChaCha20-Poly1305) oleh kunci turunan Argon2id dari master password dan disimpan di file samping `nabungo.key`; recovery key = DEK itu sendiri dalam Crockford base32, sehingga tidak perlu disimpan di mana pun. Header file DB (plaintext `SQLite format 3\0` atau tidak) adalah sumber kebenaran status enkripsi; `nabungo.key` hanya pembungkus kunci. Saat terkunci, `AppState` tidak memegang `Ledger` sama sekali (koneksi ditutup) dan semua command mengembalikan `locked`.

**Tech Stack:** rusqlite 0.40.2 `bundled-sqlcipher` (SQLCipher 4.14.0 community), `argon2`, `chacha20poly1305`, `zeroize`, `base32`, `serde_json`; Vue 3 + shadcn-vue untuk layar kunci & pengaturan.

**Spec:** `PRD.md` — SEC-01 (§ fitur keamanan), D8, baris 398–399 (Argon2id, auto-lock 5 menit configurable, tanpa password recovery + recovery key printable), 413 (enkripsi opsional saat onboarding), 464 (mitigasi lupa password); `docs/plans/2026-10-12-v1-roadmap.md` baris Fase 6b.

## Hasil spike (2026-10-20, dibuang setelahnya)

Diverifikasi dengan rusqlite 0.40.2 `bundled-sqlcipher` + OpenSSL sistem (Fedora):

| Perilaku | Hasil |
|---|---|
| DB plaintext lama dibuka tanpa key | ✅ tetap terbaca (header `SQLite format 3`) |
| Raw key `PRAGMA key = "x'<64 hex key><32 hex salt>'"` | ✅ 16 byte pertama file = salt (bukan header SQLite) |
| Buka tanpa key / key salah | `NotADatabase` ("file is not a database"); SQLCipher juga menulis `ERROR CORE … hmac check failed` ke stderr |
| Buka dengan key tanpa salt | ✅ salt dibaca dari file |
| Backup API enc → enc (key sama) | ✅ |
| Backup API enc ↔ plaintext | ❌ "backup is not supported with encrypted databases" |
| `ATTACH … KEY …; SELECT sqlcipher_export(…)` plain → enc dan enc → plain | ✅ hasil `journal_mode=delete` |
| `PRAGMA rekey` | ✅ (tidak dipakai di plan ini) |
| `bundled-sqlcipher-vendored-openssl` di Fedora lokal | ❌ butuh modul Perl `FindBin` — karena itu Linux memakai OpenSSL sistem |

## Keputusan desain (call saya, user menyerahkan ke PRD)

- **DEK + key file.** Ganti password = tulis ulang `nabungo.key` saja (cepat, atomik); file DB & backup tidak perlu di-rekey. Recovery key = DEK → tetap valid seumur DB.
- **Header adalah kebenaran.** Header plaintext → DB tidak terenkripsi; `nabungo.key` yang tersisa dianggap sisa crash dan dihapus. Header terenkripsi tanpa `nabungo.key` → hanya bisa dibuka dengan recovery key.
- **Backup ikut status DB.** Backup dibuat dengan key yang sama dengan DB aktif. Saat enkripsi diaktifkan/dimatikan, backup di `backups/` dikonversi (best-effort, dicatat di log). Restore mendeteksi enkripsi sumber dari header-nya dan mengonversi bila perlu, jadi campuran backup lama/baru tetap bisa di-restore.
- **Auto-lock:** pilihan 1/5/10/15/30 menit, default 5, disimpan di tabel `settings`. Hanya aktif bila DB terenkripsi; tanpa password, “kunci” tidak berarti apa-apa.
- **Password minimal 8 karakter** (dihitung per `char`), byte UTF-8 apa adanya, tanpa normalisasi.
- **Argon2id produksi:** m = 65536 KiB, t = 3, p = 1 (disimpan di key file agar bisa dinaikkan nanti). Test memakai `KdfParams::FAST` (m = 8, t = 1, p = 1).

## Non-goals

- Membuka backup terenkripsi dari instalasi **lain** (DEK berbeda) lewat recovery key → layar pemulihan Fase 8 (K9). Di 6b file seperti itu ditolak dengan `invalid_backup`.
- Mengganti recovery key / rekey DB.
- Enkripsi untuk mode browser (`browser-ledger` hanya meniru alur UI-nya, tanpa kriptografi).

## Global Constraints

- Dependensi wajib `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`; `nabungo-core` tidak disentuh.
- Semua open file DB lewat `nabungo_storage` (`file.rs`); jangan `Connection::open` di tempat lain.
- Tidak ada network call; tidak ada aset remote.
- `ui/src/ipc/bindings.ts` hanya di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- Error code baru wajib ada di `errors` di `ui/src/i18n/id.ts` dan `en.ts`.
- DB user tidak boleh dihapus; konversi selalu lewat file `*.partial` + rename atomik.
- Teks peringatan (id), dipakai persis: **"Tidak ada pemulihan password. Jika password dan recovery key sama-sama hilang, data tidak bisa dibuka sama sekali."** (en: "There is no password recovery. If both the password and the recovery key are lost, the data cannot be opened at all.")
- Linux: rusqlite feature `bundled-sqlcipher` (OpenSSL sistem, `openssl-devel`/`libssl-dev`). Windows: tambah `bundled-sqlcipher-vendored-openssl` lewat `[target.'cfg(windows)'.dependencies]`.
- Setiap command yang menjalankan Argon2 (`unlock`, `unlock_with_recovery`, `enable_encryption`, `disable_encryption`, `change_password`) adalah command `async` yang memakai `spawn_blocking`.
- Commit diakhiri `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **WAL basi saat swap file.** Kalau `-wal`/`-shm` milik DB plaintext tertinggal saat file terenkripsi di-rename ke `nabungo.db`, SQLite akan menerapkannya ke file baru → korup/hilang data. Yang diharapkan: semua tulisan terakhir sebelum enable/disable tetap ada. Test: `enable_keeps_latest_write_and_leaves_no_wal` (Task 3).
2. **Crash di tengah enable/disable.** Kombinasi “header plaintext + key file ada” dan “header terenkripsi + key file tidak ada” harus berujung pada keadaan yang bisa dipakai, bukan error startup. Test: `db_state_*` (Task 3).
3. **Restore backup plaintext lama ke DB yang sudah terenkripsi** (dan sebaliknya). Yang diharapkan: restore berhasil dan DB aktif tetap memakai status enkripsinya. Test: `restore_plain_backup_into_encrypted_live` (Task 1) dan `restore_named_plain_backup_after_enable` (Task 3).
4. **Password/recovery key dari input manusia:** unicode, spasi di tepi, recovery key huruf kecil atau dengan `O`/`I`/`L`. Yang diharapkan: password dipakai apa adanya; recovery key dinormalisasi. Test: `password_unicode_roundtrip`, `recovery_key_normalizes_typos` (Task 2).
5. **Command dipanggil saat terkunci** (scheduler backup, UI yang masih memanggil store). Yang diharapkan: error `locked` tanpa panic, scheduler melewati tick, UI diarahkan ke layar kunci. Test: `locked_state_returns_locked_error` (Task 4), `redirects to unlock on locked error` (Task 5).

---

### Task 1: SQLCipher di storage — key, deteksi, konversi, backup/restore berkunci

**Files:**
- Modify: `crates/storage/Cargo.toml`, `crates/storage/src/file.rs`, `crates/storage/src/lib.rs` (`open`, `StorageError`, re-export)
- Create: `crates/storage/src/key.rs`
- Modify: semua pemanggil `open`/`open_file`/`open_file_read_only`/`backup_into*`/`restore_from`/`inspect_backup` di `crates/app` (sementara meneruskan `None` / `self.key.as_ref()` yang belum ada → `None`)
- Modify: `.github/workflows/ci.yml` (Ubuntu: tambah `libssl-dev`)

**Interfaces:**
- Produces:
  - `pub struct DbKey([u8; 32])` (`Zeroize + ZeroizeOnDrop`, `Clone`, `PartialEq`, tanpa `Debug` yang mencetak isi): `DbKey::generate() -> DbKey` (OS RNG), `DbKey::from_bytes([u8; 32])`, `fn as_bytes(&self) -> &[u8; 32]`.
  - `StorageError::KeyRequired` (file terenkripsi, key `None`), `StorageError::WrongKey` (key diberikan tetapi `NotADatabase`).
  - `pub fn is_encrypted_file(path: &Path) -> std::io::Result<bool>`: `false` bila file tidak ada, kosong, atau 16 byte pertama = `b"SQLite format 3\0"`.
  - `open(path, key: Option<&DbKey>)`, `open_file(path, key)`, `open_file_read_only(path, key)`.
  - `backup_into(conn, dest, key)`, `backup_into_unverified(conn, dest, key)`: tujuan dibuka dengan key yang sama dengan DB aktif.
  - `restore_from(conn, src, live_key: Option<&DbKey>)`: bila `is_encrypted_file(src)` sesuai dengan `live_key.is_some()` → backup API langsung (sumber dibuka dengan `live_key`); bila berbeda → `convert_file` ke sibling `<src>.restore-partial` dengan `live_key`, backup API dari situ, lalu hapus partial.
  - `inspect_backup(path, key: Option<&DbKey>) -> Result<usize>`: key dipakai hanya bila file terenkripsi.
  - `pub fn convert_file(src: &Path, src_key: Option<&DbKey>, dest: &Path, dest_key: Option<&DbKey>) -> Result<()>`: `ATTACH` tujuan (`KEY ''` untuk plaintext) + `sqlcipher_export`, lalu **set `user_version` tujuan = sumber secara eksplisit**, lalu `quick_check` tujuan; ditulis ke `dest.partial` lalu rename.
- Key dipasang tepat setelah `open_raw`, sebelum PRAGMA lain: `PRAGMA key = "x'<hex>'"` (hex 64 karakter, tanpa salt → SQLCipher membuat salt acak per file). Setelah key dipasang, `SELECT count(*) FROM sqlite_master` untuk memaksa verifikasi: `NotADatabase` → `WrongKey`. Tanpa key pada file terenkripsi → `KeyRequired` (cek `is_encrypted_file` sebelum open).

- [ ] **Step 1: Ganti feature rusqlite dan pastikan workspace masih hijau**

`crates/storage/Cargo.toml`: `features = ["bundled-sqlcipher", "backup"]`, plus
```toml
[target.'cfg(windows)'.dependencies]
rusqlite = { version = "0.40.2", features = ["bundled-sqlcipher-vendored-openssl"] }
```
Tambah `zeroize = { version = "1", features = ["derive"] }` dan `getrandom` (atau `rand_core` dengan `OsRng`) untuk `DbKey::generate`.

Run: `cargo test --workspace > /tmp/... 2>&1; tail`
Expected: PASS (semua test lama; SQLCipher kompatibel dengan file plaintext).

- [ ] **Step 2: Tulis test yang gagal** di `crates/storage/src/file.rs` (`mod tests`, memakai `tempfile`):

```rust
#[test] fn plaintext_file_is_not_encrypted()      // open(p, None) → is_encrypted_file == false
#[test] fn missing_or_empty_file_is_not_encrypted()
#[test] fn keyed_file_roundtrip_and_is_encrypted() // open(p, Some(&k)) tulis; is_encrypted_file == true; buka lagi dgn k → data ada
#[test] fn encrypted_without_key_is_key_required() // matches!(open_file(p, None), Err(StorageError::KeyRequired))
#[test] fn encrypted_with_wrong_key_is_wrong_key() // matches!(…, Err(StorageError::WrongKey))
#[test] fn convert_plain_to_encrypted_and_back()   // count baris sama, schema_version sama (== latest_version()), header sesuai
#[test] fn backup_of_encrypted_db_is_encrypted()   // backup_into(conn, dest, Some(&k)); is_encrypted_file(dest); inspect_backup(dest, Some(&k)) == Ok(latest)
#[test] fn restore_plain_backup_into_encrypted_live() // backup plaintext → restore_from(enc_conn, plain, Some(&k)) → data sumber terbaca; file live tetap terenkripsi
#[test] fn restore_encrypted_backup_into_plain_live()
```

- [ ] **Step 3: Run test — Expected: FAIL (compile: `DbKey`/`is_encrypted_file`/`convert_file` belum ada)**

Run: `cargo test -p nabungo-storage file::`

- [ ] **Step 4: Implementasi** sesuai Interfaces. Perbarui pemanggil di `crates/app` (dan `crates/app/tests/crash_consistency.rs` bila perlu) agar meneruskan `None`; perilaku lama tidak berubah.

- [ ] **Step 5: Run — Expected: PASS**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

- [ ] **Step 6: Commit** — `feat(storage): SQLCipher — key DB, deteksi enkripsi, konversi, backup/restore berkunci` (sertakan perubahan CI `libssl-dev`).

---

### Task 2: Vault — Argon2id, key file, recovery key

**Files:**
- Create: `crates/app/src/vault.rs`
- Modify: `crates/app/Cargo.toml` (`argon2`, `chacha20poly1305`, `base32`, `serde`, `serde_json`, `zeroize`), `crates/app/src/lib.rs`, `crates/app/src/ledger.rs` (`AppError`)

**Interfaces:**
- Consumes: `nabungo_storage::DbKey`.
- Produces:
  - `pub struct KdfParams { pub m_kib: u32, pub t: u32, pub p: u32 }` dengan `KdfParams::DEFAULT = { 65536, 3, 1 }` dan `KdfParams::FAST = { 8, 1, 1 }`.
  - `pub struct KeyFile` (serde JSON): `{ "v": 1, "kdf": "argon2id", "m_kib", "t", "p", "salt": b64(16), "nonce": b64(24), "wrapped": b64 }`. AEAD XChaCha20-Poly1305, AAD = `b"nabungo-keyfile-v1"`.
  - `KeyFile::seal(dek: &DbKey, password: &str, params: KdfParams) -> Result<KeyFile, AppError>` (salt & nonce acak).
  - `KeyFile::unseal(&self, password: &str) -> Result<DbKey, AppError>`: tag gagal → `AppError::WrongPassword`; `v != 1` atau JSON rusak → `AppError::KeyFileInvalid`.
  - `KeyFile::read(path) -> Result<KeyFile, AppError>`, `KeyFile::write_atomic(&self, path) -> Result<(), AppError>` (`<path>.partial` + `sync_all` + rename).
  - `pub fn keyfile_path(db: &Path) -> PathBuf` = `db.with_extension("key")`.
  - `pub fn validate_password(pw: &str) -> Result<(), AppError>`: `< 8` char → `AppError::PasswordTooShort`.
  - `pub fn format_recovery_key(dek: &DbKey) -> String`: Crockford base32 tanpa padding (52 karakter), dikelompokkan 4 karakter dengan `-` (13 kelompok).
  - `pub fn parse_recovery_key(s: &str) -> Result<DbKey, AppError>`: buang spasi/`-`, uppercase, `O→0`, `I→1`, `L→1`; panjang hasil decode ≠ 32 → `AppError::InvalidRecoveryKey`.
  - `AppError` baru: `WrongPassword`, `InvalidRecoveryKey`, `PasswordTooShort`, `KeyFileInvalid`, `Locked`, `EncryptionState` (enable saat sudah terenkripsi / disable saat tidak); `StorageError::WrongKey`/`KeyRequired` dipetakan ke `WrongPassword`/`Locked` di `From`.

- [ ] **Step 1: Tulis test yang gagal** di `vault.rs`:

```rust
#[test] fn seal_unseal_roundtrip()          // unseal("rahasia-123") == dek
#[test] fn wrong_password_rejected()        // matches!(kf.unseal("salah-1234"), Err(AppError::WrongPassword))
#[test] fn password_unicode_roundtrip()     // "páss wörd 🔑 " (dengan spasi di akhir) roundtrip; "páss wörd 🔑" tanpa spasi → WrongPassword
#[test] fn keyfile_json_roundtrip_atomic()  // write_atomic lalu read → unseal OK; tidak ada *.partial tersisa
#[test] fn keyfile_bad_version_is_invalid() // ubah "v" jadi 2 → KeyFileInvalid
#[test] fn password_min_length()            // "1234567" → PasswordTooShort; "12345678" OK; "ééééééé" (7 char) → PasswordTooShort
#[test] fn recovery_key_format()            // 13 kelompok × 4, dipisah '-', total 64 karakter
#[test] fn recovery_key_roundtrip()
#[test] fn recovery_key_normalizes_typos()  // lowercase + spasi + O untuk 0 / L untuk 1 → DbKey sama
#[test] fn recovery_key_wrong_length_rejected()
```
Semua test memakai `KdfParams::FAST`.

- [ ] **Step 2: Run — Expected: FAIL (modul belum ada)** — `cargo test -p nabungo-app vault::`
- [ ] **Step 3: Implementasi** sesuai Interfaces. `KeyFile` menyimpan params dari saat seal; `unseal` memakai params dari file.
- [ ] **Step 4: Run — Expected: PASS** — `cargo test -p nabungo-app vault:: && cargo clippy -p nabungo-app --all-targets -- -D warnings`
- [ ] **Step 5: Commit** — `feat(app): vault — Argon2id, key file, recovery key`

---

### Task 3: Ledger berkunci — buka, aktif/nonaktif enkripsi, ganti password, status file

**Files:**
- Create: `crates/app/src/security.rs`
- Modify: `crates/app/src/ledger.rs` (field `key: Option<DbKey>`), `crates/app/src/backup.rs` (teruskan `self.key.as_ref()`; `ensure_not_live` juga menolak `keyfile_path`), `crates/app/src/lib.rs`, `crates/app/tests/crash_consistency.rs`

**Interfaces:**
- Consumes: Task 1 (`open*`, `convert_file`, `is_encrypted_file`, `DbKey`), Task 2 (`KeyFile`, `keyfile_path`, `parse_recovery_key`, `validate_password`, `format_recovery_key`).
- Produces:
  - `Ledger::open_with_backups(db, backups, key: Option<&DbKey>, now)` (parameter `key` baru, sebelum `now`); `Ledger::open(path)` tetap plaintext.
  - `pub enum DbState { Plain, Encrypted { has_keyfile: bool } }`, `pub fn db_state(db: &Path) -> Result<DbState, AppError>`: bila header plaintext dan `keyfile_path` ada → hapus key file (sisa crash) dan hapus `nabungo.db.convert` / `nabungo.db.convert.partial` bila ada (berlaku juga saat `Encrypted`); DB yang belum ada = `Plain`.
  - `pub fn unlock_key(db: &Path, password: &str) -> Result<DbKey, AppError>`: key file tidak ada → `KeyFileInvalid`.
  - `pub fn recover_key(db: &Path, recovery: &str, new_password: &str, params: KdfParams) -> Result<DbKey, AppError>`: validasi password dulu; parse; verifikasi dengan `open_file_read_only(db, Some(&k))` (`WrongKey` → `InvalidRecoveryKey`); tulis key file baru.
  - `Ledger::is_encrypted(&self) -> bool`.
  - `Ledger::enable_encryption(&mut self, password: &str, backups: &Path, params: KdfParams) -> Result<String, AppError>` → recovery key terformat. Urutan wajib:
    1. `EncryptionState` bila sudah terenkripsi; `validate_password`.
    2. `DbKey::generate()`; `convert_file(db, None, <db>.convert, Some(&dek))` — `<db>.convert` = sibling `nabungo.db.convert`; `convert_file` sendiri menulis lewat `nabungo.db.convert.partial`.
    3. `KeyFile::seal(...).write_atomic(keyfile_path(db))`.
    4. Tutup koneksi (ganti `self.conn` dengan koneksi in-memory sementara), hapus `<db>-wal`/`-shm` yang tersisa, `rename(<db>.convert → db)`, buka ulang dengan `open(db, Some(&dek))`, set `self.key`.
    5. Konversi tiap file di `list_backups(backups)` yang masih plaintext ke terenkripsi (convert ke sibling lalu rename); kegagalan per file → `tracing::warn!`, tidak menggagalkan use case.
  - `Ledger::disable_encryption(&mut self, password: &str, backups: &Path) -> Result<(), AppError>`: kebalikannya — verifikasi password lewat key file → convert ke plaintext → tutup → hapus `-wal`/`-shm` → rename → **baru** hapus key file → buka ulang → dekripsi backup (best-effort).
  - `Ledger::change_password(&self, old: &str, new: &str, params: KdfParams) -> Result<(), AppError>`: `EncryptionState` bila tidak terenkripsi; validasi `new`; unseal dengan `old`; seal ulang DEK yang sama; `write_atomic`.

- [ ] **Step 1: Tulis test yang gagal** di `security.rs` (tempdir, `KdfParams::FAST`):

```rust
#[test] fn enable_encrypts_file_and_returns_recovery_key() // is_encrypted_file(db); key file ada; recovery key 64 karakter
#[test] fn enable_keeps_latest_write_and_leaves_no_wal()   // catat akun tepat sebelum enable → ada setelah reopen dengan unlock_key; tidak ada <db>-wal dari sebelum swap (cek isi akun, bukan sekadar file)
#[test] fn enable_twice_is_encryption_state_error()
#[test] fn enable_rejects_short_password()                 // DB tetap plaintext, tidak ada key file
#[test] fn unlock_with_password_opens_ledger()             // open_with_backups(db, backups, Some(&unlock_key(db,pw)?), now) → saldo sama
#[test] fn unlock_wrong_password()                         // WrongPassword
#[test] fn recover_with_recovery_key_sets_new_password()   // recover_key → password lama ditolak, baru diterima
#[test] fn recover_with_wrong_recovery_key()               // InvalidRecoveryKey; key file lama tidak berubah
#[test] fn change_password_keeps_data_readable()
#[test] fn disable_restores_plaintext_and_removes_keyfile()
#[test] fn backups_follow_encryption_state()               // backup sebelum enable jadi terenkripsi setelah enable, plaintext lagi setelah disable
#[test] fn restore_named_plain_backup_after_enable()       // tulis file backup plaintext (salinan sebelum enable) ke backups/ secara manual, restore_named → berhasil, DB tetap terenkripsi
#[test] fn db_state_plain_with_stale_keyfile_cleans_up()   // tulis key file palsu di samping DB plaintext → DbState::Plain, key file hilang
#[test] fn db_state_encrypted_without_keyfile()            // enable lalu hapus key file → Encrypted { has_keyfile: false }; recover_key tetap bisa membuka
#[test] fn backup_target_keyfile_is_rejected()             // backup_to(keyfile_path(db)) → BackupTargetInvalid
```

- [ ] **Step 2: Run — Expected: FAIL (compile)** — `cargo test -p nabungo-app security::`
- [ ] **Step 3: Implementasi** sesuai Interfaces; semua pemanggil backup/restore di `backup.rs` meneruskan `self.key.as_ref()`.
- [ ] **Step 4: Run — Expected: PASS** — `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- [ ] **Step 5: Commit** — `feat(app): enkripsi DB — aktif/nonaktif, unlock, recovery key, ganti password`

---

### Task 4: Tauri — status terkunci, command keamanan, startup & scheduler

**Files:**
- Modify: `src-tauri/src/state.rs`, `src-tauri/src/lib.rs` (`init`, `collect_commands!`), `src-tauri/src/lifecycle.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/error.rs`, `src-tauri/src/dto.rs` (atau tempat DTO berada), `crates/app/src/settings.rs`
- Regenerate: `ui/src/ipc/bindings.ts`

**Interfaces:**
- Consumes: Task 3 (`db_state`, `unlock_key`, `recover_key`, `Ledger::{open_with_backups, enable_encryption, disable_encryption, change_password, is_encrypted}`), `KdfParams::DEFAULT`.
- Produces:
  - `AppState.ledger: Mutex<Option<Ledger>>`. Helper `ledger(&state) -> Result<LedgerGuard<'_>, IpcError>` dengan `pub struct LedgerGuard<'a>(MutexGuard<'a, Option<Ledger>>)` yang `Deref/DerefMut<Target = Ledger>` dan hanya dibuat bila `Some` → 56 call site lama tidak berubah; `None` → `ErrorCode::Locked`.
  - `lifecycle::open_unlocked(app: &AppHandle, key: Option<DbKey>) -> Result<(), AppError>`: dipindahkan dari `init` — `open_with_backups`, integrity check bila sesi sebelumnya tidak bersih, auto-backup, simpan ke state, `spawn_backup_scheduler`. `init` memanggilnya langsung bila `DbState::Plain`; bila `Encrypted`, `init` hanya me-manage state dengan `ledger: None` dan `StartupReport.integrity = Skipped` (cek dijalankan saat unlock). `unclean_shutdown` disimpan di state agar dipakai saat unlock.
  - Scheduler: `None` → lewati tick tanpa log warning.
  - `UiSettings.auto_lock_minutes: u8` (key `auto_lock_minutes`, default 5, nilai sah `[1, 5, 10, 15, 30]`, nilai tak dikenal → default) + `Ledger::set_auto_lock_minutes(u8) -> Result<(), AppError>` (`AppError::InvalidAutoLock` bila di luar daftar).
  - Command (semua lewat `ledger()`/state, didaftarkan di `collect_commands!`):
    - `security_status() -> SecurityStatusDto { encrypted: bool, locked: bool, has_keyfile: bool }` (tidak butuh ledger).
    - `async unlock(password: String) -> Result<(), IpcError>`
    - `async unlock_with_recovery(recovery_key: String, new_password: String) -> Result<(), IpcError>`
    - `lock() -> Result<(), IpcError>`: `EncryptionState` bila DB tidak terenkripsi; selain itu set `None`.
    - `async enable_encryption(password: String) -> Result<String, IpcError>` (recovery key)
    - `async disable_encryption(password: String) -> Result<(), IpcError>`
    - `async change_password(old_password: String, new_password: String) -> Result<(), IpcError>`
    - `set_auto_lock_minutes(minutes: u8) -> Result<(), IpcError>`; `UiSettingsDto` mendapat `auto_lock_minutes`.
  - `ErrorCode` baru (snake_case di TS): `Locked`, `WrongPassword`, `InvalidRecoveryKey`, `PasswordTooShort`, `KeyFileInvalid`, `EncryptionState`, `InvalidAutoLock`.

- [ ] **Step 1: Tulis test yang gagal**
  - `crates/app/src/settings.rs`: `auto_lock_default_and_roundtrip` (default 5; set 15 → 15; set 7 → `InvalidAutoLock`).
  - `src-tauri/src/commands.rs` (atau `state.rs`): `locked_state_returns_locked_error` — `Mutex::new(None)` → `ledger()` mengembalikan `IpcError { code: ErrorCode::Locked, .. }`; `Some(Ledger::open_in_memory())` → `Ok`.
  - `src-tauri/src/error.rs`: mapping `AppError::WrongPassword → ErrorCode::WrongPassword` dst. untuk ketujuh varian (satu test tabel).
- [ ] **Step 2: Run — Expected: FAIL (compile)** — `cargo test -p nabungo-app settings:: && cargo test -p nabungo-desktop`
- [ ] **Step 3: Implementasi** sesuai Interfaces; `unlock*`/`enable`/`disable`/`change_password` memakai `tauri::async_runtime::spawn_blocking`, mengunci mutex di dalam closure.
- [ ] **Step 4: Regenerate bindings + verifikasi**

Run: `cargo test -p nabungo-desktop export_bindings && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && pnpm --filter @nabungo/ui typecheck`
Expected: Rust PASS; typecheck **FAIL** hanya karena error code baru belum ada di `errors` i18n → tambahkan ke `id.ts`/`en.ts` sekarang (teks: `locked` "Aplikasi terkunci" / "The app is locked"; `wrong_password` "Password salah" / "Wrong password"; `invalid_recovery_key` "Recovery key tidak cocok" / "Recovery key does not match"; `password_too_short` "Password minimal 8 karakter" / "Password must be at least 8 characters"; `key_file_invalid` "File kunci rusak atau hilang; pakai recovery key" / "Key file is damaged or missing; use the recovery key"; `encryption_state` "Status enkripsi sudah berubah; muat ulang halaman" / "Encryption state already changed; reload the page"; `invalid_auto_lock` "Durasi kunci otomatis tidak valid" / "Invalid auto-lock duration"), lalu typecheck PASS.

- [ ] **Step 5: Commit** — `feat(desktop): command keamanan, state terkunci, unlock saat startup`

---

### Task 5: UI — ipc, browser-ledger, store keamanan, guard router, layar kunci

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser-ledger.ts`, `ui/src/router.ts`, `ui/src/App.vue`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`
- Create: `ui/src/stores/security.ts` (+ `.test.ts`), `ui/src/views/UnlockView.vue` (+ `.test.ts`), `ui/src/composables/useIdleLock.ts` (+ `.test.ts`)
- Modify: `ui/src/router.test.ts`

**Interfaces:**
- Consumes: command Task 4 lewat bindings.
- Produces:
  - `ipc`: `securityStatus()`, `unlock(pw)`, `unlockWithRecovery(key, newPw)`, `lock()`, `enableEncryption(pw): Promise<string>`, `disableEncryption(pw)`, `changePassword(old, new)`, `setAutoLockMinutes(n)`.
  - `browser-ledger`: tiruan tanpa kripto — menyimpan password di memori; `enableEncryption` mengembalikan recovery key berformat sama (13×4, Crockford) dan menyimpannya; `lock` membuat semua fungsi lain melempar `{ code: 'locked' }`; aturan error sama dengan Rust (`password_too_short` < 8 char, `wrong_password`, `invalid_recovery_key` dengan normalisasi yang sama, `encryption_state`, `invalid_auto_lock`).
  - Store `useSecurityStore`: state `{ encrypted, locked, hasKeyfile, autoLockMinutes }`, aksi `refresh()`, `unlock()`, `unlockWithRecovery()`, `lock()`, `enable(pw): Promise<string>`, `disable()`, `changePassword()`, `setAutoLock()`; setelah setiap aksi tulis memuat ulang dari `securityStatus()`/`getUiSettings()`.
  - Route `unlock` (`/unlock`, `meta.bare`). Guard: cek `securityStatus()` dulu — `locked` → `{ name: 'unlock' }` untuk semua route; tidak terkunci + menuju `unlock` → `dashboard`; baru setelah itu guard onboarding. `RouteName` mendapat `'unlock'`.
  - Penanganan global: `IpcFailure` dengan code `locked` (di `ipc/index.ts` titik tunggal pemanggil) → `security.locked = true` dan `router.replace({ name: 'unlock' })`.
  - `useIdleLock(minutes: Ref<number>, enabled: Ref<boolean>, onIdle: () => void)`: reset pada `pointerdown`, `pointermove`, `keydown`, `wheel`, `touchstart` (listener `window`, `passive`); dipasang di `App.vue` dengan `enabled = encrypted && !locked`; `onIdle` → `security.lock()` lalu ke `unlock`. Store lain direset saat lock (panggil `$reset` atau muat ulang setelah unlock) agar data tidak tampil di balik layar kunci.
  - `UnlockView.vue` (ikuti `docs/design/ui-guidelines.md`, layar `bare` terpusat dengan `AppLogo`): mode password (input + tombol "Buka") dan mode "Lupa password? Pakai recovery key" (input recovery key monospace + password baru + konfirmasi); error ditampilkan lewat `errors.<code>`. Sukses → `router.replace(redirect ?? dashboard)`.

- [ ] **Step 1: Tulis test yang gagal**
  - `security.test.ts`: `enable lalu lock lalu unlock` (browser-ledger), `unlock salah → wrong_password, tetap locked`, `recovery mengganti password`.
  - `router.test.ts`: `terkunci → semua route ke unlock`, `tidak terkunci → unlock ke dashboard`, `terkunci mendahului onboarding`.
  - `useIdleLock.test.ts` (fake timers): `memanggil onIdle setelah N menit tanpa aktivitas`, `keydown me-reset timer`, `tidak memanggil bila enabled=false`, `perubahan minutes berlaku`.
  - `UnlockView.test.ts`: `password salah menampilkan pesan`, `mode recovery memvalidasi konfirmasi password`, `redirects to unlock on locked error` (panggil fungsi ipc saat terkunci → route jadi `unlock`).
- [ ] **Step 2: Run — Expected: FAIL** — `pnpm --filter @nabungo/ui test`
- [ ] **Step 3: Implementasi** sesuai Interfaces; teks i18n di `id.ts` & `en.ts` (namespace `security`).
- [ ] **Step 4: Run — Expected: PASS** — `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test`
- [ ] **Step 5: Commit** — `feat(ui): layar kunci, auto-lock idle, store keamanan`

---

### Task 6: UI — pengaturan keamanan, dialog recovery key, opsi enkripsi di onboarding

**Files:**
- Create: `ui/src/components/settings/SecuritySection.vue` (+ `.test.ts`), `ui/src/components/RecoveryKeyDialog.vue` (+ `.test.ts`), `ui/src/components/EncryptionFields.vue` (password + konfirmasi + checkbox peringatan; dipakai di Settings & onboarding)
- Modify: `ui/src/views/SettingsView.vue`, `ui/src/views/OnboardingView.vue` (+ test), `ui/src/style.css` (aturan `@media print`), i18n

**Interfaces:**
- Consumes: `useSecurityStore` (Task 5).
- Produces:
  - `EncryptionFields` (`v-model` `{ password, confirm, acknowledged }`, emit `valid: boolean`): valid bila password ≥ 8 char, konfirmasi sama, dan checkbox peringatan dicentang. Teks peringatan = Global Constraints.
  - `RecoveryKeyDialog` (prop `recoveryKey: string`, `open`): kunci dalam font monospace per kelompok; tombol **Salin** (clipboard) dan **Cetak** (`window.print()`; CSS print hanya menampilkan elemen `.print-recovery` berisi judul "Recovery key Nabungo", tanggal, kunci, dan teks peringatan); tombol **Selesai** baru aktif setelah checkbox "Saya sudah menyimpan recovery key ini" dicentang; tidak bisa ditutup dengan Esc/klik luar.
  - `SecuritySection` di Settings (section `settings-security`, di atas Backup):
    - Tidak terenkripsi: penjelasan singkat + tombol "Aktifkan enkripsi" → dialog `EncryptionFields` → `enable()` → `RecoveryKeyDialog`.
    - Terenkripsi: badge "Terenkripsi"; select "Kunci otomatis" (1/5/10/15/30 menit); tombol "Kunci sekarang", "Ganti password" (lama + baru + konfirmasi), "Matikan enkripsi" (`ConfirmDialog` + password).
  - Onboarding: bagian akhir "Enkripsi database (opsional)" dengan switch default **mati**; bila menyala tampil `EncryptionFields` dan tombol selesai memerlukan `valid`. Alur: `completeOnboarding` → bila switch menyala `enableEncryption` → `RecoveryKeyDialog` → dashboard. Bila `enableEncryption` gagal, onboarding tetap selesai dan toast menampilkan error + "aktifkan nanti dari Pengaturan".

- [ ] **Step 1: Tulis test yang gagal**
  - `EncryptionFields`: `tidak valid sampai peringatan dicentang`, `konfirmasi berbeda → tidak valid`.
  - `RecoveryKeyDialog.test.ts`: `Selesai nonaktif sampai checkbox dicentang`, `Salin menulis kunci ke clipboard` (stub `navigator.clipboard`).
  - `SecuritySection.test.ts`: `aktifkan → menampilkan recovery key dari ipc`, `terenkripsi → menampilkan kunci otomatis 5 menit`, `ganti password salah → pesan wrong_password`.
  - `OnboardingView.test.ts`: `enkripsi default mati`, `menyalakan enkripsi → enableEncryption dipanggil setelah completeOnboarding dan dialog recovery key tampil`.
- [ ] **Step 2: Run — Expected: FAIL** — `pnpm --filter @nabungo/ui test`
- [ ] **Step 3: Implementasi** sesuai Interfaces dan `docs/design/ui-guidelines.md` (komponen shadcn yang sudah ada: Dialog, Switch, Select, Checkbox, Button, Input; tambah komponen shadcn hanya lewat CLI sesuai CLAUDE.md).
- [ ] **Step 4: Run — Expected: PASS** — `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui build`
- [ ] **Step 5: Commit** — `feat(ui): pengaturan keamanan, recovery key yang bisa dicetak, enkripsi opsional saat onboarding`

---

### Task 7: E2E, dokumentasi, checklist manual

**Files:**
- Create: `ui/e2e/security.spec.ts`
- Modify: `CLAUDE.md`, `docs/plans/2026-10-12-v1-roadmap.md`

- [ ] **Step 1: Tulis e2e** (browser-ledger): onboarding dengan enkripsi menyala → dialog recovery key → centang → dashboard; Pengaturan → "Kunci sekarang" → layar kunci; password salah → pesan; password benar → kembali ke dashboard dengan data utuh; recovery key (disalin dari langkah awal) + password baru → terbuka.
- [ ] **Step 2: Run — Expected: PASS** — `pnpm --filter @nabungo/ui e2e`
- [ ] **Step 3: Dokumentasi**
  - `CLAUDE.md` § Backup & file DB: SQLCipher (`bundled-sqlcipher`, OpenSSL sistem di Linux, vendored di Windows), `nabungo.key` di folder data (jangan dihapus; tanpa file itu DB hanya bisa dibuka dengan recovery key), header file = sumber kebenaran status enkripsi, konversi lewat `convert_file` (`sqlcipher_export`) karena backup API tidak bisa lintas enkripsi, `AppState.ledger` = `None` saat terkunci. Baris folder data: tambah `nabungo.key`.
  - Roadmap: Fase 6b → selesai + ringkasan + penundaan (pemulihan backup dari instalasi lain → Fase 8 K9).
- [ ] **Step 4: Verifikasi penuh** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui e2e` → semua PASS.
- [ ] **Step 5: Commit** — `docs: Fase 6b selesai`

**Checklist manual Tauri (dijalankan user, `pnpm dev`):**
1. DB lama (plaintext) tetap terbuka tanpa password setelah upgrade.
2. Aktifkan enkripsi dari Pengaturan → recovery key tampil → **Cetak** membuka dialog cetak sistem dan pratinjau hanya berisi recovery key.
3. Tutup app → buka lagi → layar kunci; password salah ditolak; password benar → data utuh.
4. Diam 1 menit (setel kunci otomatis 1 menit) → layar kunci muncul.
5. Hapus `nabungo.key` saat app tertutup → buka app → hanya recovery key yang bisa membuka, lalu password baru berlaku.
6. `file nabungo.db` / buka dengan `sqlite3` → bukan database (terenkripsi); file di `backups/` juga.
7. Matikan enkripsi → `nabungo.key` hilang, `sqlite3 nabungo.db .tables` terbaca.
8. Build Windows (CI) lulus dengan OpenSSL vendored.
