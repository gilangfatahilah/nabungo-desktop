# Fase 9a — Email Capture: Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EML-01 sampai EML-03 di sisi backend, tanpa UI. Email notifikasi BCA dan Bank Jago diambil lewat IMAP, di-parse rule engine lokal, lalu masuk review queue (`email_capture`) atau log `email_unparsed`. Review queue bisa di-confirm, di-link ke transaksi yang sudah ada, atau di-ignore. Semua bisa dijalankan dari CLI example dan dites tanpa jaringan.

**Architecture:**
- Crate baru `crates/mail` (`nabungo-mail`):
  - `message.rs` untuk raw RFC 822 → `MailMessage` (mail-parser + html2text);
  - `rule.rs` untuk rule engine murni (regex → `Captured`);
  - `defaults.rs` untuk rule bawaan BCA & Jago;
  - `source.rs` untuk trait `MailSource` + implementasi IMAP.
  Arah dependensi: `nabungo-app → nabungo-mail → nabungo-core`. Core tetap tanpa regex/IO.
- `nabungo-storage`: migrasi `0011_email_capture.sql` + repository `email.rs`.
- `nabungo-app`: `capture.rs` untuk use case review queue (blok `impl Ledger`), `email_sync.rs` untuk loop sync di atas `MailSource`.
- `src-tauri`: hanya pemetaan error baru. Command, keychain, dan scheduler masuk 9b.
- Example `capture_sync` (CLI) menjalankan sync sungguhan ke folder data; credential dari env var. Example ini satu-satunya kode yang membuka jaringan di 9a.

**Tech Stack:** Rust; dependensi baru hanya di `nabungo-mail`: `imap` 2.4.1 (fitur `tls` → `native-tls`: OpenSSL di Linux yang sudah dibundel untuk SQLCipher, SChannel di Windows), `mail-parser` 1.0, `html2text` 0.17, `regex` 1, `sha2` 0.10.

**Spec:**
- `PRD.md`:
  - EML-01–EML-03 (§5) dan D13 (§3).
  - Acceptance criteria "Capture idempoten" dan "Tanpa jaringan secara default" (§5).
  - Skema email capture (§6).
  - Baris Offline/Privasi (§8) dan risiko email (§10).
- Brief: `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` (alur sync, rule engine, risiko).

**Rencana sub-fase:**
- **9a (plan ini):** backend + CLI.
- **9b:** command Tauri, keychain (`keyring`), setting akun/label, sync saat unlock + berkala, event progress.
- **9c:** layar Inbox (Pending/Unparsed), duplicate hint, mode browser, lalu rule editor (EML-04).

