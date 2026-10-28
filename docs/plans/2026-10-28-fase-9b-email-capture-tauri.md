# Fase 9b — Email Capture: Integrasi Tauri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EML-01 bisa dipakai dari aplikasi. User menghubungkan Gmail dari Settings: alamat, App Password, dan label. Password disimpan di OS keychain. App lalu men-sync label itu saat dibuka/di-unlock, tiap 15 menit, dan saat tombol "Sync sekarang" ditekan. Hasilnya masuk staging `email_capture`/`email_unparsed` dari 9a. Inbox review belum ada (9c). Tanpa konfigurasi, app tetap tidak membuka koneksi jaringan.

**Architecture:**
- `nabungo-app`:
  - `secret.rs`: trait `SecretStore` + `KeyringStore` (OS keychain) + `MemorySecretStore` (test).
  - `email_settings.rs`: setting akun email di tabel `setting`. Hanya alamat dan label; password tidak pernah ada di sini.
  - `email_sync.rs` diperluas: sync memakai `LedgerAccess`, sehingga mutex ledger hanya dikunci per email, tidak selama fetch jaringan.
  - `email_runner.rs`: orkestrasi satu putaran (setting → keychain → connect → sync) dan `SyncSchedule` (kapan putaran berikutnya jatuh tempo).
- `src-tauri`:
  - `email.rs`: menjalankan putaran sync (flag running, event, status terakhir) dan thread `email-sync`. Pola thread-nya sama dengan `spawn_backup_scheduler`.
  - Command tipis di `commands.rs` dan DTO di `dto.rs`.
- `ui`: `EmailSection.vue` di Settings, facade `ipc` + fallback browser, event progress.

**Tech Stack:**
- Rust: `keyring-core` + store native per OS. Linux: `dbus-secret-service-keyring-store` dengan crypto OpenSSL, memakai libdbus sistem yang sudah dipakai WebKitGTK. Windows: `windows-native-keyring-store`. Task 1 adalah gerbang ukurannya.
- Event bertipe lewat `tauri-specta` (`Event` derive, `collect_events!`).
- UI: Vue 3 + shadcn-vue.

**Spec:**
- `PRD.md`:
  - EML-01 (§5), D13 (§3).
  - Acceptance "Tanpa jaringan secara default" (§5).
  - Baris Offline & Privasi (§8): credential hanya di keychain; 0 paket keluar dengan konfigurasi default.
- Brief: `docs/plans/Brief Auto-Capture Transaksi via IMAP.md`, terutama §Setup Gmail, §Alur sync, §Stack (kontrak `sync_email` + event), dan milestone 4.
- Plan 9a: `docs/plans/2026-10-27-fase-9a-email-capture-backend.md` (K4 cursor per email; `sync_mailbox` dirancang untuk dipecah per batch).

