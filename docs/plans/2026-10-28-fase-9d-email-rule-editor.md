# Fase 9d — Email Capture: Rule Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EML-04 bisa dipakai dari aplikasi. User bisa:
- Melihat, menambah, mengubah, menonaktifkan, dan menghapus (dengan undo) rule parsing email.
- Mengisi **akun & kategori default** per rule. Rule bawaan (9a) belum punya akun, jadi saat ini hampir tidak ada draft yang *siap* untuk `Catat`.
- **Menguji** rule terhadap email di tab Gagal dibaca atau teks email yang ditempel, dan melihat langkah mana yang gagal.
- **Membuat rule dari email gagal dibaca**: form terisi saran dari email itu.
- **Memproses ulang** email gagal dibaca dengan rule terbaru, tanpa jaringan: email yang kini cocok pindah ke Perlu ditinjau.

**Architecture:**
- `nabungo-mail` (`rule.rs`): validasi rule, `trace` (penjelasan langkah per langkah, satu jalur kode dengan `apply`), saran rule dari email. Semua murni, tanpa IO.
- `nabungo-storage` (`email.rs`): tulis/ubah/hapus-lunak/pulihkan rule, daftar semua rule, hapus baris unparsed.
- `nabungo-app` (`email_rules.rs` baru, blok `impl Ledger`): use case rule + `reparse_unparsed`.
- `src-tauri`: command tipis + DTO.
- `ui`: tab **Rule** di Inbox, `EmailRuleSheet` (form + panel uji), tombol `Buat rule` di tab Gagal dibaca, port rule engine untuk mode browser.

**Tech Stack:** Rust (regex, rusqlite, tauri-specta), Vue 3 + shadcn-vue (Tabs, Sheet, Select, Switch, Collapsible, AlertDialog), Pinia, vitest, Playwright.

**Spec:**
- `PRD.md`: EML-02, EML-04 (§5), skema `email_rule` (§6), layar Inbox & Pengaturan (§7), risiko format email berubah (§10).
- Brief `docs/plans/Brief Auto-Capture Transaksi via IMAP.md`: §Data model & rule engine, §Review flow (Unparsed), milestone 6.
- Plan 9a `docs/plans/2026-10-27-fase-9a-email-capture-backend.md`: rule engine, rule bawaan, `seed_email_rules`.
- Plan 9c `docs/plans/2026-10-28-fase-9c-email-inbox.md`: K12 (tab Gagal dibaca), K14 (mode browser).