**Di luar plan ini:**
- UI dan command Tauri.
- Keychain; CLI membaca `NABUNGO_IMAP_USER`/`NABUNGO_IMAP_PASSWORD`.
- Rule editor dan "buat rule dari unparsed".
- Re-parse unparsed setelah rule berubah.
- Multi-akun email.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Crate terpisah `nabungo-mail`**, bukan di core/app. Isinya parsing murni + `MailSource`. Core tidak mendapat dependensi `regex`. | Core harus kecil & bebas IO; dependensi jaringan terisolasi satu crate. |
| K2 | **`imap` 2.4.1 sync + `native-tls`**, dipanggil dari thread (9b: `spawn_blocking`). Task 1 adalah gerbang: bila build Windows CI gagal atau ukuran installer naik > 1,5 MB, pindah ke alternatif (`imap` 3.0 alpha atau `async-imap`) sebelum Task 2. | D2 tanpa runtime async; native-tls tidak menambah TLS stack kedua. Batas installer < 15 MB. |
| K3 | **Kunci idempotency** = `(message_id, idx)`. `message_id` = header `Message-ID` tanpa `<>`; bila tidak ada, `sha256:<hex>` dari raw bytes. `idx` = urutan match `amount_regex` di body (0, 1, …). | Satu email bisa berisi beberapa transaksi (risiko di brief). Email tanpa Message-ID tetap stabil. |
| K4 | **Cursor per email**: `ingest_email` menulis capture/unparsed **dan** `last_uid` dalam satu SQLite transaction. UIDVALIDITY berbeda → `last_uid = 0` dengan validity baru, lalu rescan; dedup K3 mencegah dobel. | Koneksi putus tidak melewatkan atau mendobel email. |
| K5 | **Tanggal**: `date_regex` (grup 1) + `date_format` ∈ {`dd/mm/yyyy`, `dd-mm-yyyy`, `dd mmm yyyy`}; `mmm` menerima singkatan bulan Indonesia & Inggris (`Okt`/`Oct`, `Agu`/`Aug`, `Des`/`Dec`, `Mei`/`May`), tanpa beda huruf besar/kecil. Bila tidak ada atau gagal: header `Date` dikonversi ke offset yang diberikan pemanggil (`local_offset`), lalu diambil tanggalnya. | Brief: waktu dari body; fallback agar email tetap masuk queue. |
| K6 | **Nominal**: grup 1 dari `amount_regex`, buang semua karakter selain `0-9.,`, lalu `nabungo_core::amount::parse_amount`. Hasil ≤ 0 atau error → email itu masuk unparsed (seluruh email, bukan per match). | Parser format Indonesia sudah ada dan teruji (`1.250.000,00` → 1250000, `25.000,50` → `Fractional`). |
| K7 | **Pemilihan rule**: rule `enabled` dan tidak dihapus, `sender_pattern` = substring alamat From (tanpa beda huruf besar/kecil), urut `priority` DESC lalu `name`. Rule pertama yang cocok `subject_regex` (bila ada) **dan** punya ≥1 match `amount_regex` dipakai. Regex invalid membuat rule dilewati (log `warn`), bukan menggagalkan sync. | Brief "rule engine". Rule rusak tidak boleh menghentikan semua capture. |
| K8 | **Rule bawaan** (`defaults::default_rules()`): BCA (debit, kredit) dan Jago (keluar, masuk), dengan `account_id`/`category_id` NULL. Rule disalin ke DB oleh `Ledger::seed_email_rules()` sekali saja, ditandai setting `email.rules_seeded = 1`; user yang menghapus rule bawaan tidak mendapatkannya kembali. Pola regex awal dibuat dari fixture **sintetis**; Task 3 punya langkah mengganti fixture dengan contoh asli tersensor saat tersedia. | Pilihan user: seed dulu, editor nanti. Belum ada contoh email asli. |
| K9 | **Draft review**: `capture_draft(id)` mengisi `TxnForm` sebisanya: kind dari rule; akun = `rule.account_id` → `payee_suggestion(payee).account_id` → `None`; kategori = `rule.default_category_id` → `payee_suggestion` → `None`. Field kosong diisi user di 9c. | Reuse `Ledger::payee_suggestion` sebagai auto-kategori. |
| K10 | **Confirm** = `write_txn` (jalur `create_txn`) + `status='confirmed'`, `txn_id` diisi, dalam satu SQLite transaction. **Link** = tandai confirmed dengan `txn_id` transaksi yang sudah ada (merge dari duplicate hint), tanpa transaksi baru. **Ignore** = `status='ignored'`. Ketiganya mensyaratkan status `pending`; selain itu `AppError::CaptureNotPending` (kode `capture_not_pending`). | Klik ganda tidak mendobel transaksi. |
| K11 | **Duplicate candidates**: transaksi aktif berjenis sama dengan nominal (absolut posting akun) = nominal capture, tanggal ±1 hari, dan bila akun draft diketahui hanya di akun itu. Urut berdasarkan selisih tanggal, maksimal 5. | Brief: "nominal sama ±1 hari". UI di 9c. |
| K12 | **Kind rule** di 9a hanya `income`/`expense` (CHECK di migrasi tetap mengizinkan `transfer` untuk 9c). | Transfer butuh akun tujuan; review UI belum ada. |
| K13 | **Snippet**: `raw_snippet` = teks body hasil normalisasi, dipotong 4000 karakter (batas char, bukan byte). Header lengkap & lampiran tidak disimpan. | Cukup untuk debug & membuat rule; DB tetap kecil. Isi terenkripsi bila SQLCipher aktif. |

### Revisi setelah sampel email asli (27 Okt 2026)

Sampel body asli BCA (myBCA) & Jago, plus spike ke Gmail user, mengubah beberapa asumsi di atas. Revisi ini mengalahkan baris K yang dirujuk.