**Di luar plan ini (9c):**
- Layar Inbox (Pending/Unparsed), confirm/edit/ignore di UI, duplicate hint.
- Data capture di browser-ledger, rule editor, re-parse unparsed.
- Multi-akun email; interval sync yang bisa diubah user.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Keychain**: `keyring-core` + store native per OS (lihat Tech Stack), bukan crate `keyring` 4 fitur `v1`. Fitur `v1` memakai zbus, yang menarik runtime async. Entry: service `nabungo.email`, user = alamat email (lowercase). Gerbang Task 1: build Windows CI lulus dan installer naik ≤ 1 MB. Bila gagal, berhenti dan laporkan sebelum Task 2. | D2 tanpa runtime async; batas installer < 15 MB. |
| K2 | **Setting** (tabel `setting`): `email.enabled` (`"1"`/`"0"`), `email.address`, `email.mailbox` (default `Finance`), `email.last_sync_at` (ISO-8601 UTC, ditulis hanya saat putaran sukses). Tidak ada migrasi baru. | Data sudah per-DB; credential tetap di keychain (PRD §8). |
| K3 | **Connect** = validasi → login + `EXAMINE` label (di luar mutex) → simpan password ke keychain → simpan setting `enabled=1` → putaran sync pertama dijadwalkan segera. Bila login/label gagal, keychain dan setting tidak berubah. Password `None` saat sudah terhubung = pakai password tersimpan (ganti label saja). Ganti alamat → entry keychain alamat lama dihapus. | User tidak bisa menyimpan konfigurasi yang tidak jalan. |
| K4 | **App Password dinormalisasi**: semua whitespace dibuang sebelum dipakai/disimpan. Gmail menampilkannya sebagai `abcd efgh ijkl mnop`. | Salin-tempel dari Google menyertakan spasi. |
| K5 | **Disconnect** = hapus entry keychain (tidak ada pun tidak error), `email.enabled=0`. Alamat, label, cursor, capture, dan unparsed tetap disimpan. | Data staging milik user; menghubungkan lagi melanjutkan cursor. |
| K6 | **Lock per email**: `sync_mailbox_shared` mengambil rule dan cursor dalam satu lock, lalu `fetch_after` tanpa lock, lalu satu lock per `ingest_email`. Ledger `None` (terkunci) → `SyncError::Locked`. Sebelum tiap ingest, cursor di DB dibandingkan dengan cursor yang diharapkan; beda (DB diganti restore/recovery di tengah sync) → `SyncError::Interrupted`, berhenti tanpa menulis. `sync_mailbox` (CLI) tetap ada sebagai pembungkus. | UI tidak macet selama fetch IMAP; restore tidak membuat cursor meloncati email. |
| K7 | **Jadwal**: interval tetap **15 menit**, dicek tiap 30 detik oleh thread `email-sync` (satu per proses). Putaran pertama setelah dasbor tampil (`ready_logged`, maks 30 s), dan putaran segera setelah unlock/restore/connect (`SyncSchedule::reset`). Thread dilewati bila terkunci, mode pemulihan, `email.enabled != 1`, atau sync sedang berjalan. Keychain dan jaringan **tidak disentuh** bila email belum dikonfigurasi. | Pilihan user (15 menit). Firewall test tetap 0 paket. |
| K8 | **Auth gagal menjeda sync otomatis** (`SyncSchedule.paused = true`) sampai user connect ulang. "Sync sekarang" tetap boleh dan menampilkan error. Error lain (koneksi, label) tidak menjeda; dicoba lagi di putaran berikutnya. | Login gagal berulang tiap 15 menit bisa membuat Google memblokir akun. |
| K9 | **Satu sync pada satu waktu**: `AtomicBool running` di `EmailRuntime`. "Sync sekarang" saat running → `email_sync_running`; thread melewati tick. Flag dilepas lewat guard (`Drop`) agar panic/error tidak membuatnya macet. | Dua koneksi IMAP paralel menulis cursor yang sama. |
| K10 | **Offset lokal** dibaca sekali di awal `run()`, sebelum ada thread lain: `UtcOffset::current_local_offset()`, fallback `+07:00`. Nilainya disimpan di `EmailRuntime.local_offset`. | Crate `time` mengembalikan error untuk `current_local_offset` di proses multi-thread (Linux); hasilnya fallback UTC dan tanggal email jam 00–07 WIB bergeser sehari. |
| K11 | **Event** (tauri-specta, bertipe): `EmailSyncProgress { processed, new_items, unparsed }` setelah tiap batch; `EmailSyncDone { summary: Option<SyncSummaryDto>, error: Option<IpcError> }` di akhir setiap putaran, otomatis maupun manual. Event tidak pernah memuat alamat, password, atau isi email. | Kontrak brief §Stack. |
| K12 | **Status** `email_status`: `{ enabled, address, mailbox, last_sync_at, running, last_error, paused }`. `last_error` dan `paused` hanya di memori (`EmailRuntime`), tidak dipersist. | Restart = coba lagi dari bersih. |
| K13 | **Log**: hanya hitungan (`new_items`, `unparsed`, `seen`) dan jenis error. Alamat email, password, dan isi email tidak dicatat. `SourceError::Connection(msg)` dicatat `warn` (pesan server/OS, tanpa credential). | PRD §8 Privasi. |
| K14 | **Mode browser**: `browser-ledger` menyimpan setting email in-memory. Validasinya sama dengan Rust (alamat/label, `email_not_configured`). Password `wrong` → `email_auth_failed`, label `Missing` → `email_mailbox_not_found`, selain itu sukses. `syncEmail` mengembalikan ringkasan nol dan memancarkan `done`. | Test UI tanpa Tauri; data capture di browser menunggu 9c. |

## Global Constraints