**Di luar plan ini:**
- Ambil ulang email dari IMAP untuk re-parse (re-parse hanya memakai data lokal, lihat K6).
- Re-parse capture yang sudah ada di Perlu ditinjau (nominal/payee/tanggal capture tidak berubah saat rule diedit; akun/kategori ikut, lihat K7).
- "Pulihkan rule bawaan", impor/ekspor rule, rule berbasis ML.
- Import mutasi CSV/PDF (fallback brief, sub-fase terpisah).

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Lokasi**: tab ketiga **Rule** di Inbox (`/inbox?tab=rules`, tab di query seperti Label `?tab=payee`). Kartu Email di Settings mendapat link `Kelola rule`. PRD §7 baris Pengaturan diperbarui ("rule" pindah ke Inbox). | Alur "email gagal dibaca → buat rule → proses ulang" ada di satu layar. Menu Inbox sudah hanya tampil saat email capture relevan. |
| K2 | **Daftar rule** urut `priority` DESC, `name`. Isi: rule aktif dan nonaktif (bukan yang dihapus). Baris 48px: nama, meta (pengirim · jenis · akun · kategori/tujuan), badge `Nonaktif` bila mati, switch aktif, `⋯` (Ubah, Duplikat, Hapus). Akun/kategori kosong atau diarsip ditulis "Tanpa akun"/"Tanpa kategori" berwarna warning. | User langsung melihat rule mana yang membuat draft belum siap. |
| K3 | **Validasi di Rust** (`nabungo_mail::rule::validate`): nama & pengirim tidak kosong; semua regex bisa di-compile (flag sama dengan engine: case-insensitive + multi-line); `amount_regex` punya ≥ 1 grup tangkap; `payee_regex`/`date_regex` juga punya ≥ 1 grup bila diisi; `date_regex` dan `date_format` diisi berpasangan, format ∈ {`dd/mm/yyyy`, `dd-mm-yyyy`, `dd mmm yyyy`}. Gagal → `AppError::EmailRuleInvalid(RuleField)` → code `email_rule_invalid` dengan `entity` = nama field (`name`, `sender`, `subject`, `amount`, `payee`, `date`, `date_format`). Referensi diperiksa di `nabungo-app` dengan error yang sudah ada: akun/kategori tidak ada → `not_found`, diarsip → `archived`; kategori hanya untuk income/expense dan jenisnya harus sama; akun lawan hanya untuk transfer dan berbeda dari akun rule. | Error tampil di field yang tepat. Rule rusak tidak pernah tersimpan (selama ini `CompiledRules` diam-diam melewatinya). |
| K4 | **`trace` satu jalur dengan `apply`**: langkah pencocokan dipecah ke fungsi bersama; `apply` = jalankan `trace` per rule lalu ambil hasil pertama yang lolos sampai langkah nominal. `RuleTrace { sender_ok, subject_ok: Option<bool>, amounts: Vec<String>, payee: Option<String>, date_text: Option<String>, result: Result<Vec<Captured>, TraceFail> }`, `TraceFail` = `Sender \| Subject \| NoAmount \| AmountInvalid(String) \| NoDate \| DateInvalid(String)`. Test parity: untuk semua fixture `.eml`, hasil `apply` == hasil `trace` rule terpilih. | Pratinjau tidak boleh berbeda dari perilaku sync sungguhan. |
| K5 | **Uji rule** (`test_email_rule(rule, sample)`) tidak menulis apa pun. `sample` = `Unparsed(message_id)` atau `Pasted { sender, subject, text }`. Hasil = `RuleTrace` + `unparsed_matches` (jumlah email gagal dibaca yang lolos rule ini **sendiri**, tanpa memperhitungkan rule lain). Panel uji di sheet: pilih email gagal dibaca (Select, label = subjek) atau tab "Tempel email", tombol `Uji`, lalu daftar langkah ✓/✗ + nominal/payee/tanggal hasil. | User membuat regex secara iteratif tanpa menunggu sync. |
| K6 | **Re-parse lokal** (`reparse_unparsed(local_offset)`): setiap baris `email_unparsed` dengan pengirim tidak kosong diubah menjadi `MailMessage` (from = sender, subject, sent_at = received_at, text = `raw_snippet`), lalu dijalankan `CompiledRules` aktif. Yang cocok → `insert_capture` per item + hapus baris unparsed; semuanya satu SQLite transaction. Hasil `{ emails, items }`. Baris dari raw yang tidak terbaca (pengirim kosong) dan email yang nominalnya ada setelah 4.000 karakter pertama (batas `SNIPPET_CHARS`) tetap di Gagal dibaca. | Tidak ada network call baru. Data yang dibutuhkan sudah tersimpan di 9a. |
| K7 | **Kapan re-parse**: otomatis di akhir `create_email_rule`, `update_email_rule`, dan `set_email_rule_enabled(true)` dalam transaction yang sama. Hasil use case = `RuleSaved { id, reparsed: ReparseSummary }`. Toast: "Rule disimpan · 3 email dipindah ke Perlu ditinjau" (bagian kedua hanya bila > 0). Capture pending tidak di-parse ulang; akun/kategori draft sudah dihitung dari rule saat dibaca (`draft_of`), jadi mengubah akun rule langsung membuat draft lama siap. | Satu langkah bagi user. Capture tidak pernah berubah nominal di belakang layar. |
| K8 | **Hapus rule** = soft delete (`deleted_at`). Capture lama tetap menunjuk rule (nama tetap tampil di Inbox). Toast `Urungkan` → `restore_email_rule`. Dialog konfirmasi menyebut jumlah capture pending yang memakai rule ini. Tidak ada hard delete. | Konvensi repo: hapus bisa di-undo; FK `email_capture.rule_id` tetap valid. |
| K9 | **Buat rule dari email gagal dibaca**: tombol `Buat rule` di baris unparsed → `suggest_email_rule(message_id)` → sheet terisi lalu panel uji otomatis memakai email itu. Saran (murni, `nabungo_mail::rule::suggest`): nama = subjek (≤ 60 karakter); pengirim = domain alamat (`bca.co.id`); `subject_regex` = `^<subjek di-escape>$`; `amount_regex` = label + nominal pertama yang ditemukan, mis. `^Total Payment : IDR ([\d.,]+)$` (label = teks sebelum `Rp`/`IDR` di baris yang sama, di-escape), fallback `(?:Rp\|IDR) ?([\d.,]+)`; `date_regex`/`date_format` diisi bila ada baris berlabel yang cocok dengan salah satu format; `kind` = expense; `priority` = 10 (di atas rule bawaan). Tidak ada payee regex. | Titik awal yang langsung lolos uji pada email contoh; user tinggal merapikan. |
| K10 | **Form sheet** (`EmailRuleSheet`, sheet kanan 440px, pola `RuleSheet`): Nama, Pengirim, Jenis (Pengeluaran/Pemasukan/Transfer keluar/Transfer masuk), Akun, Kategori default (income/expense) atau Akun lawan (transfer), lalu kotak lipat **Pola** (subjek, nominal, payee, tanggal + format) dan Prioritas + switch Aktif. Pola terbuka default saat membuat rule baru / dari email, tertutup saat mengubah. Hint di bawah Pola: "Regex Rust, tanpa beda huruf besar/kecil; `^`/`$` per baris. Grup pertama `(...)` diambil." | Kebanyakan user hanya perlu mengisi akun/kategori rule bawaan; regex tetap bisa diakses. |
| K11 | **Seed rule** dipindah juga ke `save_email_account` (selain awal sync), agar tab Rule berisi rule bawaan begitu akun email disimpan. Tetap sekali (`email.rules_seeded`). | Sebelumnya rule baru ada setelah sync pertama. |
| K12 | **Mode browser**: `ui/src/ipc/browser-rule.ts` adalah port `crates/mail/src/rule.rs` (validate, trace/apply, normalize_amount, parse_date, suggest) + data rule bawaan (`defaults.rs`). Regex JS dengan flag `im`. Paritas dijaga satu file kasus bersama `crates/mail/tests/fixtures/rule-cases.json` (input rule + email teks → hasil) yang dibaca test Rust **dan** vitest. Kasus hanya memakai sintaks regex yang sama di kedua dialek. CLAUDE.md: "ubah keduanya bersamaan". | Test UI & e2e tanpa Tauri; pola `browser-amount.ts`/`browser-recurrence.ts`. |
| K13 | **Privasi**: regex dan nama rule boleh di log; subjek, pengirim, dan teks email tidak (termasuk teks tempelan & hasil uji). Teks tempelan tidak disimpan. | Sama dengan 9c. |