| # | Revisi | Alasan |
|---|---|---|
| R1 | **Normalisasi nominal** (ganti K6): `normalize_amount` di `nabungo-mail` menerima format Indonesia (`Rp 21.000`, `Rp500`, `25.000,50`) **dan** Inggris (`IDR 41,700.00`). Bila `.` dan `,` sama-sama ada, pemisah terakhir adalah desimal; bila hanya ada satu `.` yang tidak diikuti tepat 3 digit, `.` adalah desimal. Hasilnya diubah ke format Indonesia lalu `parse_amount`. Desimal bukan nol → unparsed; `1,500` (ambigu) dibaca sebagai desimal → unparsed, tidak pernah salah nominal. | BCA memakai format Inggris; Jago format Indonesia. |
| R2 | **Kind rule** (ganti K12): `income`, `expense`, `transfer_out` (dari `account_id` ke `counter_account_id`), `transfer_in` (dari `counter_account_id` ke `account_id`). Kolom baru `email_rule.counter_account_id` (FK `account`, nullable); PRD §6 diperbarui. | Sampel berisi tarik tunai BCA → Cash dan terima uang GoPay → Jago. |
| R3 | **`payee_regex`** dicoba di body, lalu di subject. | Jago menaruh merchant hanya di subject ("Kamu telah membayar ke X💸"). |
| R4 | **Tanggal** (perluas K5): `date_format` `dd mmm yyyy` boleh diikuti jam (`09 May 2026 11:14:28`, `15 May 2026, 22:25 WIB`); hanya prefiks tanggal yang dibaca. Jago debit/terima/refund tanpa tanggal di body → fallback header `Date`. | Format asli. |
| R5 | **Rule bawaan** (ganti K8 fixture sintetis): BCA QRIS, transfer, VA, tarik tunai; Jago bayar, kartu debit, terima GoPay, refund. `amount_regex` di-anchor ke label (`Total Payment`, `Transfer Amount`, `Jumlah` di baris sendiri, dst.) agar nominal yang diulang (VA: `billdesc`/`Bill Total`/`Total Payment`) tidak jadi beberapa item. Fixture dibuat dari sampel asli yang disensor (nama, nomor rekening, referensi diganti). | Sampel asli tersedia. |
| R6 | **Credential CLI**: `NABUNGO_IMAP_USER` + `GOOGLE_APP_PASSWORD` (nama variabel di `.env` user); label default tetap `Finance` (spike: label user saat ini `notion-synced`). | Menyesuaikan setup user. |

## Global Constraints

- Arah dependensi: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage, nabungo-mail)`, `nabungo-mail → nabungo-core`, `nabungo-storage → nabungo-core`. `nabungo-core` tidak mendapat dependensi baru.
- Migrasi `0001`–`0010` tidak diubah. Migrasi baru `0011_email_capture.sql` didaftarkan di `migrations()` (`crates/storage/src/lib.rs`).
- Konvensi skema: ID UUIDv7 TEXT, tanggal `YYYY-MM-DD`, nominal `INTEGER` satuan minor (positif) + `currency`, timestamp `now_utc()`, tabel `STRICT`.
- Repository tulis menerima `&Transaction`; baca boleh `&Connection`. Satu use case tulis = satu SQLite transaction.
- Transaksi hanya lewat `write_txn` (`crates/app/src/transactions.rs:42`); invariant oleh `ValidTxn::new`.
- Tidak ada credential di DB, setting, file, atau log. CLI tidak mencetak password.
- Aplikasi Tauri tidak memanggil kode jaringan di 9a; firewall test tidak berubah.
- Kode error baru masuk `ErrorCode` (`src-tauri/src/error.rs`) dan `errors` di `ui/src/i18n/id.ts` + `en.ts`; bindings di-generate ulang dan di-commit.
- Installer deb/rpm/msi/setup.exe < 15 MB (`node scripts/check-bundle-size.mjs`).
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`.

## Review Focus

1. **Sync diulang setelah koneksi putus di tengah batch** → tidak ada capture dobel atau email terlewat. Test: `sync_resumes_after_source_error` (Task 6).
2. **UIDVALIDITY berubah** (label dibuat ulang) → rescan tanpa dobel, capture yang di-ignore tetap ignored. Test: `uidvalidity_change_rescans_without_duplicates` (Task 6).
3. **Email HTML-only dengan tabel dan `&nbsp;`/charset non-UTF-8** → nominal tetap terbaca. Test: `html_only_body_is_converted` (Task 2).
4. **Nominal dengan desimal bukan nol** (`Rp 25.000,50`) → masuk unparsed, bukan dibulatkan diam-diam. Test: `fractional_amount_goes_unparsed` (Task 3).
5. **Confirm dua kali / confirm setelah ignore** → `CaptureNotPending`, satu transaksi saja. Test: `confirm_twice_is_rejected` (Task 5).

---

### Task 1: Spike IMAP + gerbang ukuran

**Files:**
- Create: `crates/mail/Cargo.toml`, `crates/mail/src/lib.rs`, `crates/mail/examples/imap_spike.rs`
- Modify: `Cargo.toml` (workspace `members` + `crates/mail`), `crates/app/Cargo.toml` (dependensi `nabungo-mail`)

**Interfaces:**
- Produces: crate `nabungo-mail` (lints workspace, versi workspace) yang ter-link ke `nabungo-app`.