- Arah dependensi tetap `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage, nabungo-mail)`. Dependensi keychain hanya di `nabungo-app`. `src-tauri` tidak memanggil `keyring`/`imap` langsung.
- Command Tauri tipis; logika (validasi, orkestrasi, jadwal) di `nabungo-app`. Command baru didaftarkan di `collect_commands!`, event di `collect_events!` + `mount_events` (`src-tauri/src/lib.rs`).
- Credential tidak pernah ada di DB, setting, file, log, event, DTO keluaran, atau pesan error.
- Konfigurasi default (email belum dikonfigurasi) = 0 koneksi jaringan dan 0 akses keychain. `scripts/firewall-test.sh` tetap lulus tanpa perubahan.
- Kerja jaringan dan keychain di thread blocking (`spawn_blocking`/thread `email-sync`), tidak pernah di main thread; mutex ledger tidak dipegang selama I/O jaringan.
- Kode error baru: `ErrorCode` (`src-tauri/src/error.rs`) + `errors` di `ui/src/i18n/id.ts` & `en.ts`. Bindings di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- UI mengikuti `docs/design/design.md`; pola kartu mengikuti `components/settings/SecuritySection.vue`. Semua panggilan lewat `ui/src/ipc/`.
- Installer deb/rpm/msi/setup.exe < 15 MB.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`.

## Review Focus

1. **App Password ditempel dengan spasi** (`abcd efgh ijkl mnop`) → login berhasil, yang tersimpan tanpa spasi. Test: `app_password_whitespace_is_stripped` (Task 3).
2. **App dikunci (auto-lock) atau DB di-restore di tengah sync** → sync berhenti tanpa menulis ke DB yang salah, cursor tidak meloncat; sync berikutnya melanjutkan. Test: `lock_mid_sync_stops_with_locked`, `cursor_changed_mid_sync_is_interrupted` (Task 3).
3. **Secret Service tidak tersedia** (Linux tanpa keyring daemon) → connect gagal dengan `keychain_unavailable`, setting tidak berubah, app tidak crash. Test: `connect_keychain_failure_saves_nothing` (Task 3).
4. **Password dicabut di Google** → sync otomatis berhenti mencoba sampai connect ulang; "Sync sekarang" menampilkan `email_auth_failed`. Test: `auth_failure_pauses_schedule` (Task 3).
5. **Email belum dikonfigurasi** → thread `email-sync` tidak pernah memanggil keychain/connect. Test: `disabled_never_touches_keychain_or_network` (Task 3), plus firewall test (Task 6).

---

### Task 1: Keychain + gerbang ukuran

**Files:**
- Create: `crates/app/src/secret.rs`
- Modify: `crates/app/Cargo.toml` (dependensi keychain per target, K1), `crates/app/src/lib.rs` (`pub mod secret;`), `crates/app/src/ledger.rs` (variant `AppError::KeychainUnavailable`)

**Interfaces:**
- Produces:
  ```rust
  pub trait SecretStore: Send + Sync {
      fn get(&self, account: &str) -> Result<Option<String>, AppError>;   // NoEntry → Ok(None)
      fn set(&self, account: &str, secret: &str) -> Result<(), AppError>;
      fn delete(&self, account: &str) -> Result<(), AppError>;            // NoEntry → Ok(())
  }
  pub const SERVICE: &str = "nabungo.email";
  pub struct KeyringStore;          // KeyringStore::new() -> Result<Self, AppError>; memasang default store OS sekali
  #[derive(Default)] pub struct MemorySecretStore { /* Mutex<HashMap> */ }
  impl MemorySecretStore { pub fn failing() -> Self; }   // semua operasi → KeychainUnavailable (test)
  ```
  Error store selain NoEntry → `AppError::KeychainUnavailable` (log `warn` jenis error saja). `account` di-lowercase oleh pemanggil.

- [ ] **Step 1: Tulis test** di `secret.rs` untuk `MemorySecretStore`: `get_missing_is_none`, `set_get_delete_roundtrip`, `delete_missing_is_ok`, `failing_store_reports_keychain_unavailable`.
- [ ] **Step 2:** `cargo test -p nabungo-app secret::` → FAIL; implementasi; → PASS.
- [ ] **Step 3: Implementasi `KeyringStore`** dengan `keyring_core::Entry::new(SERVICE, account)`. Default store dipasang di `KeyringStore::new()` lewat `std::sync::OnceLock`. Tambah test `#[ignore]` `keyring_roundtrip_real` (set/get/delete entry `nabungo.test`), lalu jalankan manual: `cargo test -p nabungo-app secret::keyring_roundtrip_real -- --ignored`.
  Expected: PASS di Fedora (GNOME Keyring). Entry hilang lagi dari Seahorse setelah test.