## Global Constraints

- Arah dependensi `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage, nabungo-mail)`. `nabungo-mail` tidak bergantung pada storage. Command Tauri tipis; logika di `nabungo-app`/`nabungo-mail`.
- Satu use case tulis = satu SQLite transaction (termasuk re-parse dalam simpan rule, K7). Repository tulis menerima `&Transaction`; baca boleh `&Connection`.
- Tidak ada migrasi baru: skema `email_rule` (0011) sudah memuat semua kolom.
- Command baru di `collect_commands!` (`src-tauri/src/lib.rs`); bindings di-generate dan di-commit.
- Error code baru `email_rule_invalid` masuk `ErrorCode` + `errors` di `ui/src/i18n/id.ts` & `en.ts`; pesan per field lewat `emailRules.fieldError.<field>`.
- UI mengikuti `docs/design/design.md`; semua panggilan lewat `ui/src/ipc/`; store memuat ulang setelah setiap aksi tulis.
- Tidak ada network call baru. Re-parse dan uji hanya membaca DB/teks lokal.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`, `pnpm --filter @nabungo/ui e2e`.

## Review Focus

1. **Pratinjau = perilaku sync**: rule yang lolos uji juga menangkap email yang sama saat sync/re-parse. Test: `trace_matches_apply_on_fixtures` (Task 1), `rule_cases_parity` (Rust Task 1 + vitest Task 4).
2. **Rule rusak tidak bisa disimpan** dan error tampil di field yang tepat. Test: `validate_rejects_*` (Task 1), `invalid_regex_shows_field_error` (Task 5).
3. **Re-parse atomik & idempoten**: email pindah dari Gagal dibaca ke Perlu ditinjau tepat sekali; simpan rule dua kali tidak membuat capture dobel; error di tengah membatalkan semuanya. Test: `reparse_moves_matching_unparsed`, `reparse_twice_is_noop`, `invalid_rule_save_rolls_back` (Task 2).
4. **Mengubah akun rule membuat draft lama siap** tanpa mengubah nominal/payee capture. Test: `rule_account_change_updates_pending_draft` (Task 2), e2e `set account makes Catat appear` (Task 6).
5. **Hapus rule tidak merusak capture**: capture pending tetap tampil dengan nama rule; undo mengembalikan rule ke daftar dan ke engine. Test: `delete_keeps_capture_rule_name`, `restore_reactivates_rule` (Task 2).

---

### Task 1: Validasi, trace, dan saran rule di `nabungo-mail`

**Files:**
- Modify: `crates/mail/src/rule.rs`, `crates/mail/src/lib.rs`
- Create: `crates/mail/src/suggest.rs`, `crates/mail/tests/fixtures/rule-cases.json`, `crates/mail/tests/rule_cases.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum RuleField { Name, Sender, Subject, Amount, Payee, Date, DateFormat }
  pub const DATE_FORMATS: [&str; 3] = ["dd/mm/yyyy", "dd-mm-yyyy", "dd mmm yyyy"];
  /// K3 (tanpa cek akun/kategori).
  pub fn validate(rule: &Rule) -> Result<(), RuleField>;

  pub enum TraceFail { Sender, Subject, NoAmount, AmountInvalid(String), NoDate, DateInvalid(String) }
  pub struct RuleTrace {
      pub sender_ok: bool,
      pub subject_ok: Option<bool>,        // None = rule tanpa subject_regex
      pub amounts: Vec<String>,            // teks grup 1 apa adanya
      pub payee: Option<String>,
      pub date_text: Option<String>,
      pub result: Result<Vec<Captured>, TraceFail>,
  }
  /// K4. Rule tidak valid → Err(field).
  pub fn trace(rule: &Rule, msg: &MailMessage, local_offset: UtcOffset) -> Result<RuleTrace, RuleField>;

  // suggest.rs (K9)
  pub fn suggest(msg: &MailMessage) -> Rule;   // id None, account/kategori None
  ```
  `CompiledRules::apply` direfaktor memakai langkah yang sama dengan `trace`; perilaku publik tidak berubah (test 9a tetap hijau).

- [ ] **Step 1: Tulis test** di `rule.rs`:
  - `validate_accepts_default_rules`: semua `default_rules()` lolos.
  - `validate_rejects_empty_name_and_sender`, `validate_rejects_bad_regex` (`amount_regex = "("` → `Amount`), `validate_rejects_amount_without_group`, `validate_rejects_date_without_format` (dan sebaliknya), `validate_rejects_unknown_date_format`.
  - `trace_reports_failing_step`: pengirim beda → `Sender`; subjek beda → `subject_ok == Some(false)`, `Subject`; nominal `25,000` → `AmountInvalid("25,000")`; date regex cocok tapi `32 Okt 2026` → `DateInvalid`.
  - `trace_matches_apply_on_fixtures`: setiap `.eml` di `tests/fixtures/` → rule yang dipilih `apply` punya `trace(..).result == Ok(items)` yang sama.
- [ ] **Step 2: Tulis test** `suggest.rs`:
  - `suggest_from_bca_qris`: fixture QRIS BCA (teks, pengirim & subjek dari fixture) → sender `bca.co.id`, subject `^Internet Transaction Journal$`, amount regex menangkap `20,000.00`, `date_format == Some("dd mmm yyyy")`, `trace` hasilnya `Ok` dengan nominal yang benar.
  - `suggest_falls_back_to_generic_amount`: teks `Anda membayar Rp 15.000 di toko` → amount regex generik, `trace` Ok 15.000.
  - `suggest_escapes_subject`: subjek `Transaksi (QRIS) sukses?` → regex valid dan cocok dengan subjek itu.
  - `suggest_name_truncates`: subjek 100 karakter → nama ≤ 60 karakter.
- [ ] **Step 3: Tulis** `rule-cases.json` (± 12 kasus: tiap `TraceFail`, multi-item, format tanggal ×3, nominal ID/EN) + `tests/rule_cases.rs` `rule_cases_parity` yang membaca file itu dan membandingkan `trace`.
- [ ] **Step 4:** `cargo test -p nabungo-mail` → FAIL; implementasi; → PASS.
- [ ] **Step 5: Commit** `feat(mail): validasi, trace, dan saran rule email`.

### Task 2: Storage & use case rule di `nabungo-app`

**Files:**
- Modify: `crates/storage/src/email.rs`
- Create: `crates/app/src/email_rules.rs`
- Modify: `crates/app/src/capture.rs` (`to_rule`/`kind_str` jadi `pub(crate)`), `crates/app/src/email_settings.rs` (seed, K11), `crates/app/src/ledger.rs` (`AppError::EmailRuleInvalid(RuleField)`), `crates/app/src/lib.rs` (re-export)

**Interfaces:**
- Produces (storage):
  ```rust
  /// Semua rule yang tidak dihapus (aktif & nonaktif), urut priority DESC, name, id.
  pub fn list_rules(conn: &Connection) -> Result<Vec<RuleRecord>>;
  pub fn update_rule(tx: &Transaction, id: Uuid, r: &RuleWrite) -> Result<()>;      // NotFound bila dihapus
  pub fn set_rule_enabled(tx: &Transaction, id: Uuid, enabled: bool) -> Result<()>;
  pub fn soft_delete_rule(tx: &Transaction, id: Uuid) -> Result<()>;
  pub fn restore_rule(tx: &Transaction, id: Uuid) -> Result<()>;
  pub fn count_pending_for_rule(conn: &Connection, id: Uuid) -> Result<u32>;
  pub fn get_unparsed(conn: &Connection, message_id: &str) -> Result<Option<UnparsedRecord>>;
  pub fn delete_unparsed(tx: &Transaction, message_id: &str) -> Result<()>;
  ```
- Produces (app):
  ```rust
  pub struct EmailRuleInput { /* = Rule tanpa id, + enabled: bool */ }
  pub struct EmailRuleItem { pub rule: Rule /* id Some */, pub enabled: bool, pub pending: u32 }
  pub struct ReparseSummary { pub emails: u32, pub items: u32 }
  pub struct RuleSaved { pub id: Uuid, pub reparsed: ReparseSummary }
  pub enum RuleSample { Unparsed(String), Pasted { sender: String, subject: String, text: String } }
  pub struct RuleTest { pub trace: RuleTrace, pub unparsed_matches: u32 }
  impl Ledger {
      pub fn email_rules_list(&self) -> Result<Vec<EmailRuleItem>, AppError>;                         // K2
      pub fn create_email_rule(&mut self, input: &EmailRuleInput, off: UtcOffset) -> Result<RuleSaved, AppError>;
      pub fn update_email_rule(&mut self, id: Uuid, input: &EmailRuleInput, off: UtcOffset) -> Result<RuleSaved, AppError>;
      pub fn set_email_rule_enabled(&mut self, id: Uuid, enabled: bool, off: UtcOffset) -> Result<RuleSaved, AppError>;
      pub fn delete_email_rule(&mut self, id: Uuid) -> Result<(), AppError>;                          // K8
      pub fn restore_email_rule(&mut self, id: Uuid, off: UtcOffset) -> Result<RuleSaved, AppError>;
      pub fn test_email_rule(&self, input: &EmailRuleInput, sample: &RuleSample, off: UtcOffset) -> Result<RuleTest, AppError>; // K5
      pub fn suggest_email_rule(&self, message_id: &str) -> Result<EmailRuleInput, AppError>;         // K9
      pub fn reparse_unparsed(&mut self, off: UtcOffset) -> Result<ReparseSummary, AppError>;          // K6
  }
  ```
  Re-parse dipanggil sebagai fungsi privat `reparse_in(tx: &Transaction, off)` dari use case simpan (K7) dan dari `reparse_unparsed`.

- [ ] **Step 1: Tulis test storage** `rules_crud`: insert 2 rule → `list_rules` urut priority; `set_rule_enabled(false)` → hilang dari `list_active_rules`, tetap di `list_rules`; `soft_delete_rule` → hilang dari keduanya; `restore_rule` → kembali; `update_rule` pada rule terhapus → `NotFound`. `unparsed_get_and_delete`.
- [ ] **Step 2: Tulis test app** di `email_rules.rs` (helper `kit`/`ingest` dari `capture.rs` dibuat `pub(crate)` di modul test bersama):
  - `create_rejects_invalid_regex`: `Err(EmailRuleInvalid(Amount))`, tidak ada baris baru.
  - `create_rejects_archived_account`, `create_rejects_category_kind_mismatch`, `create_rejects_counter_on_expense`, `create_rejects_counter_equal_account`.
  - `reparse_moves_matching_unparsed`: ingest email UNKNOWN (unparsed) → `create_email_rule` dari `suggest_email_rule(msg_id)` → `reparsed == { emails: 1, items: 1 }`, `list_unparsed` kosong, `pending_captures` berisi 1 dengan nama rule baru.
  - `reparse_twice_is_noop`: `reparse_unparsed` kedua → `{0, 0}`, jumlah capture tetap.
  - `reparse_skips_unreadable`: unparsed dengan sender kosong tetap ada.
  - `invalid_rule_save_rolls_back`: `update_email_rule` dengan akun tidak ada → error; rule & unparsed tidak berubah.
  - `rule_account_change_updates_pending_draft`: capture QRIS (rule bawaan tanpa akun) → update rule dengan akun Kit → `pending_captures()[0].draft.account_id == Some(akun)`, nominal & payee sama.
  - `disable_rule_excludes_from_sync`: nonaktifkan rule QRIS → ingest QRIS kedua (message-id lain) → unparsed.
  - `delete_keeps_capture_rule_name` & `restore_reactivates_rule`.
  - `test_rule_counts_unparsed_matches`: 2 unparsed cocok, 1 tidak → `unparsed_matches == 2`; tidak ada yang berubah di DB.
  - `seed_on_save_email_account`: `save_email_account` → `email_rules_list().len() == default_rules().len()`.
- [ ] **Step 3:** `cargo test -p nabungo-storage email::` dan `cargo test -p nabungo-app email_rules::` → FAIL; implementasi; → PASS; `cargo test --workspace` → PASS.
- [ ] **Step 4: Commit** `feat(app): use case rule email + re-parse lokal`.

### Task 3: Command & DTO Tauri

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs`
- Modify (generate): `ui/src/ipc/bindings.ts`
- Modify: `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts` (`errors.email_rule_invalid` saja)