- [ ] **Step 1: Buat crate** dengan dependensi `imap = "2.4.1"` (default `tls`), `native-tls = "0.2"`, `nabungo-core` (path). `lib.rs` hanya berisi doc comment modul.
- [ ] **Step 2: Tulis `examples/imap_spike.rs`**: baca `NABUNGO_IMAP_USER`, `NABUNGO_IMAP_PASSWORD`, `NABUNGO_IMAP_MAILBOX` (default `Finance`); connect `imap.gmail.com:993` TLS; `examine` (read-only) mailbox; cetak `UIDVALIDITY`, lalu `uid_fetch` 10 UID terakhir dengan `BODY.PEEK[HEADER.FIELDS (SUBJECT FROM DATE MESSAGE-ID)]` dan cetak header-nya; `logout`.
- [ ] **Step 3: Jalankan manual** dengan App Password user: `cargo run -p nabungo-mail --example imap_spike`.
  Expected: 10 subject tercetak. Cek di Gmail bahwa email-nya tetap belum dibaca.
- [ ] **Step 4: Ukur ukuran installer**. Agar dependensi tidak dibuang sebagai dead code, sementara panggil `imap::ClientBuilder` dari fungsi `pub` yang dipakai `nabungo-app`. Lalu `APPIMAGE_EXTRACT_AND_RUN=1 pnpm tauri build --bundles deb,rpm` dan `node scripts/check-bundle-size.mjs target/release/bundle`; bandingkan dengan build `main`. Catat selisih di deskripsi PR.
  Expected: semua < 15 MB dan selisih ≤ 1,5 MB (K2). Bila tidak, berhenti dan laporkan ke user sebelum Task 2. Hapus pemanggilan sementara setelah diukur.
- [ ] **Step 5: Commit** `feat(mail): crate nabungo-mail + spike IMAP`.

### Task 2: Parse pesan email

**Files:**
- Create: `crates/mail/src/message.rs`, `crates/mail/tests/fixtures/` (lihat Task 3)
- Modify: `crates/mail/Cargo.toml` (`mail-parser = "1.0"` dengan fitur `full_encoding`, `html2text = "0.17"`, `sha2 = "0.10"`, `time` sama dengan core)

**Interfaces:**
- Produces:
  ```rust
  pub struct MailMessage {
      pub message_id: String,          // tanpa <>, atau "sha256:<hex>"
      pub from: String,                // alamat saja, lowercase
      pub subject: String,             // "" bila tidak ada
      pub sent_at: Option<time::OffsetDateTime>,
      pub text: String,                // body teks ternormalisasi
  }
  pub fn parse_message(raw: &[u8]) -> Option<MailMessage>; // None = bukan email yang bisa dibaca
  pub const SNIPPET_CHARS: usize = 4000;
  pub fn snippet(text: &str) -> String;
  ```

- [ ] **Step 1: Tulis test** di `message.rs`:
  - `plain_text_body`: raw dengan `Message-ID: <a@b>` → `message_id == "a@b"`, `from` lowercase, `text` berisi baris body.
  - `html_only_body_is_converted`: `text/html` ber-charset `iso-8859-1` dengan `<table><tr><td>Nominal</td><td>Rp&nbsp;150.000,00</td></tr></table>` → `text` mengandung `Rp 150.000,00` (spasi biasa).
  - `missing_message_id_uses_hash`: tanpa header → `message_id` diawali `sha256:` dan stabil untuk raw yang sama.
  - `whitespace_is_collapsed`: `\u{a0}`, tab, dan spasi berulang → satu spasi; baris kosong beruntun → satu.
  - `snippet_truncates_on_char_boundary`: 4001 karakter `é` → panjang `chars()` 4000.
- [ ] **Step 2: `cargo test -p nabungo-mail message::`** → FAIL (belum ada).
- [ ] **Step 3: Implementasi.** Pilih `body_text(0)`. Bila hanya ada HTML, pakai `html2text::from_read(html, 200)`. Normalisasi whitespace setelahnya.
- [ ] **Step 4: Test** → PASS.
- [ ] **Step 5: Commit** `feat(mail): parse pesan email ke teks`.

### Task 3: Rule engine + rule bawaan BCA & Jago

**Files:**
- Create: `crates/mail/src/rule.rs`, `crates/mail/src/defaults.rs`, `crates/mail/tests/fixtures/{bca_debit,bca_kredit,jago_keluar,jago_masuk,bca_fractional,unknown_sender}.eml`, `crates/mail/tests/rules.rs`
- Modify: `crates/mail/Cargo.toml` (`regex = "1"`)