- [ ] **Step 4: Gerbang ukuran (K1).** Sementara panggil `KeyringStore::new()` dari kode yang ter-link ke app, lalu `APPIMAGE_EXTRACT_AND_RUN=1 pnpm tauri build --bundles deb,rpm` + `node scripts/check-bundle-size.mjs target/release/bundle`. Bandingkan dengan `main` dan catat selisihnya untuk deskripsi PR. Buka draft PR: job test Rust Windows di `ci.yml` membuktikan `windows-native-keyring-store` ter-build.
  Expected: < 15 MB, selisih ≤ 1 MB, CI Windows hijau. Bila tidak, berhenti dan laporkan. Hapus pemanggilan sementara.
- [ ] **Step 5: Commit** `feat(app): penyimpanan App Password di OS keychain`.

### Task 2: Setting akun email

**Files:**
- Create: `crates/app/src/email_settings.rs`
- Modify: `crates/app/src/lib.rs` (mod + re-export), `crates/app/src/ledger.rs` (variant `EmailAddressInvalid`, `EmailMailboxEmpty`, `EmailNotConfigured`)

**Interfaces:**
- Produces:
  ```rust
  pub const DEFAULT_MAILBOX: &str = "Finance";
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct EmailSettings { pub enabled: bool, pub address: Option<String>, pub mailbox: String, pub last_sync_at: Option<String> }
  /// Trim + lowercase; tepat satu '@' dengan bagian kiri & kanan tidak kosong, tanpa whitespace, ≤ 254 char.
  pub fn normalize_address(input: &str) -> Result<String, AppError>;      // → EmailAddressInvalid
  pub fn normalize_mailbox(input: &str) -> Result<String, AppError>;      // trim; kosong → EmailMailboxEmpty
  impl Ledger {
      pub fn email_settings(&self) -> Result<EmailSettings, AppError>;    // nilai hilang → default (disabled, Finance)
      pub fn save_email_account(&mut self, address: &str, mailbox: &str) -> Result<(), AppError>; // enabled=1; input sudah dinormalisasi
      pub fn disable_email(&mut self) -> Result<(), AppError>;            // enabled=0, sisanya tetap (K5)
      pub fn mark_email_synced(&mut self, at: OffsetDateTime) -> Result<(), AppError>;
  }
  ```

- [ ] **Step 1: Tulis test** di `email_settings.rs`:
  - `defaults_when_unset`: `enabled == false`, `address == None`, `mailbox == "Finance"`.
  - `save_then_read`: `save_email_account("me@gmail.com", "Bank")` → enabled, address & mailbox sesuai.
  - `disable_keeps_address_and_mailbox`.
  - `normalize_address_cases`: `" Me@Gmail.COM "` → `"me@gmail.com"`; `""`, `"me"`, `"a@@b"`, `"@b"`, `"a@"`, `"a b@c"` → `EmailAddressInvalid`.
  - `normalize_mailbox_trims_and_rejects_empty`: `"  Finance "` → `"Finance"`, `"   "` → `EmailMailboxEmpty`. Label bersarang `"Finance/Bank"` diterima apa adanya.
  - `mark_synced_sets_timestamp` (format `format_timestamp`).
- [ ] **Step 2:** `cargo test -p nabungo-app email_settings::` → FAIL; implementasi (satu SQLite transaction per tulis, pola `settings.rs`); → PASS.
- [ ] **Step 3: Commit** `feat(app): setting akun email capture`.

### Task 3: Sync dengan lock per email + orkestrasi + jadwal