**Interfaces:**
- Produces (DTO, `camelCase`):
  ```rust
  pub enum EmailRuleKindDto { Income, Expense, TransferOut, TransferIn }
  pub struct EmailRuleInputDto {
      pub name: String, pub sender_pattern: String, pub subject_regex: Option<String>,
      pub amount_regex: String, pub payee_regex: Option<String>, pub date_regex: Option<String>,
      pub date_format: Option<String>, pub kind: EmailRuleKindDto, pub account_id: Option<String>,
      pub counter_account_id: Option<String>, pub default_category_id: Option<String>,
      pub priority: i64, pub enabled: bool,
  }
  pub struct EmailRuleDto { pub id: String, #[serde(flatten)] pub input: EmailRuleInputDto, pub pending: u32 }
  pub struct ReparseSummaryDto { pub emails: u32, pub items: u32 }
  pub struct RuleSavedDto { pub id: String, pub reparsed: ReparseSummaryDto }
  pub enum RuleSampleDto { Unparsed { message_id: String }, Pasted { sender: String, subject: String, text: String } }
  pub enum TraceFailDto { Sender, Subject, NoAmount, AmountInvalid, NoDate, DateInvalid }
  pub struct RuleTestDto {
      pub sender_ok: bool, pub subject_ok: Option<bool>, pub amounts: Vec<String>, pub payee: Option<String>,
      pub date_text: Option<String>, pub fail: Option<TraceFailDto>, pub fail_text: Option<String>,
      pub items: Vec<RuleTestItemDto /* amount, date, payee */>, pub unparsed_matches: u32,
  }
  ```
  `ErrorCode::EmailRuleInvalid`; `entity` = `name`/`sender`/`subject`/`amount`/`payee`/`date`/`date_format`.