**Interfaces:**
- Consumes: `MailMessage` (Task 2), `nabungo_core::amount::parse_amount`.
- Produces:
  ```rust
  pub enum RuleKind { Income, Expense }
  pub struct Rule {
      pub id: Option<uuid::Uuid>,       // None untuk rule bawaan sebelum disimpan
      pub name: String, pub sender_pattern: String,
      pub subject_regex: Option<String>, pub amount_regex: String,
      pub payee_regex: Option<String>, pub date_regex: Option<String>,
      pub date_format: Option<String>, pub kind: RuleKind,
      pub account_id: Option<uuid::Uuid>, pub default_category_id: Option<uuid::Uuid>,
      pub priority: i64,
  }
  pub struct Captured { pub idx: u32, pub amount: i64, pub payee: Option<String>, pub date: time::Date }
  pub enum Outcome { Parsed { rule: usize /* index ke slice rules */, items: Vec<Captured> }, Unparsed }
  pub fn apply_rules(rules: &[Rule], msg: &MailMessage, local_offset: time::UtcOffset) -> Outcome;
  pub fn default_rules() -> Vec<Rule>; // di defaults.rs
  ```
  `rules` sudah terurut oleh pemanggil (K7). `apply_rules` meng-compile regex sekali per panggilan; pemanggil yang memproses banyak email memakai `CompiledRules::new(&[Rule]) -> CompiledRules` + `CompiledRules::apply(&self, &MailMessage, UtcOffset) -> Outcome` (`apply_rules` hanya pembungkus).

- [ ] **Step 1: Buat fixture sintetis.** Isi `.eml` meniru format notifikasi BCA (KlikBCA/myBCA, pengirim `@bca.co.id` dan `@klikbca.com`) dan Jago (`@jago.com`): nominal `Rp 150.000,00` / `IDR 2.500.000`, nama merchant/penerima, tanggal `27/10/2026` atau `27 Okt 2026`. `bca_fractional.eml` bernominal `Rp 25.000,50`. Tambah `fixtures/README.md`: "Sintetis. Ganti dengan contoh asli tersensor (nomor rekening/nama disamarkan) saat tersedia; jangan commit email tanpa sensor."
- [ ] **Step 2: Tulis test** di `tests/rules.rs` (memakai `parse_message` + `default_rules()` + offset `+07:00`):
  - `bca_debit_is_expense`: `Parsed`, 1 item, `amount == 150_000`, `payee == Some("TOKO ABC")`, `date == 2026-10-27`, rule kind `Expense`.
  - `bca_kredit_is_income`, `jago_keluar_is_expense`, `jago_masuk_is_income` dengan nilai dari fixture masing-masing.
  - `fractional_amount_goes_unparsed`: `bca_fractional.eml` → `Unparsed`.
  - `unknown_sender_goes_unparsed`.
  - `multiple_amounts_get_indexes`: email sintetis dengan dua baris transaksi → `idx` 0 dan 1.
  - `date_falls_back_to_header`: rule tanpa `date_regex`, header `Date: Mon, 26 Oct 2026 20:30:00 +0000` dengan offset `+07:00` → `2026-10-27`.
  - `invalid_regex_rule_is_skipped`: rule priority tertinggi ber-regex `(` dilewati, rule berikutnya dipakai.
  - `subject_regex_must_match`.
  - `month_names_id_and_en`: `27 Okt 2026`, `27 Oct 2026`, `3 Mei 2026` terbaca.
- [ ] **Step 3: Jalankan** `cargo test -p nabungo-mail --test rules` → FAIL.
- [ ] **Step 4: Implementasi `rule.rs` dan `defaults.rs`** sesuai K5–K7. Regex `(?i)`; nominal dari grup 1. Payee di-trim; payee kosong → `None`.
- [ ] **Step 5: Test** → PASS. `cargo clippy -p nabungo-mail --all-targets -- -D warnings`.
- [ ] **Step 6: Commit** `feat(mail): rule engine + rule bawaan BCA & Jago`.

### Task 4: Migrasi & repository storage

**Files:**
- Create: `crates/storage/migrations/0011_email_capture.sql`, `crates/storage/src/email.rs`
- Modify: `crates/storage/src/lib.rs` (`pub mod email;` + `MIGRATIONS`)