**Files:**
- Create: `crates/app/src/email_runner.rs`
- Modify: `crates/app/src/email_sync.rs`, `crates/app/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 (`SecretStore`), Task 2 (`EmailSettings`, `normalize_*`, `save_email_account`, `mark_email_synced`), 9a (`MailSource`, `ingest_email`, `email_cursor`, `email_rules`, `seed_email_rules`).
- Produces (`email_sync.rs`):
  ```rust
  /// Akses ledger bersama; `Err(SyncError::Locked)` bila app terkunci / ledger tidak ada.
  pub trait LedgerAccess {
      fn with_ledger<R>(&self, f: impl FnOnce(&mut Ledger) -> Result<R, AppError>) -> Result<R, SyncError>;
  }
  impl LedgerAccess for std::sync::Mutex<Option<Ledger>> { /* None → Locked; poisoned → into_inner() (pola lifecycle.rs) */ }
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
  pub struct SyncProgress { pub processed: u32, pub new_items: u32, pub unparsed: u32 }
  pub enum SyncError { Source(SourceError), App(AppError), Locked, Interrupted }   // tambah 2 varian
  pub fn sync_mailbox_shared<L: LedgerAccess>(ledger: &L, source: &mut dyn MailSource, mailbox: &str,
      local_offset: UtcOffset, on_batch: &mut dyn FnMut(SyncProgress)) -> Result<SyncSummary, SyncError>;
  ```
  Lock pertama menjalankan `seed_email_rules` + `email_rules` + `email_cursor`. `sync_mailbox(&mut Ledger, …)` tetap dengan signature lama dan mendelegasikan ke `sync_mailbox_shared` lewat adapter privat `LedgerAccess` untuk `RefCell<&mut Ledger>`. Test 9a yang ada harus tetap lulus.
- Produces (`email_runner.rs`):
  ```rust
  pub type Connect<'a> = &'a dyn Fn(&str /*address*/, &str /*password*/) -> Result<Box<dyn MailSource>, SourceError>;
  pub fn normalize_app_password(input: &str) -> String;                    // buang semua whitespace (K4)
  /// K3: verifikasi login+label, lalu keychain, lalu setting. Tidak menyentuh ledger saat I/O.
  pub fn connect_email<L: LedgerAccess>(ledger: &L, secrets: &dyn SecretStore, connect: Connect,
      address: &str, mailbox: &str, password: Option<&str>) -> Result<(), SyncError>;
  pub fn disconnect_email<L: LedgerAccess>(ledger: &L, secrets: &dyn SecretStore) -> Result<(), SyncError>;
  /// Satu putaran: setting (disabled → App(EmailNotConfigured)) → password (tidak ada → App(EmailNotConfigured))
  /// → connect → sync_mailbox_shared → mark_email_synced.
  pub fn run_email_sync<L: LedgerAccess>(ledger: &L, secrets: &dyn SecretStore, connect: Connect,
      local_offset: UtcOffset, on_batch: &mut dyn FnMut(SyncProgress)) -> Result<SyncSummary, SyncError>;
  pub const SYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);
  #[derive(Debug, Default)]
  pub struct SyncSchedule { last_attempt: Option<Instant>, pub paused: bool }
  impl SyncSchedule {
      pub fn due(&self, now: Instant) -> bool;              // !paused && (None || now - last >= SYNC_INTERVAL)
      pub fn record(&mut self, now: Instant, result: &Result<SyncSummary, SyncError>); // Source(Auth) → paused = true
      pub fn reset(&mut self);                              // last_attempt = None, paused = false (unlock/restore/connect)
  }
  ```
  Saat connect, `Source(Auth)` dari langkah verifikasi dikembalikan apa adanya; keychain dan setting tidak berubah.

- [ ] **Step 1: Tulis test** di `email_sync.rs`, memakai `FakeSource` yang sudah ada dan `Mutex<Option<Ledger>>`:
  - `shared_sync_matches_direct`: 3 pesan → ringkasan sama dengan `first_sync_ingests_all`; `on_batch` dipanggil sekali dengan `processed == 3`.
  - `lock_mid_sync_stops_with_locked`: `FakeSource` dengan hook yang meng-`take()` ledger setelah batch pertama (120 pesan) → `Err(Locked)`; pasang kembali ledger yang sama, sync ulang → total 120 capture tanpa dobel.
  - `cursor_changed_mid_sync_is_interrupted`: hook mengganti ledger dengan `Ledger::open_in_memory()` baru setelah batch pertama → `Err(Interrupted)`, ledger baru tidak punya capture maupun cursor.
- [ ] **Step 2: Tulis test** di `email_runner.rs` (fake `Connect` yang mencatat jumlah panggilan; `MemorySecretStore`):
  - `connect_verifies_then_saves`: sukses → keychain berisi password, setting enabled.
  - `app_password_whitespace_is_stripped`: `"abcd efgh ijkl mnop"` → fake connect menerima `"abcdefghijklmnop"`, keychain menyimpan yang sama.
  - `connect_auth_failure_saves_nothing`: fake → `Auth` → `Err(Source(Auth))`, keychain kosong, setting tetap disabled.
  - `connect_keychain_failure_saves_nothing`: `MemorySecretStore::failing()` → `Err(App(KeychainUnavailable))`, setting tetap disabled.
  - `connect_without_password_reuses_stored`: sudah terhubung, `password: None`, label baru → fake connect menerima password lama, label terbarui. Belum pernah terhubung + `None` → `EmailNotConfigured`.
  - `changing_address_deletes_old_secret`.
  - `disconnect_removes_secret_keeps_data`: capture yang ada tetap ada, `enabled == false`.
  - `disabled_never_touches_keychain_or_network`: `run_email_sync` dengan setting default → `EmailNotConfigured`, panggilan connect = 0, store `failing()` tidak dipanggil (bukan `KeychainUnavailable`).
  - `run_sync_marks_synced_on_success_only`: sukses → `last_sync_at` terisi; fake `Connection` → tetap `None`.
  - `auth_failure_pauses_schedule`: `record(Err(Source(Auth)))` → `due` false selamanya sampai `reset`; `record(Err(Source(Connection)))` → `due` lagi setelah `SYNC_INTERVAL`.
  - `schedule_due_rules`: baru dibuat → due; setelah record sukses → tidak due sebelum 15 menit, due tepat 15 menit.
- [ ] **Step 3:** `cargo test -p nabungo-app email_` → FAIL; implementasi; → PASS (termasuk test 9a di `email_sync.rs`).
- [ ] **Step 4:** CLI `capture_sync` tidak berubah (tetap `sync_mailbox`); pastikan `cargo build -p nabungo-app --examples` lulus.
- [ ] **Step 5: Commit** `feat(app): sync email tanpa menahan lock ledger + jadwal`.

### Task 4: Command, event, dan thread `email-sync`

**Files:**
- Create: `src-tauri/src/email.rs`
- Modify: `src-tauri/src/state.rs` (`EmailRuntime` di `AppState`), `src-tauri/src/lib.rs` (offset di awal `run()`, `mod email`, command + `collect_events!` + `mount_events`), `src-tauri/src/lifecycle.rs` (`open_unlocked`/`on_restored` memanggil `email::on_ledger_opened`), `src-tauri/src/commands.rs`, `src-tauri/src/dto.rs`, `src-tauri/src/error.rs`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `ui/src/ipc/bindings.ts` (generate)

**Interfaces:**
- Consumes: Task 3 (`connect_email`, `disconnect_email`, `run_email_sync`, `SyncSchedule`, `SyncProgress`, `LedgerAccess` untuk `Mutex<Option<Ledger>>`), Task 2 (`email_settings`), Task 1 (`KeyringStore`).
- Produces:
  ```rust
  // state.rs
  pub struct EmailRuntime {
      pub running: AtomicBool, pub schedule: Mutex<SyncSchedule>, pub last_error: Mutex<Option<ErrorCode>>,
      pub local_offset: UtcOffset, pub thread_started: AtomicBool,
  }
  // email.rs
  pub fn run_sync(app: &AppHandle) -> Result<SyncSummary, IpcError>;   // K9 guard, emit progress+done, schedule.record, last_error (sukses → None)
  pub fn on_ledger_opened(app: &AppHandle);                            // schedule.reset() + spawn_email_scheduler sekali
  fn spawn_email_scheduler(app: AppHandle);                            // K7: poll 30 s
  // dto.rs
  pub struct EmailStatusDto { enabled: bool, address: Option<String>, mailbox: String, last_sync_at: Option<String>, running: bool, last_error: Option<ErrorCode>, paused: bool }
  pub struct SyncSummaryDto { new_items: u32, unparsed: u32, seen: u32 }
  #[derive(Serialize, Deserialize, Clone, specta::Type, tauri_specta::Event)] pub struct EmailSyncProgress { processed: u32, new_items: u32, unparsed: u32 }
  #[derive(.., tauri_specta::Event)] pub struct EmailSyncDone { summary: Option<SyncSummaryDto>, error: Option<IpcError> }
  // commands.rs (semua async + blocking kecuali email_status)
  email_status() -> EmailStatusDto
  connect_email(address: String, mailbox: String, password: Option<String>) -> ()   // sukses → schedule.reset() + last_error None: thread sync dalam ≤ 30 s
  disconnect_email() -> ()
  sync_email() -> SyncSummaryDto
  ```
  Pemetaan `SyncError → IpcError` di `error.rs`: `Source(Auth)` → `email_auth_failed`, `Source(Mailbox)` → `email_mailbox_not_found`, `Source(Connection)` → `email_connection` (log `warn`), `Locked` → `locked`, `Interrupted` → `email_sync_interrupted`, `App(e)` → pemetaan `AppError` yang ada. Varian `AppError` baru: `KeychainUnavailable` → `keychain_unavailable`, `EmailAddressInvalid` → `email_address_invalid`, `EmailMailboxEmpty` → `email_mailbox_empty`, `EmailNotConfigured` → `email_not_configured`. Running → `email_sync_running`.
  Semua command email memanggil `ensure_not_recovering` lebih dulu.

- [ ] **Step 1: Tulis test** di `error.rs`: `sync_errors_map_to_codes` (enam varian di atas) dan `email_app_errors_map_to_codes`. Di `email.rs`, logika thread murni diekstrak ke `fn tick_should_run(enabled: bool, unlocked: bool, recovering: bool, running: bool, due: bool) -> bool` + test `tick_skips_when_locked_recovering_disabled_or_running`.
- [ ] **Step 2:** `cargo test -p nabungo-desktop` → FAIL; implementasi; → PASS. `KeyringStore::new()` dibuat lazily di dalam `run_sync`/`connect_email`, tidak saat startup (K7). Kirim event lewat `EmailSyncProgress(..).emit(&app)`; gagal emit hanya `warn`.
- [ ] **Step 3: Teks i18n** (`errors.*`, id / en):
  - `email_auth_failed`: "Login Gmail gagal. Periksa alamat email dan App Password." / "Gmail sign-in failed. Check the email address and App Password."
  - `email_mailbox_not_found`: "Label tidak ditemukan di Gmail." / "Label not found in Gmail."
  - `email_connection`: "Tidak bisa terhubung ke Gmail. Periksa koneksi internet." / "Could not connect to Gmail. Check your internet connection."
  - `email_sync_running`: "Sync email sedang berjalan." / "Email sync is already running."
  - `email_sync_interrupted`: "Sync dihentikan karena data berubah. Akan dilanjutkan di sync berikutnya." / "Sync stopped because the data changed. It will resume on the next sync."
  - `email_not_configured`: "Email belum dihubungkan." / "Email is not connected."
  - `email_address_invalid`: "Alamat email tidak valid." / "Invalid email address."
  - `email_mailbox_empty`: "Label wajib diisi." / "Label is required."
  - `keychain_unavailable`: "Keychain sistem tidak tersedia, App Password tidak bisa disimpan." / "System keychain unavailable; the App Password cannot be saved."
- [ ] **Step 4:** `cargo test -p nabungo-desktop export_bindings` lalu `pnpm --filter @nabungo/ui typecheck` → PASS (bindings memuat 4 command + `events.emailSyncProgress`/`emailSyncDone`).
- [ ] **Step 5: Uji manual singkat** `pnpm dev` dengan data dir salinan (`XDG_DATA_HOME` sementara): app start normal, dan log tidak berisi baris sync email selama belum dikonfigurasi. Uji end-to-end lewat UI ada di Task 6.
- [ ] **Step 6: Commit** `feat(desktop): command & penjadwal sync email`.

### Task 5: Kartu Email di Settings

**Files:**
- Create: `ui/src/components/settings/EmailSection.vue`, `ui/src/components/settings/EmailSection.test.ts`, `ui/src/ipc/browser-email.ts`, `ui/src/ipc/browser-email.test.ts`
- Modify: `ui/src/ipc/index.ts` (facade: `emailStatus`, `connectEmail`, `disconnectEmail`, `syncEmail`, `onEmailSync`), `ui/src/ipc/browser.ts` (gabung `browserEmail`; ikut `lockGuard`), `ui/src/views/SettingsView.vue` (sisipkan setelah `SecuritySection`), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`