- Produces (command; yang menulis memakai `UtcOffset` lokal yang sama dengan sync, yang disimpan di `state::EmailRuntime` saat `init` di `src-tauri/src/lib.rs`):
  ```rust
  list_email_rules() -> Vec<EmailRuleDto>
  create_email_rule(input: EmailRuleInputDto) -> RuleSavedDto
  update_email_rule(id: String, input: EmailRuleInputDto) -> RuleSavedDto
  set_email_rule_enabled(id: String, enabled: bool) -> RuleSavedDto
  delete_email_rule(id: String) -> ()
  restore_email_rule(id: String) -> RuleSavedDto
  test_email_rule(input: EmailRuleInputDto, sample: RuleSampleDto) -> RuleTestDto
  suggest_email_rule(message_id: String) -> EmailRuleInputDto
  reparse_unparsed() -> ReparseSummaryDto
  ```

- [ ] **Step 1: Tulis test** `dto.rs`/`error.rs`: `email_rule_dto_round_trip` (camelCase, `kind: "transfer_out"`), `rule_test_dto_maps_fail_text` (`AmountInvalid("25,000")` → `fail: "amount_invalid"`, `failText: "25,000"`), `email_rule_invalid_carries_field`.
- [ ] **Step 2:** `cargo test -p nabungo-desktop` → FAIL; implementasi; → PASS.
- [ ] **Step 3:** `cargo test -p nabungo-desktop export_bindings && pnpm --filter @nabungo/ui typecheck` → PASS setelah `errors.email_rule_invalid` ditambahkan.
- [ ] **Step 4: Commit** `feat(desktop): command rule email` (termasuk `bindings.ts`).