**Interfaces:**
- Produces (semua di `nabungo_storage::email`):
  ```rust
  pub struct Cursor { pub uid_validity: u32, pub last_uid: u32 }
  pub fn get_cursor(conn: &Connection, mailbox: &str) -> Result<Option<Cursor>>;
  pub fn set_cursor(tx: &Transaction, mailbox: &str, c: Cursor) -> Result<()>;   // juga set last_synced_at
  pub struct RuleRecord { pub id: Uuid, /* field Rule tanpa Option<id> */ pub enabled: bool }
  pub fn insert_rule(tx: &Transaction, r: &RuleWrite) -> Result<Uuid>;
  pub fn list_active_rules(conn: &Connection) -> Result<Vec<RuleRecord>>;      // enabled, deleted_at NULL, priority DESC, name
  pub fn get_rule(conn: &Connection, id: Uuid) -> Result<Option<RuleRecord>>;
  pub struct CaptureWrite<'a> { pub message_id: &'a str, pub idx: u32, pub rule_id: Uuid, pub date: Date, pub amount: i64, pub currency: &'a str, pub payee: Option<&'a str>, pub raw_snippet: &'a str }
  pub fn insert_capture(tx: &Transaction, w: &CaptureWrite) -> Result<bool>;    // false bila (message_id, idx) sudah ada
  pub enum CaptureStatus { Pending, Confirmed, Ignored }
  pub struct CaptureRecord { pub id: Uuid, pub message_id: String, pub idx: u32, pub rule_id: Option<Uuid>, pub date: Date, pub amount: i64, pub currency: String, pub payee: Option<String>, pub raw_snippet: Option<String>, pub status: CaptureStatus, pub txn_id: Option<Uuid> }
  pub fn get_capture(conn: &Connection, id: Uuid) -> Result<Option<CaptureRecord>>;
  pub fn list_captures(conn: &Connection, status: CaptureStatus) -> Result<Vec<CaptureRecord>>; // date DESC, id DESC
  pub fn set_capture_status(tx: &Transaction, id: Uuid, status: CaptureStatus, txn_id: Option<Uuid>) -> Result<()>;
  pub fn capture_exists(conn: &Connection, message_id: &str) -> Result<bool>;   // capture atau unparsed
  pub struct UnparsedWrite<'a> { pub message_id: &'a str, pub sender: &'a str, pub subject: &'a str, pub received_at: Option<&'a str>, pub raw_snippet: &'a str }
  pub fn insert_unparsed(tx: &Transaction, w: &UnparsedWrite) -> Result<bool>;
  pub struct UnparsedRecord { pub message_id: String, pub sender: String, pub subject: String, pub received_at: Option<String>, pub raw_snippet: String, pub created_at: String }
  pub fn list_unparsed(conn: &Connection) -> Result<Vec<UnparsedRecord>>;      // created_at DESC
  ```
  `RuleWrite` field-nya sama dengan `RuleRecord` tanpa `id`. `kind` disimpan sebagai `'income'|'expense'`. Tipe rule di storage tidak bergantung pada `nabungo-mail` (konversi di app).

- [ ] **Step 1: Tulis `0011_email_capture.sql`** persis seperti skema PRD §6 (`email_sync_cursor`, `email_rule`, `email_capture`, `email_unparsed`), ditambah `CREATE INDEX idx_email_capture_status ON email_capture(status, date)`.
- [ ] **Step 2: Tulis test** di `email.rs` (`open_in_memory()`):
  - `cursor_roundtrip`;
  - `insert_capture_is_idempotent` (insert kedua → `false`, satu baris);
  - `same_message_different_idx_both_stored`;
  - `list_active_rules_orders_and_filters` (disabled dan terhapus tidak muncul; priority DESC);
  - `status_update_sets_txn_id`;
  - `unparsed_is_idempotent`;
  - `capture_exists_checks_both_tables`.
- [ ] **Step 3:** `cargo test -p nabungo-storage email::` → FAIL; implementasi; → PASS. Pastikan `migrations_are_valid` tetap lulus.
- [ ] **Step 4: Commit** `feat(storage): tabel & repository email capture`.

### Task 5: Use case review queue

**Files:**
- Create: `crates/app/src/capture.rs`
- Modify: `crates/app/src/lib.rs` (mod + re-export), `crates/app/src/ledger.rs` (variant `AppError::CaptureNotPending`), `src-tauri/src/error.rs` (`ErrorCode::CaptureNotPending` + pemetaan + test), `ui/src/i18n/id.ts` & `en.ts` (`errors.capture_not_pending`), `ui/src/ipc/bindings.ts` (generate)