**Interfaces:**
- Consumes: Task 4 (bindings `EmailStatusDto`, `SyncSummaryDto`, event `EmailSyncProgress`/`EmailSyncDone`).
- Produces:
  ```ts
  // ipc/index.ts
  export function onEmailSync(h: { progress?: (p: EmailSyncProgress) => void; done?: (d: EmailSyncDone) => void }): Promise<() => void>
  // Tauri: events.emailSyncProgress.listen / emailSyncDone.listen; browser: emitter lokal di browser-email.ts
  ```
  Komponen tidak memanggil `events` atau `commands` langsung.

UI (ikuti `design.md` + pola `SecuritySection`):
- **Belum terhubung:** judul "Email capture", deskripsi singkat (opt-in, hanya membaca label, password di keychain sistem), dan 3 langkah setup teks biasa (2-Step Verification → buat App Password → filter label). Tidak ada tautan atau aset remote. Field: Email, App Password (`type=password`, `autocomplete=off`), Label (default `Finance`). Tombol "Hubungkan" menampilkan spinner selama verifikasi.
- **Terhubung:** alamat, label, "Terakhir sync: <relatif>" atau "Belum pernah". Bila `paused`, badge "Dijeda: login gagal". Error terakhir ditampilkan lewat `errors.<code>`. Selama sync berjalan, teks "Menyinkronkan… N email". Tombol: "Sync sekarang" (disabled saat running), "Ubah" (form terisi, password kosong = tidak diganti), "Putuskan" (ConfirmDialog: "Data yang sudah tersinkron tetap disimpan.").
- `done` dengan `summary` → toast "N transaksi baru, M gagal dibaca". Teks review queue: "Tinjau di Inbox (segera hadir)". Link Inbox ditambahkan di 9c.
- Password field dikosongkan setelah submit, berhasil maupun gagal.