### Task 4: IPC, port browser, store `emailRules`

**Files:**
- Create: `ui/src/ipc/browser-rule.ts`, `ui/src/ipc/browser-rule.test.ts`, `ui/src/stores/emailRules.ts`, `ui/src/stores/emailRules.test.ts`
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/ipc/browser-ledger.ts` (rule in-memory, seed bawaan saat `connectEmail` browser, re-parse memindahkan `unparsed` → `captures`), `ui/src/ipc/browser-ledger.test.ts`, `ui/src/stores/inbox.ts` (reload setelah re-parse)

**Interfaces:**
- Produces (`ipc`): `listEmailRules`, `createEmailRule`, `updateEmailRule`, `setEmailRuleEnabled`, `deleteEmailRule`, `restoreEmailRule`, `testEmailRule`, `suggestEmailRule`, `reparseUnparsed` (tipe dari bindings).
- Produces (`browser-rule.ts`, K12): `validate`, `trace`, `apply`, `normalizeAmount`, `parseDate`, `suggest`, `DEFAULT_RULES`. Mode browser menyimpan `raw_snippet` unparsed seed sebagai teks email; `__seedEmailInbox` menerima `unparsed[].snippet` seperti sekarang.
- Produces (`stores/emailRules.ts`, setup store `useEmailRulesStore`):
  ```ts
  rules: Ref<EmailRuleDto[]>; loaded: Ref<boolean>
  reload(): Promise<void>
  save(id: string | null, input: EmailRuleInputDto): Promise<RuleSavedDto>   // lalu reload + inbox.reload bila reparsed > 0
  setEnabled(id: string, on: boolean): Promise<RuleSavedDto>
  remove(id: string): Promise<void>
  restore(id: string): Promise<RuleSavedDto>
  ```
  Uji (`testEmailRule`) dan saran dipanggil langsung lewat `ipc` dari sheet (tidak menulis, tidak perlu state store).

- [ ] **Step 1: Tulis test** `browser-rule.test.ts`: `rule_cases_parity` membaca `../../../crates/mail/tests/fixtures/rule-cases.json` (cek `server.fs.allow` Vite/vitest mengizinkan root workspace; bila tidak, tambahkan) dan membandingkan `trace`; `normalizeAmount` kasus ID/EN/ambigu sama dengan test Rust; `suggest_from_text` sama dengan kasus Task 1.
- [ ] **Step 2: Tulis test** `browser-ledger.test.ts` describe `email rules`: create invalid → `email_rule_invalid` + `entity: 'amount'`; create dari `suggestEmailRule` → unparsed pindah ke pending; nonaktifkan/hapus/pulihkan.
- [ ] **Step 3: Tulis test** `emailRules.test.ts`: `save_reloads_inbox_when_reparsed`, `remove_then_restore`.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-rule.test.ts src/ipc/browser-ledger.test.ts src/stores/emailRules.test.ts` → FAIL; implementasi; → PASS. `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit** `feat(ui): ipc & store rule email`.

### Task 5: Tab Rule, sheet rule, buat rule dari email

**Files:**
- Create: `ui/src/components/inbox/EmailRuleList.vue`, `ui/src/components/inbox/EmailRuleSheet.vue`, `ui/src/components/inbox/EmailRuleSheet.test.ts`, `ui/src/components/inbox/RuleTestPanel.vue`
- Modify: `ui/src/views/InboxView.vue` (+ test; tab `rules`, sinkron ke `?tab=`), `ui/src/components/inbox/UnparsedList.vue` (tombol `Buat rule`), `ui/src/components/settings/EmailSection.vue` (+ test; link `Kelola rule`), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/design.md` (§8.10)