**Interfaces:**
- Consumes: Task 3 (`Rule`, `CompiledRules`, `Outcome`, `default_rules`), Task 4 (repository), `write_txn`, `Ledger::payee_suggestion`, `nabungo_storage::setting`.
- Produces:
  ```rust
  pub enum IngestOutcome { Captured(u32 /* item baru */), AlreadySeen, Unparsed }
  impl Ledger {
      pub fn seed_email_rules(&mut self) -> Result<(), AppError>;                         // K8, idempoten
      pub fn email_cursor(&self, mailbox: &str) -> Result<Option<Cursor>, AppError>;
      /// Satu email = satu SQLite transaction: capture/unparsed + cursor (K4).
      pub fn ingest_email(&mut self, mailbox: &str, cursor: Cursor, raw: &[u8], rules: &CompiledRules, local_offset: UtcOffset) -> Result<IngestOutcome, AppError>;
      pub fn email_rules(&self) -> Result<CompiledRules, AppError>;                     // dari list_active_rules
      pub fn list_captures(&self, status: CaptureStatus) -> Result<Vec<CaptureRecord>, AppError>;
      pub fn list_unparsed(&self) -> Result<Vec<UnparsedRecord>, AppError>;
      pub fn capture_draft(&self, id: Uuid) -> Result<CaptureDraft, AppError>;            // K9
      pub fn confirm_capture(&mut self, id: Uuid, form: &TxnForm) -> Result<Uuid, AppError>; // K10
      pub fn link_capture(&mut self, id: Uuid, txn_id: Uuid) -> Result<(), AppError>;
      pub fn ignore_capture(&mut self, id: Uuid) -> Result<(), AppError>;
      pub fn duplicate_candidates(&self, id: Uuid) -> Result<Vec<Uuid>, AppError>;        // K11, maks 5
  }
  pub struct CaptureDraft { pub capture_id: Uuid, pub kind: TxnKind, pub date: Date, pub amount: i64, pub payee: Option<String>, pub account_id: Option<Uuid>, pub category_id: Option<Uuid>, pub note: Option<String> /* nama rule */ }
  ```
  `ingest_email` yang menerima `parse_message == None` tetap memajukan cursor dan mengembalikan `Unparsed` dengan `message_id` hash, sender/subject kosong. `CompiledRules` membawa `rule_id` tiap rule agar capture bisa menyimpan `rule_id`. `CaptureStatus`, `CaptureRecord`, `UnparsedRecord`, `Cursor` di-re-export dari `nabungo-app`.

- [ ] **Step 1: Tulis test** di `capture.rs` (pakai `testkit::Kit` + raw fixture dari `nabungo-mail` lewat `include_bytes!("../../mail/tests/fixtures/…")`):
  - `seed_is_once`: seed, hapus satu rule (`deleted_at`), seed lagi → jumlah rule aktif tetap berkurang satu.
  - `ingest_creates_pending_and_advances_cursor`: cursor jadi `last_uid` email tersebut.
  - `ingest_same_email_twice_is_already_seen`.
  - `ingest_unknown_goes_unparsed`.
  - `draft_uses_payee_suggestion`: transaksi manual lama payee "TOKO ABC" kategori Makan di akun Dompet → draft berisi akun Dompet & kategori Makan.
  - `confirm_creates_txn_and_marks_confirmed`: saldo BCA turun 150.000, `txn_id` terisi.
  - `confirm_twice_is_rejected`: confirm kedua → `CaptureNotPending`, jumlah transaksi tetap satu.
  - `ignore_then_confirm_rejected`.
  - `link_marks_confirmed_without_new_txn`.
  - `duplicate_candidates_within_one_day`: expense manual 150.000 tanggal 28/10 cocok; 29/10 dan 150.001 tidak.
  - `confirm_failure_keeps_pending`: form dengan akun diarsip → `Archived`, status tetap pending.