- [ ] **Step 1: Tulis test** `browser-email.test.ts` (aturan K14): validasi alamat/label sama dengan `normalize_address`/`normalize_mailbox` (kasus Task 2); `wrong` → `email_auth_failed`; `Missing` → `email_mailbox_not_found`; `syncEmail` saat belum terhubung → `email_not_configured`; sukses → `done` terpancar dengan ringkasan nol; `disconnectEmail` mempertahankan alamat.
- [ ] **Step 2: Tulis test** `EmailSection.test.ts`:
  - `shows setup form when disconnected`;
  - `connect success switches to connected state and clears password`;
  - `connect auth error shows translated message and keeps form`;
  - `sync now disabled while running and re-enabled on done`;
  - `paused badge shown when status.paused`;
  - `disconnect asks confirmation then shows setup form with address prefilled`.
- [ ] **Step 3:** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-email.test.ts src/components/settings/EmailSection.test.ts` → FAIL; implementasi; → PASS.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test` → PASS. Cek visual di `pnpm --filter @nabungo/ui dev` (terang + gelap, lebar sempit).
- [ ] **Step 5: Commit** `feat(ui): kartu email capture di Settings`.

### Task 6: Uji end-to-end, firewall, dokumentasi

**Files:**
- Modify: `CLAUDE.md` (bullet email di §Arsitektur `crates/app` & `src-tauri`; keychain), `docs/plans/2026-10-12-v1-roadmap.md` (baris 9b: link plan + status), `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` (status milestone 4; open question interval → 15 menit)