**Interfaces:**
- Consumes: Task 4.
- Produces:
  - `EmailRuleSheet`: props `{ open: boolean; rule: EmailRuleDto | null; seed: { input: EmailRuleInputDto; sampleMessageId: string } | null }`, emits `close`. Snapshot lokal saat dibuka (pola `CaptureSheet`). Error `email_rule_invalid` → pesan di field `entity`; error lain lewat `withErrorToast`.
  - `RuleTestPanel`: props `{ input: EmailRuleInputDto; unparsed: UnparsedEmailDto[]; initialMessageId?: string }`. Langkah: Pengirim, Subjek (dilewati bila kosong), Nominal (daftar), Payee, Tanggal, lalu hasil (nominal `MoneyText`, tanggal, payee) atau alasan gagal. Baris "Cocok dengan n email di Gagal dibaca".

Tata letak (tambahkan ke `design.md` §8.10):

```
 [ Perlu ditinjau 12 | Gagal dibaca 2 | Rule 8 ]                [+ Rule baru]
 BCA Pembayaran (QRIS/VA)                               [●] ⋯
     bca.co.id · Pengeluaran · BCA · Tanpa kategori        ← kosong: warning
 Jago Terima Uang                    (Nonaktif)          [○] ⋯
     jago.com · Transfer masuk · Tanpa akun
```

Gagal dibaca: tombol `Buat rule` (outline kecil) di tiap baris dengan pengirim.

Copy kunci i18n `emailRules.*` (Indonesia; `en.ts` padanan):
- `tab` "Rule", `new` "Rule baru", `edit` "Ubah rule", `duplicate` "Duplikat", `delete` "Hapus", `disabled` "Nonaktif", `noAccount` "Tanpa akun", `noCategory` "Tanpa kategori".
- Jenis: `kind.expense` "Pengeluaran", `kind.income` "Pemasukan", `kind.transfer_out` "Transfer keluar", `kind.transfer_in` "Transfer masuk".
- Field: `name` "Nama", `sender` "Pengirim", `senderHint` "Bagian alamat email pengirim, mis. bca.co.id", `account` "Akun", `counterAccount` "Akun lawan", `category` "Kategori default", `patterns` "Pola", `subject` "Pola subjek", `amount` "Pola nominal", `payee` "Pola payee", `date` "Pola tanggal", `dateFormat` "Format tanggal", `priority` "Prioritas", `priorityHint` "Rule berprioritas lebih tinggi dicoba lebih dulu", `enabled` "Aktif", `regexHint` (K10).
- `fieldError.<field>` per field (mis. `amount` "Pola nominal tidak valid atau tanpa grup ( )").
- Uji: `test` "Uji", `testTitle` "Uji rule", `sampleUnparsed` "Email gagal dibaca", `samplePasted` "Tempel email", `pastedText` "Isi email", `stepSender` "Pengirim cocok", `stepSubject` "Subjek cocok", `stepAmount` "Nominal ditemukan", `stepDate` "Tanggal terbaca", `fail.<code>` per `TraceFailDto`, `unparsedMatches` "Cocok dengan {n} email di Gagal dibaca".
- Toast: `saved` "Rule disimpan", `savedReparsed` "Rule disimpan · {n} email dipindah ke Perlu ditinjau", `deleted` "Rule dihapus", `deleteConfirm` "Hapus rule {name}? {n} item di Perlu ditinjau tetap ada.".
- `createFromEmail` "Buat rule", `manageRules` "Kelola rule" (Settings).