- [ ] **Step 2:** `cargo test -p nabungo-app capture::` → FAIL; implementasi; → PASS.
- [ ] **Step 3: Pemetaan error**: `ErrorCode::CaptureNotPending` (`capture_not_pending`) di `src-tauri/src/error.rs`, test pemetaan seperti `AppError::Duplicate`. Teks i18n: id "Transaksi email ini sudah diproses.", en "This email transaction has already been processed." Generate bindings: `cargo test -p nabungo-desktop export_bindings`.
- [ ] **Step 4:** `cargo test --workspace` dan `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit** `feat(app): review queue email capture`.

### Task 6: Loop sync + sumber IMAP + CLI

**Files:**
- Create: `crates/mail/src/source.rs`, `crates/app/src/email_sync.rs`, `crates/app/examples/capture_sync.rs`
- Modify: `crates/mail/src/lib.rs`, `crates/app/src/lib.rs`

**Interfaces:**
- Consumes: Task 5 (`ingest_email`, `email_cursor`, `email_rules`, `seed_email_rules`).
- Produces:
  ```rust
  // nabungo-mail::source
  #[derive(Debug, thiserror::Error)]
  pub enum SourceError { #[error("login failed")] Auth, #[error("mailbox not found")] Mailbox, #[error("connection: {0}")] Connection(String) }
  pub struct Fetched { pub uid: u32, pub raw: Vec<u8> }
  pub trait MailSource {
      /// Read-only (EXAMINE). Mengembalikan UIDVALIDITY.
      fn open(&mut self, mailbox: &str) -> Result<u32, SourceError>;
      /// UID > `after`, naik, maksimal `limit`. Memakai BODY.PEEK[] (tidak menandai dibaca).
      fn fetch_after(&mut self, after: u32, limit: usize) -> Result<Vec<Fetched>, SourceError>;
  }
  pub struct ImapSource { /* session */ }
  impl ImapSource { pub fn connect(host: &str, port: u16, user: &str, password: &str) -> Result<Self, SourceError>; }

  // nabungo-app::email_sync
  pub const SYNC_BATCH: usize = 50;
  pub struct SyncSummary { pub new_items: u32, pub unparsed: u32, pub seen: u32 }
  #[derive(Debug, thiserror::Error)]
  pub enum SyncError { #[error(transparent)] Source(#[from] SourceError), #[error(transparent)] App(#[from] AppError) }
  pub fn sync_mailbox(ledger: &mut Ledger, source: &mut dyn MailSource, mailbox: &str, local_offset: UtcOffset) -> Result<SyncSummary, SyncError>;
  ```
  Alur `sync_mailbox`: `seed_email_rules` → `open` → cursor (validity berbeda atau belum ada → `last_uid = 0`) → loop `fetch_after(last_uid, SYNC_BATCH)` sampai kosong → `ingest_email` per pesan dengan `Cursor { uid_validity, last_uid: uid }`. Fungsi ini dirancang agar 9b bisa memecahnya per batch tanpa memegang lock ledger selama fetch.

- [ ] **Step 1: Tulis test** di `email_sync.rs` dengan `FakeSource { validity, messages: Vec<Fetched>, fail_after: Option<usize> }`:
  - `first_sync_ingests_all`;
  - `second_sync_fetches_nothing_new` (`fetch_after` dipanggil dengan UID terakhir);
  - `sync_resumes_after_source_error`: fail setelah 2 pesan → `Err(Source)`; sync ulang tanpa fail → total capture = semua email, tanpa dobel;
  - `uidvalidity_change_rescans_without_duplicates`: ignore satu capture, ganti validity, sync → tidak ada capture baru, yang di-ignore tetap `Ignored`;
  - `batches_until_empty`: 120 pesan → tiga batch.
- [ ] **Step 2:** `cargo test -p nabungo-app email_sync::` → FAIL; implementasi; → PASS.
- [ ] **Step 3: Implementasi `ImapSource`** dengan `imap` 2.4.1: `examine`, `uid_fetch("{after+1}:*", "BODY.PEEK[]")` (buang hasil dengan UID ≤ `after`, karena `n:*` selalu mengembalikan pesan terakhir), urutkan, ambil `limit`. Error login → `Auth`, `NO` saat EXAMINE → `Mailbox`. Tidak di-unit-test (jaringan); diuji manual di Step 5.
- [ ] **Step 4: Tulis `examples/capture_sync.rs`**: `capture_sync <data-dir> [--mailbox Finance]`. Buka DB seperti `seed_fixture` (password via `NABUNGO_DB_PASSWORD` bila terenkripsi). Credential dari `NABUNGO_IMAP_USER`/`NABUNGO_IMAP_PASSWORD`. Offset dari `UtcOffset::current_local_offset()` dengan fallback `+07:00`. Cetak `SyncSummary` lalu daftar pending (`tanggal  nominal  payee  rule`). Password tidak pernah dicetak.
- [ ] **Step 5: Uji manual** ke folder data salinan: `cargo run -p nabungo-app --example capture_sync -- /tmp/nbg-test`, dua kali.
  Expected: run pertama ada capture/unparsed; run kedua `new_items: 0`; email di Gmail tetap belum dibaca.
- [ ] **Step 6: Commit** `feat(app): sync email IMAP + CLI capture_sync`.

### Task 7: Dokumentasi & verifikasi akhir

**Files:**
- Modify: `CLAUDE.md` (arah dependensi + `nabungo-mail`; command `capture_sync`), `docs/plans/2026-10-12-v1-roadmap.md` (baris fase 9 dengan link plan ini), `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` (status milestone 1–3)

- [ ] **Step 1: Update dokumen** di atas. Di `CLAUDE.md`, tambah bullet `crates/mail` di §Arsitektur: "parsing email + rule engine murni + `MailSource` (IMAP). Satu-satunya crate dengan kode jaringan; tidak dipanggil app sampai user mengonfigurasi email (9b)."
- [ ] **Step 2: Verifikasi penuh**: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test`.
  Expected: semua lulus, `git diff --exit-code ui/src/ipc/bindings.ts` bersih.
- [ ] **Step 3: Ukur ulang installer** (`check-bundle-size.mjs`) → < 15 MB.
- [ ] **Step 4: Commit** `docs: fase 9a email capture backend`.