- [ ] **Step 1: Uji manual end-to-end** `pnpm dev` dengan folder data salinan:
  - Hubungkan memakai App Password dengan spasi.
  - Pastikan entry `nabungo.email` muncul di Seahorse.
  - Pastikan event `done` tiba dan `email_capture` terisi: `sqlite3 <dir>/nabungo.db "select status,count(*) from email_capture group by 1"`.
  - "Sync sekarang" kedua kalinya → 0 baru.
  - Kunci app (DB terenkripsi) saat sync berjalan → tidak ada error selain `locked`.
  - Putuskan → entry keychain hilang, capture tetap ada.
  - Email di Gmail tetap belum dibaca.
  - `grep -ri "<alamat email>\|<app password>" <dir>/logs` → kosong.
- [ ] **Step 2: Firewall test** dengan build release: `cargo build --release -p nabungo-desktop && cargo build --release -p nabungo-app --example seed_fixture && scripts/firewall-test.sh target/release/nabungo-desktop target/release/examples/seed_fixture`.
  Expected: exit 0 (0 paket); `FW_SELFTEST=1` → exit 1.
- [ ] **Step 3: Update dokumen** di atas. Tambahkan di `CLAUDE.md` §Model data, bullet email capture: "App Password hanya di OS keychain (`nabungo_app::secret`, service `nabungo.email`); setting `email.*` hanya alamat/label/status. Sync (`email_runner`) mengunci ledger per email, tidak selama fetch; thread `email-sync` (15 menit) tidak menyentuh keychain/jaringan selama `email.enabled != 1`."
- [ ] **Step 4: Verifikasi penuh**: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && node scripts/check-bundle-size.mjs target/release/bundle`.
  Expected: semua lulus, installer < 15 MB, `git diff --exit-code ui/src/ipc/bindings.ts` bersih.
- [ ] **Step 5: Commit** `docs: fase 9b email capture integrasi tauri`.