- [ ] **Step 1: Tulis test** `EmailRuleSheet.test.ts` (mode browser):
  - `prefills_from_rule` dan `patterns_collapsed_when_editing`.
  - `kind_switch_toggles_category_and_counter`: transfer → field Kategori hilang, Akun lawan muncul; kembali ke expense → kategori yang salah jenis dikosongkan.
  - `invalid_regex_shows_field_error`: pola nominal `(` → pesan di bawah field Pola nominal, sheet tetap terbuka.
  - `test_panel_shows_failing_step`: sample tempel dengan pengirim lain → langkah Pengirim ✗.
  - `test_panel_shows_result`: sample cocok → nominal & tanggal tampil.
  - `save_reparses_and_toasts`.
- [ ] **Step 2: Tulis test** di `InboxView.test.ts`:
  - `rules_tab_lists_rules_with_warnings`: rule tanpa akun → "Tanpa akun".
  - `toggle_rule_enabled`.
  - `delete_rule_with_undo`.
  - `create_rule_from_unparsed`: `Buat rule` → sheet terisi pengirim/subjek dari email, panel uji memakai email itu, Simpan → baris pindah dari Gagal dibaca ke Perlu ditinjau.
  - `tab_follows_query`: `/inbox?tab=rules` membuka tab Rule.
  - `EmailSection.test.ts`: link `Kelola rule` → `/inbox?tab=rules`.
- [ ] **Step 3:** `pnpm --filter @nabungo/ui exec vitest run src/components/inbox src/views/InboxView.test.ts src/components/settings/EmailSection.test.ts` → FAIL; implementasi; → PASS. `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test` → PASS.
- [ ] **Step 4: Commit** `feat(ui): rule editor email capture`.

### Task 6: E2E, aksesibilitas, dokumentasi

**Files:**
- Create: `ui/e2e/inbox-rules-9d.spec.ts`
- Modify: `ui/e2e/a11y.spec.ts` (tab Rule + sheet rule terbuka, light + dark)
- Modify: `CLAUDE.md` (bagian email capture: rule editor di tab Inbox, re-parse lokal dari `raw_snippet`, `browser-rule.ts` port `rule.rs` + `rule-cases.json`, `email_rule_invalid`), `PRD.md` (§7 Pengaturan/Inbox, K1), `docs/plans/2026-10-12-v1-roadmap.md` (baris 9d: link plan + status), `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` (status milestone 6 + open question rule editor), `docs/design/design.md` (bila belum di Task 5)

- [ ] **Step 1: Tulis e2e** `inbox-rules-9d.spec.ts` (onboarding seperti spec lain, `connectEmail` browser, `__seedEmailInbox`):
  - `set account makes Catat appear`: capture QRIS dari rule bawaan tanpa akun (tanpa `Catat`) → tab Rule → ubah rule, pilih akun & kategori → Simpan → tab Perlu ditinjau menampilkan `Catat`.
  - `create rule from unparsed`: email gagal dibaca → `Buat rule` → `Uji` lolos → Simpan → toast "1 email dipindah" → baris ada di Perlu ditinjau → `Catat` setelah memilih kategori.
  - `invalid regex blocked`.
  - `disable rule then enable reparses`.
  - `delete rule + undo`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui e2e` → PASS (termasuk `a11y.spec.ts` 0 pelanggaran).
- [ ] **Step 3: Verifikasi akhir** (Global Constraints) → semua PASS. Cek manual `pnpm dev` dengan Gmail sungguhan: isi akun/kategori rule BCA → draft lama siap; buat rule dari satu email yang gagal dibaca → pindah ke Perlu ditinjau tanpa sync; firewall test tidak terpengaruh (tidak ada jaringan baru).
- [ ] **Step 4: Commit** `docs: fase 9d rule editor email capture`.
