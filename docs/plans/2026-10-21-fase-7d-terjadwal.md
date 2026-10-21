# Fase 7d — Transaksi Terjadwal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** TRX-06, yaitu **recurring rule** harian/mingguan/bulanan/tahunan dengan tanggal tertentu. Setiap kejadian yang jatuh tempo muncul di layar "Terjadwal" untuk dikonfirmasi, dilewati, atau diedit lalu dicatat. Per jadwal bisa diaktifkan "Catat otomatis".

**Architecture:**
- `nabungo-core`: modul baru `recurrence.rs`. Isinya subset RRULE (parse/format) dan `next_after`, dengan tanggal di akhir bulan di-clamp.
- `nabungo-storage`: repository baru `recurring.rs` untuk tabel `recurring_rule` yang sudah ada sejak `0001`. Fase ini tanpa migrasi.
- `nabungo-app` (file baru `recurring.rs`):
  - template disimpan sebagai JSON dari `TxnForm` tanpa tanggal;
  - CRUD jadwal, daftar kejadian jatuh tempo, konfirmasi/lewati/konfirmasi semua, dan auto-post.
  - Kejadian **tidak** dimaterialisasi sebagai baris `txn` berstatus `scheduled`. Kejadian dihitung dari `next_date` + aturan.
- `src-tauri`: DTO dan command. Auto-post dijalankan saat unlock dan di tick scheduler yang sudah ada.
- UI:
  - `ScheduledView` menggantikan placeholder;
  - `RuleSheet` membungkus `TxnForm` dan menambah field pengulangan;
  - menu "Jadikan berulang" di `TxnList` dan badge jumlah jatuh tempo di sidebar;
  - `browser-ledger.ts` meniru semuanya lewat port `browser-recurrence.ts`.

**Tech Stack:** Rust (tanpa dependensi baru), Vue 3 + shadcn-vue (komponen yang sudah ada: sheet, select, switch, dropdown-menu, badge), Vitest, Playwright.

**Spec:**
- `PRD.md`:
  - TRX-06 dan glosarium "Recurring rule".
  - Tabel layar "Terjadwal".
  - Skema `recurring_rule` dan `txn.recurring_rule_id`/`status`.
  - §6 invariant.
  - Tabel tech stack (`rrule`, `serde_json` untuk template).
- Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 7, baris 7d.
- Desain: `docs/design/ui-guidelines.md`:
  - §7.2 header;
  - §7.4 sidebar;
  - §8.4 sheet transaksi;
  - §8.5 baris;
  - §8.7 empty state;
  - §8.9 dialog;
  - §9.4a sebagai pola layar daftar.

**Di luar plan ini:**
- Widget dasbor "jatuh tempo", yang masuk 7e dan memakai `list_rules` dari plan ini.
- Notifikasi OS.
- Toast setelah auto-post.
- BYDAY lebih dari satu hari, COUNT, dan "hari kerja terakhir".
- Menulis ulang template saat kategori digabung. Template yang merujuk kategori/akun yang hilang atau diarsip memberi error yang jelas dan bisa diperbaiki lewat Edit (K9).
- Jadwal tidak ikut ekspor CSV/JSON. File DB dan backup tetap membawanya.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Tidak memakai crate `rrule`.** `nabungo-core::recurrence` mengimplementasi subset RRULE. Kolom `rrule` tetap berisi teks RFC 5545 yang valid:<br>• `FREQ=DAILY;INTERVAL=n`<br>• `FREQ=WEEKLY;INTERVAL=n;BYDAY=MO`<br>• `FREQ=MONTHLY;INTERVAL=n;BYMONTHDAY=d`<br>• `FREQ=YEARLY;INTERVAL=n;BYMONTH=m;BYMONTHDAY=d`<br>• opsional `;UNTIL=YYYYMMDD` di akhir<br>Parser menerima kunci dalam urutan apa pun dan menolak kunci tak dikenal. Penyimpangan ini dicatat di tabel tech stack PRD. | Crate `rrule` menarik chrono/chrono-tz. Di RFC, `BYMONTHDAY=31` **melewati** bulan pendek, sedangkan user tagihan tanggal 31 berharap tagihan jatuh di akhir bulan. Core harus bebas IO dan kecil. |
| K2 | **Anchor = tanggal pertama.** `BYDAY`/`BYMONTHDAY`/`BYMONTH` diturunkan dari tanggal mulai saat jadwal dibuat atau diubah. `next_after(d)`:<br>• harian: `d + interval` hari<br>• mingguan: `d + 7·interval` hari<br>• bulanan: bulan `d` + interval, hari = `min(BYMONTHDAY, hari terakhir bulan)`<br>• tahunan: tahun + interval, `BYMONTH`, hari di-clamp (29 Feb → 28 Feb di tahun biasa)<br>Hasil > `UNTIL` → `None`. Interval 1–99 (`MAX_INTERVAL = 99`). | Fase dari `next_date` + aturan cukup: tidak perlu kolom `dtstart`, jadi tidak perlu migrasi. Clamp bulanan selalu kembali ke tanggal 31 setelah Februari. |
| K3 | **Kejadian tidak disimpan.** `recurring_rule.next_date` = kejadian tertua yang belum ditangani. Jatuh tempo = `next_date, next_after(next_date), …` selama ≤ hari ini, maksimal `MAX_CATCH_UP = 366` per jadwal per panggilan. `txn.status` selalu `'posted'` di v1. Transaksi hasil konfirmasi diberi `recurring_rule_id`. | Query saldo/laporan/ekspor/tag/lampiran tidak perlu tahu status `scheduled`. Kejadian yang terlewat tampil satu per satu (pilihan user). |
| K4 | **Kejadian ditangani berurutan.** `confirm_occurrence(id, date, form)` dan `skip_occurrence(id, date)` mensyaratkan `date == next_date` dan jadwal aktif. Kalau tidak, hasilnya `AppError::OccurrenceStale` (kode `occurrence_stale`). Di UI hanya kejadian tertua per jadwal yang punya tombol aktif; kejadian berikutnya tampil dengan tombol nonaktif. | Model `next_date` tidak bisa menyatakan "kejadian ke-2 dicatat tapi ke-1 belum". Syarat tanggal juga mencegah klik ganda mendobel transaksi. |
| K5 | **Konfirmasi** = template dengan `date` kejadian dicatat lewat jalur `write_txn` yang sama dengan `create_txn` (validasi core, payee/tag dibuat bila perlu), lalu `next_date` maju. Semua terjadi dalam satu SQLite transaction. **Edit** = user membuka `TxnForm` yang terisi template + tanggal kejadian, dan form yang disimpan menggantikan template untuk kejadian itu saja. Tanggal transaksi boleh diubah, tetapi kejadian yang dimajukan tetap `date`. **Lewati** = hanya memajukan. Konfirmasi tunggal memberi budget alert seperti `create_txn` (`confirm_occurrence_with_alerts`). | TRX-06 "konfirmasi / lewati / edit". Template tidak berubah oleh edit satu kejadian. |
| K6 | **Konfirmasi semua** (`confirm_all_due(id, today)`): semua kejadian ≤ `today` (maksimal `MAX_CATCH_UP`) dicatat dalam **satu** SQLite transaction, all-or-nothing, dan mengembalikan jumlahnya. Tidak ada budget alert. | Mengejar beberapa bulan tagihan sekaligus. |
| K7 | **Auto-post**: jadwal dengan `auto_post = 1` menjalankan `confirm_all_due` per jadwal (satu SQLite transaction per jadwal) dari `post_auto_due(today)`. Fungsi ini dipanggil `src-tauri` setelah unlock/open (sesudah backup otomatis) dan pada setiap tick scheduler backup. Bila satu jadwal gagal (mis. akun diarsip), jadwal itu dilewati dengan log `warn` dan `next_date`-nya tidak maju, jadi kejadiannya tetap tampil untuk ditangani manual. Jadwal lain tetap diproses. | Pilihan user: "Konfirmasi + opsi otomatis". |
| K8 | **Selesai**: bila `next_after` mengembalikan `None` (melewati `UNTIL`), `ended_at = now` dan `next_date` tidak berubah. Jadwal selesai tidak punya jatuh tempo dan tampil di bawah dengan badge "Selesai". Mengubah jadwal (`update_rule`) selalu menyetel `ended_at = NULL` dan `next_date` = tanggal di form, sehingga menambah `UNTIL` menghidupkan lagi jadwal. **Hapus jadwal** = `UPDATE txn SET recurring_rule_id = NULL` lalu `DELETE` jadwal. Transaksi yang sudah tercatat tidak terpengaruh. Penghapusan memakai dialog konfirmasi tanpa undo. | Sederhana; riwayat transaksi tetap utuh. |
| K9 | **Template** = JSON `{"v":1,"kind","account_id","to_account_id","amount","splits":[{"category_id","amount","memo"}],"fee":{"amount","category_id"}|null,"payee","note","tags"}`. Struct `TemplateJson` privat di `nabungo-app` (serde) beserta konversi dari/ke `TxnForm`. Saat `create_rule`/`update_rule`, template **divalidasi** dengan tanggal mulai: dijalankan `resolve` + `ValidTxn::new` tanpa insert, sehingga akun/kategori diarsip/tidak ada ditolak sejak awal. Saat konfirmasi, aturan validasi biasa berlaku: akun diarsip → `archived`, kategori hilang → `not_found`, dan `next_date` tidak maju. | Payee/tag disimpan sebagai nama, sama seperti `TxnForm`. Error saat konfirmasi bisa diperbaiki lewat Edit kejadian atau Edit jadwal. |
| K10 | `confirm_occurrence` tidak memeriksa `date ≤ today`. Mencatat lebih awal diperbolehkan, tetapi UI hanya menampilkan tombol untuk kejadian yang jatuh tempo. | Tidak ada alasan domain untuk menolak bayar lebih awal. Yang dijaga cukup urutan (K4). |
| K11 | **Hari ini**:<br>• Rust: `nabungo_app::local_today()`; command tidak menerima `today` dari UI.<br>• browser-ledger: `todayIso()`. | Satu sumber per backend; konsisten dengan budget. |
| K12 | Error baru:<br>• `AppError::OccurrenceStale` → `occurrence_stale`<br>• `AppError::Recurrence(RecurrenceError)`: `Interval` → `recurrence_interval`, `UntilBeforeStart` → `recurrence_until`, `Invalid` → `internal`<br>• Jadwal tidak ada → `NotFound("recurring_rule")`, kode `not_found` | Pola error yang sudah ada. |
| K13 | **UI**:<br>• `ScheduledView` (`/scheduled`, item sidebar yang sudah ada) memakai `PageHeader` "Terjadwal" dengan tombol "+ Jadwal baru".<br>• Seksi **Jatuh tempo** berisi baris kejadian (tanggal, nama, nominal, akun) diurutkan menurut tanggal lalu jadwal. Kejadian tertua per jadwal punya tombol `Catat` (primary kecil), `Lewati` (ghost), dan menu `⋯` (Edit lalu catat…, Konfirmasi semua (n) bila n > 1). Kejadian lain berisi tombol nonaktif dengan tooltip "Tangani kejadian sebelumnya dulu".<br>• Seksi **Jadwal** berisi semua jadwal: nama, deskripsi pengulangan ("Tiap bulan, tgl 25"), "Berikutnya: <tanggal>", nominal, badge "Otomatis"/"Selesai", dan menu `⋯` (Ubah, Hapus).<br>• Nama tampilan = payee → nama kategori pertama → catatan → label jenis.<br>• Empty state bila belum ada jadwal.<br>• Badge angka jatuh tempo pada item sidebar "Terjadwal" (disembunyikan bila 0). | Tabel layar PRD: "Daftar recurring rule dan transaksi jatuh tempo (konfirmasi / lewati / edit)". |
| K14 | **`RuleSheet`** (sheet kanan, sama dengan `NewTxnSheet`) punya empat mode di `useUiStore().ruleEditor`:<br>• `create`: form kosong, tanggal = hari ini<br>• `edit {id}`: template + `next_date`<br>• `fromTxn {txnId}`: `duplicateOf(detail)`, tanggal = kejadian **kedua** dari `previewRecurrence({monthly,1}, txn.date, 2)`, agar transaksi asal tidak tercatat dua kali<br>• `occurrence {ruleId, date}`: template + tanggal kejadian, tanpa field pengulangan, dan menyimpan lewat `confirmOccurrence`<br>Field pengulangan (`RecurrenceFields`): Frekuensi (select), "Setiap n" (input angka 1–99), Berakhir (Tidak pernah / Pada tanggal), switch "Catat otomatis", dan baris "Berikutnya: a, b, c" dari `previewRecurrence(…, 3)`. Label tanggal di form = "Mulai" (create/fromTxn) atau "Berikutnya" (edit). | Pilihan user: buat dari layar Terjadwal + "Jadikan berulang" dari transaksi. |
| K15 | `TxnForm` mendapat prop opsional `submit?: (s: TxnFormState) => Promise<string>` (bila ada, dipakai menggantikan `store.save`), `dateLabel?: string`, dan slot `extra` yang dirender sebelum tombol aksi. Perilaku tanpa prop tidak berubah. | Satu form untuk semua alur. Validasi tetap di Rust. |
| K16 | `browser-ledger.ts` menyimpan jadwal sebagai `{ id, form (tanpa tanggal), recurrence: {freq, interval, byMonthDay, byMonth, weekday, until}, nextDate, autoPost, endedAt }`, dengan `ipc/browser-recurrence.ts` sebagai port `crates/core/src/recurrence.rs`. Ubah keduanya bersamaan; test kasus clamp yang sama ada di kedua sisi. Auto-post browser dijalankan di awal `listRules()`. Aturan error sama dengan Rust. | Pola `browser-amount.ts`. |

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`. `src-tauri` tanpa logika bisnis.
- `nabungo-core` tanpa IO, `#![deny(clippy::float_arithmetic)]`; `recurrence.rs` hanya memakai `time::Date`.
- Tidak ada migrasi baru. Migrasi `0001`–`0007` tidak diubah. Tabel `recurring_rule` dipakai apa adanya.
- Repository tulis menerima `&Transaction`; baca boleh `&Connection`. Satu use case tulis = satu SQLite transaction, kecuali `post_auto_due`, yang memakai satu transaction per jadwal.
- Invariant transaksi tetap ditegakkan oleh `ValidTxn::new`/`write_txn`. UI tidak menyusun postings.
- `ui/src/ipc/bindings.ts` hanya lewat generate (`cargo test -p nabungo-desktop export_bindings`) dan wajib di-commit.
- Kode error baru masuk `errors` di `ui/src/i18n/id.ts` **dan** `en.ts`.
- Semua teks UI lewat i18n (id + en). UI mengikuti `docs/design/ui-guidelines.md`; tambahkan §9.4b "Terjadwal" di sana.
- Offline: tidak ada aset/request remote.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`, `pnpm --filter @nabungo/ui e2e`.
- Commit diakhiri `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Klik ganda / dua konfirmasi kejadian yang sama.** Panggilan kedua harus `occurrence_stale` dan hanya satu transaksi yang tercatat. Test: Task 4 `confirm_twice_is_stale` dan Task 6 `confirmOccurrence twice → occurrence_stale`.
2. **Tagihan tanggal 31 dan 29 Februari.** Urutannya 31 Jan → 28/29 Feb → 31 Mar, tidak bergeser ke 28; 29 Feb tahunan → 28 Feb tahun biasa → 29 Feb tahun kabisat. Test: Task 1 `monthly_31_clamps_and_returns`, `yearly_feb29_clamps` dan port-nya di Task 6.
3. **App lama tidak dibuka** (jadwal harian dengan `next_date` 2 tahun lalu). Daftar dibatasi `MAX_CATCH_UP` dan tidak hang. `confirm_all_due` mencatat maksimal 366, sisanya tetap jatuh tempo. Test: Task 3 `due_list_is_capped`, Task 4 `confirm_all_caps_and_continues`.
4. **Template rusak oleh perubahan katalog** (akun diarsip, kategori digabung/hilang). Konfirmasi gagal dengan `archived`/`not_found` tanpa memajukan `next_date`. Auto-post melewati jadwal itu tetapi tetap memproses jadwal lain. Test: Task 4 `confirm_with_archived_account_keeps_next_date`, `auto_post_skips_failing_rule`.
5. **`UNTIL` tercapai.** Kejadian terakhir dicatat → jadwal selesai, tidak ada jatuh tempo, dan konfirmasi berikutnya `occurrence_stale`. Mengubah `UNTIL` menghidupkan lagi jadwal. Test: Task 4 `last_occurrence_ends_rule`, Task 3 `update_reactivates_ended_rule`.

---

### Task 1: Core — `recurrence`

**Files:**
- Create: `crates/core/src/recurrence.rs`
- Modify: `crates/core/src/lib.rs` (tambahkan `pub mod recurrence;`)

**Interfaces:**
- Produces:
  ```rust
  pub const MAX_INTERVAL: u8 = 99;
  #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Freq { Daily, Weekly, Monthly, Yearly }
  #[derive(Debug, Clone, PartialEq, Eq)] pub struct Recurrence { /* privat */ }
  #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
  pub enum RecurrenceError { Interval, UntilBeforeStart, Invalid(String) }
  impl Recurrence {
      pub fn new(freq: Freq, interval: u8, anchor: Date, until: Option<Date>) -> Result<Self, RecurrenceError>;
      pub fn parse(rrule: &str) -> Result<Self, RecurrenceError>;
      pub fn to_rrule(&self) -> String;
      pub fn freq(&self) -> Freq;
      pub fn interval(&self) -> u8;
      pub fn until(&self) -> Option<Date>;
      pub fn next_after(&self, d: Date) -> Option<Date>;
      /// `start` lalu kejadian berikutnya, maksimal `n`, berhenti di UNTIL.
      pub fn occurrences(&self, start: Date, n: usize) -> Vec<Date>;
  }
  ```

- [ ] **Step 1: Write the failing tests** (in `recurrence.rs` `mod tests`, using `time::macros::date`):
  - `rrule_round_trip`:
    - `Recurrence::new(Monthly, 1, date!(2026-01-31), None)?.to_rrule() == "FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31"`
    - weekly from Monday 2026-10-19, interval 2, until 2026-12-31 → `"FREQ=WEEKLY;INTERVAL=2;BYDAY=MO;UNTIL=20261231"`
    - yearly from 2028-02-29 → `"FREQ=YEARLY;INTERVAL=1;BYMONTH=2;BYMONTHDAY=29"`
    - `parse(to_rrule(x)) == x` for all four freqs
    - `parse("INTERVAL=1;FREQ=DAILY")` is ok
  - `parse_rejects_unknown_and_bad`: `parse("FREQ=HOURLY")`, `"FREQ=DAILY;COUNT=3"`, `"FREQ=MONTHLY;INTERVAL=1"` (no BYMONTHDAY), and `"FREQ=DAILY;INTERVAL=0"` each give `Err`.
  - `interval_bounds`: `new(Daily, 0, ..)` and `new(Daily, 100, ..)` → `Err(Interval)`; `new(Daily, 99, ..)` ok.
  - `until_before_start`: `new(Daily, 1, date!(2026-05-02), Some(date!(2026-05-01)))` → `Err(UntilBeforeStart)`; `until == anchor` ok.
  - `daily_and_weekly_steps`: daily interval 3 from 2026-12-30 → `next_after` = 2027-01-02; weekly interval 2 from 2026-10-19 → 2026-11-02.
  - `monthly_31_clamps_and_returns`: `occurrences(2026-01-31, 4) == [2026-01-31, 2026-02-28, 2026-03-31, 2026-04-30]`; anchor 2028-01-31 gives 2028-02-29 as its second occurrence; monthly interval 2 from 2026-11-30 gives 2027-01-30.
  - `yearly_feb29_clamps`: `occurrences(2028-02-29, 5) == [2028-02-29, 2029-02-28, 2030-02-28, 2031-02-28, 2032-02-29]`.
  - `until_stops`: monthly from 2026-01-15 with until 2026-03-15 → `occurrences(.., 10)` has 3 items and `next_after(2026-03-15) == None`.

- [ ] **Step 2: Run to verify fail**
  Run: `cargo test -p nabungo-core recurrence::`
  Expected: FAIL (module/items not defined).

- [ ] **Step 3: Implement** per K1/K2. Rules:
  - The anchor fields are derived in `new`: weekday for weekly; day for monthly; month+day for yearly. Daily needs none.
  - `UNTIL` format is `YYYYMMDD`.
  - `BYDAY` codes are `MO TU WE TH FR SA SU`.
  - `parse` validates field ranges: day 1–31, month 1–12, and the parse-side interval check behaves exactly like the one in `new`.
  - Use `time::util::days_in_year_month` for clamping.
  - Integer arithmetic only.

- [ ] **Step 4: Run to verify pass**
  Run: `cargo test -p nabungo-core recurrence:: && cargo clippy -p nabungo-core --all-targets -- -D warnings`
  Expected: PASS, no warnings.

- [ ] **Step 5: Commit**: `git commit -m "feat(core): subset RRULE untuk transaksi terjadwal"`

### Task 2: Storage — repository `recurring_rule`

**Files:**
- Create: `crates/storage/src/recurring.rs`
- Modify: `crates/storage/src/lib.rs` (`pub mod recurring;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct RuleRecord { pub id: Uuid, pub template: String, pub rrule: String,
                          pub next_date: Date, pub auto_post: bool, pub ended_at: Option<String> }
  pub struct RuleWrite<'a> { pub template: &'a str, pub rrule: &'a str, pub next_date: Date, pub auto_post: bool }
  pub fn insert_rule(tx: &Transaction, w: &RuleWrite) -> Result<Uuid>;          // UUIDv7
  pub fn update_rule(tx: &Transaction, id: Uuid, w: &RuleWrite) -> Result<bool>; // ended_at = NULL
  /// `next = None` → ended_at = now, next_date tidak berubah.
  pub fn advance_rule(tx: &Transaction, id: Uuid, next: Option<Date>, now: &str) -> Result<()>;
  pub fn delete_rule(tx: &Transaction, id: Uuid) -> Result<bool>;               // lepas txn.recurring_rule_id dulu
  pub fn link_txn(tx: &Transaction, txn_id: Uuid, rule_id: Uuid) -> Result<()>;
  pub fn get_rule(conn: &Connection, id: Uuid) -> Result<Option<RuleRecord>>;
  /// Aktif dulu (ended_at IS NULL), lalu next_date, lalu id.
  pub fn list_rules(conn: &Connection) -> Result<Vec<RuleRecord>>;
  ```

- [ ] **Step 1: Write the failing tests** (`open_in_memory()`):
  - `insert_get_round_trip`: all fields read back; `auto_post` is a bool; `ended_at` is `None`.
  - `advance_and_end`:
    - `advance_rule(Some(d2))` → `next_date == d2`;
    - `advance_rule(None, "2026-10-21T00:00:00Z")` → `ended_at == Some(..)` and `next_date` still `d2`;
    - `update_rule` then makes `ended_at` `None` again.
  - `list_orders_active_first`: an ended rule with an earlier `next_date` is listed after the active ones; the active ones are sorted by `next_date`.
  - `delete_unlinks_txns`: insert a txn (via the existing test helper used in `txn.rs` tests) and `link_txn` it; after `delete_rule`, the txn still exists with `recurring_rule_id IS NULL`, and `get_rule` → `None`. `delete_rule` on an unknown id → `false`.

- [ ] **Step 2: Run to verify fail**: `cargo test -p nabungo-storage recurring::` → FAIL.
- [ ] **Step 3: Implement** with `format_date`/`parse_date` and `auto_post` stored as `0/1`.
- [ ] **Step 4: Run to verify pass**: `cargo test -p nabungo-storage recurring::` → PASS.
- [ ] **Step 5: Commit**: `git commit -m "feat(storage): repository recurring_rule"`

### Task 3: App — template, CRUD jadwal, daftar jatuh tempo

**Files:**
- Create: `crates/app/src/recurring.rs`
- Modify:
  - `crates/app/src/lib.rs` (`mod recurring;` + re-export `RuleInput, RuleRow, MAX_CATCH_UP` and `nabungo_core::recurrence::{Freq, Recurrence, RecurrenceError}`)
  - `crates/app/src/ledger.rs` (`AppError::OccurrenceStale`, `AppError::Recurrence(#[from] RecurrenceError)`)
  - `crates/app/src/transactions.rs`: expose to the crate whatever the template validation in Step 3 needs — `form_input`/`resolve` as `pub(crate)` (if not already).

**Interfaces:**
- Consumes: Task 1 `Recurrence`, `Freq`; Task 2 repo functions; existing `TxnForm`, `write_txn`, `form_input`, `resolve`, `ValidTxn::new`.
- Produces:
  ```rust
  pub const MAX_CATCH_UP: usize = 366;
  pub struct RuleInput { pub form: TxnForm /* date = tanggal mulai/berikutnya */, pub freq: Freq,
                         pub interval: u8, pub until: Option<Date>, pub auto_post: bool }
  pub struct RuleRow { pub id: Uuid, pub form: TxnForm /* date = next_date */, pub freq: Freq,
                       pub interval: u8, pub until: Option<Date>, pub auto_post: bool,
                       pub ended: bool, pub due: Vec<Date> /* ≤ MAX_CATCH_UP, ≤ today */ }
  impl Ledger {
      pub fn create_rule(&mut self, input: &RuleInput) -> Result<Uuid, AppError>;
      pub fn update_rule(&mut self, id: Uuid, input: &RuleInput) -> Result<(), AppError>;
      pub fn delete_rule(&mut self, id: Uuid) -> Result<(), AppError>;
      pub fn list_rules(&self, today: Date) -> Result<Vec<RuleRow>, AppError>;
  }
  // pub(crate), dipakai Task 4:
  fn template_form(rec: &RuleRecord, date: Date) -> Result<TxnForm, AppError>;
  ```

- [ ] **Step 1: Write the failing tests** (`testkit::Kit::new(1, date!(2026-01-01))`; build the expense form via the same `TxnForm` constructor the existing transaction tests use):
  - `template_round_trip`: an expense with 2 splits, payee "PLN", tags `["rumah"]`, and a transfer with a fee both go through `create_rule` → `list_rules`, and `row.form` equals the input (date = start).
  - `create_validates_template`: `create_rule` with an archived account → `Err(Archived("account"))`; with `interval: 0` → `Err(Recurrence(Interval))`; with `until` < form date → `Err(Recurrence(UntilBeforeStart))`.
  - `due_lists_past_occurrences`: monthly from 2026-07-25, today 2026-10-21 → `due == [07-25, 08-25, 09-25]`; start 2026-11-01 → `due` is empty and `form.date == 2026-11-01`.
  - `due_list_is_capped`: daily from 2024-01-01, today 2026-10-21 → `due.len() == MAX_CATCH_UP`.
  - `update_reactivates_ended_rule`: after `advance_rule(None)` (through the storage function in the test), `update_rule` with a later `until` → `ended == false` and `due` is filled again.
  - `delete_missing_rule`: `delete_rule(Uuid::now_v7())` → `Err(NotFound("recurring_rule"))`; `update_rule` likewise.

- [ ] **Step 2: Run to verify fail**: `cargo test -p nabungo-app recurring::` → FAIL.
- [ ] **Step 3: Implement** per K2/K3/K8/K9.
  - `TemplateJson` is serde with `v: 1`.
  - Validation: `ValidTxn::new(resolve(&tx, &form_input(&input.form)?, &HashSet::new())?)?` inside a transaction that is not committed (the template only), followed by `insert_rule`/`update_rule` in the same transaction.
  - The `Recurrence` anchor is `input.form.date`.
  - `list_rules`: `ended` = `ended_at.is_some()`; `due` comes from `Recurrence::parse(rrule)` iterating `next_date` while `≤ today`.

- [ ] **Step 4: Run to verify pass**: `cargo test -p nabungo-app recurring::` → PASS.
- [ ] **Step 5: Commit**: `git commit -m "feat(app): jadwal berulang, template, daftar jatuh tempo"`

### Task 4: App — konfirmasi, lewati, konfirmasi semua, auto-post

**Files:**
- Modify: `crates/app/src/recurring.rs`, `crates/app/src/budget_alerts.rs`

**Interfaces:**
- Consumes: Task 3 `template_form`, `RuleRow`; Task 2 `advance_rule`, `link_txn`, `get_rule`.
- Produces:
  ```rust
  impl Ledger {
      /// `form = None` → template dengan `date`. Mensyaratkan jadwal aktif dan `date == next_date`.
      pub fn confirm_occurrence(&mut self, id: Uuid, date: Date, form: Option<&TxnForm>) -> Result<Uuid, AppError>;
      pub fn confirm_occurrence_with_alerts(&mut self, id: Uuid, date: Date, form: Option<&TxnForm>)
          -> Result<(Uuid, Vec<BudgetAlert>), AppError>;   // di budget_alerts.rs, periode = tanggal form/kejadian
      pub fn skip_occurrence(&mut self, id: Uuid, date: Date) -> Result<(), AppError>;
      pub fn confirm_all_due(&mut self, id: Uuid, today: Date) -> Result<u32, AppError>;
      /// Satu transaction per jadwal; jadwal yang gagal di-log dan dilewati.
      pub fn post_auto_due(&mut self, today: Date) -> u32;
  }
  ```

- [ ] **Step 1: Write the failing tests**:
  - `confirm_posts_and_advances`: monthly from 2026-09-25, confirm(09-25, None) → a txn with date 09-25, `recurring_rule_id` = rule (check via `get_txn` + a SQL query in the test), and BCA balance down by the amount; `list_rules(date!(2026-10-21))` → `due == []` and `form.date == 10-25`.
  - `confirm_twice_is_stale`: the second `confirm_occurrence(id, 09-25, None)` → `Err(OccurrenceStale)`, and `list_txns` holds a single txn for that rule.
  - `confirm_edited_form`: form with amount 175000 and date 09-27 → the txn has amount 175000 and date 09-27; the template still has its original amount; `next_date` = 10-25.
  - `skip_advances_without_txn`: skip 09-25 → no txn; `next_date` = 10-25; skip with the wrong date → `OccurrenceStale`.
  - `confirm_with_archived_account_keeps_next_date`: archive the account after the rule is created → confirm → `Err(Archived("account"))`, `next_date` unchanged, no txn.
  - `last_occurrence_ends_rule`: monthly 09-25 with until 10-25, confirm 09-25 and 10-25 → `ended == true`, `due` is empty; confirm 11-25 → `OccurrenceStale`.
  - `confirm_all_caps_and_continues`: daily from 2025-01-01, today 2026-10-21 → returns 366; `due` is still non-empty; calling it again posts the rest (sum = total days); `confirm_all_due` on a rule with no due occurrences → `Ok(0)`.
  - `confirm_all_is_atomic`: monthly with 3 occurrences due, and the category used is archived → `Err`, no txn, `next_date` unchanged.
  - `auto_post_skips_failing_rule`: two `auto_post` rules (A valid, B with an archived account) plus one manual rule C → `post_auto_due(today)` returns the number of A's occurrences; B and C still have `due`.
  - `confirm_alerts`: confirming an expense that exceeds a budget (follow the setup in the existing `budget_alerts.rs` tests) returns ≥ 1 alert.

- [ ] **Step 2: Run to verify fail**: `cargo test -p nabungo-app recurring::` → FAIL.
- [ ] **Step 3: Implement** per K4–K7/K10.
  - Inside a single transaction: `get_rule` → check active + `next_date` → `write_txn(&tx, None, &form_input(form)?, &HashSet::new())` → `link_txn` → `advance_rule(rec.next_after(date), now)`.
  - `confirm_all_due` loops inside a single transaction, at most `MAX_CATCH_UP` times.
  - `post_auto_due` uses `tracing::warn!(rule = %id, error = %e, ...)` on failure.

- [ ] **Step 4: Run to verify pass**: `cargo test -p nabungo-app` → PASS (full app suite).
- [ ] **Step 5: Commit**: `git commit -m "feat(app): konfirmasi/lewati kejadian terjadwal dan auto-post"`

### Task 5: Tauri — DTO, command, auto-post, bindings

**Files:**
- Modify:
  - `src-tauri/src/dto.rs`
  - `src-tauri/src/commands.rs`
  - `src-tauri/src/lib.rs` (`collect_commands!`)
  - `src-tauri/src/error.rs` (codes)
  - `src-tauri/src/lifecycle.rs`
  - `ui/src/ipc/bindings.ts` (generated)

**Interfaces:**
- Consumes: Task 3/4 `Ledger` methods, `RuleInput`, `RuleRow`, `Freq`, `local_today()`.
- Produces (TS names after specta camelCase):
  ```rust
  enum FreqDto { Daily, Weekly, Monthly, Yearly }                 // serde lowercase
  struct RecurrenceDto { freq: FreqDto, interval: u8, until: Option<String> }
  struct RuleInputDto { form: TxnFormDto, recurrence: RecurrenceDto, auto_post: bool }
  struct RuleDto { id: String, form: TxnFormDto /* date = next_date */, recurrence: RecurrenceDto,
                   auto_post: bool, ended: bool, due: Vec<String> }
  impl From<&TxnForm> for TxnFormDto   // kebalikan TryFrom yang sudah ada
  // commands:
  list_rules() -> Vec<RuleDto>                                    // today = local_today()
  create_rule(input: RuleInputDto) -> String
  update_rule(id: String, input: RuleInputDto) -> ()
  delete_rule(id: String) -> ()
  confirm_occurrence(id: String, date: String, form: Option<TxnFormDto>) -> SavedTxnDto
  skip_occurrence(id: String, date: String) -> ()
  confirm_all_due(id: String) -> u32
  preview_recurrence(recurrence: RecurrenceDto, start: String, count: u32) -> Vec<String>  // count ≤ 12
  ```

- [ ] **Step 1: Write the failing tests** (in `dto.rs`/`commands.rs` test modules, following the existing DTO tests):
  - `txn_form_dto_round_trip`: `TxnForm` (expense split, transfer+fee) → `TxnFormDto` → `TxnForm` gives the same value.
  - `preview_recurrence_monthly_31`: start `"2026-01-31"`, monthly 1, count 3 → `["2026-01-31","2026-02-28","2026-03-31"]`; count 50 → at most 12 items.
  - `error_codes`: `OccurrenceStale` → `"occurrence_stale"`; `Recurrence(Interval)` → `"recurrence_interval"`; `Recurrence(UntilBeforeStart)` → `"recurrence_until"`; `Recurrence(Invalid(_))` → `"internal"`.

- [ ] **Step 2: Run to verify fail**: `cargo test -p nabungo-desktop` → FAIL.
- [ ] **Step 3: Implement** the thin commands. `preview_recurrence` calls `Recurrence::new(..).occurrences(start, count.min(12))`.
  - `lifecycle.rs`: add `pub fn run_auto_post(ledger: &mut Ledger)` (calls `post_auto_due(local_today())` and logs `info` when > 0). Call it in `open_unlocked` after `run_auto_backup`; in the scheduler loop, use `slot.as_mut()` and call it after `run_auto_backup`.
  - Register the commands in `collect_commands!`.
- [ ] **Step 4: Run to verify pass and regenerate bindings**
  Run: `cargo test -p nabungo-desktop && cargo test -p nabungo-desktop export_bindings && git diff --stat ui/src/ipc/bindings.ts`
  Expected: PASS; bindings change (new types/commands).
- [ ] **Step 5: Commit**: `git commit -m "feat(desktop): command transaksi terjadwal dan auto-post"`

### Task 6: UI ipc — wrapper, `browser-recurrence`, `browser-ledger`, error i18n

**Files:**
- Create: `ui/src/ipc/browser-recurrence.ts`, `ui/src/ipc/browser-recurrence.test.ts`
- Modify:
  - `ui/src/ipc/index.ts`
  - `ui/src/ipc/browser-ledger.ts`
  - `ui/src/ipc/browser-ledger.test.ts`
  - `ui/src/i18n/id.ts`
  - `ui/src/i18n/en.ts`

**Interfaces:**
- Consumes: Task 5 bindings (`RuleDto`, `RuleInputDto`, `RecurrenceDto`, `FreqDto`, `SavedTxnDto`).
- Produces:
  - `ipc` exports: `listRules, createRule, updateRule, deleteRule, confirmOccurrence, skipOccurrence, confirmAllDue, previewRecurrence`, with the same signatures as the commands.
  - `browser-recurrence.ts`: `export function occurrences(rec: RecurrenceDto, start: string, n: number): string[]` and `export function nextAfter(rec: RecurrenceDto, anchor: string, d: string): string | null`. The anchor is stored separately in browser-ledger: the start date at create/update.

- [ ] **Step 1: Write the failing tests**
  - `browser-recurrence.test.ts` copies the Task 1 cases (monthly 31, yearly 29 Feb, interval 2, weekly, until).
  - `browser-ledger.test.ts`:
    - `createRule` + `listRules` with start 3 months back → `due.length === 3`;
    - `confirmOccurrence` twice → second rejects with code `occurrence_stale`, and `listTxns` has 1 txn;
    - `skipOccurrence` with the wrong date → `occurrence_stale`;
    - `createRule` with `interval: 0` → `recurrence_interval`;
    - `until` < start → `recurrence_until`;
    - an `autoPost` rule is posted when `listRules()` is called;
    - `deleteRule` leaves existing txns in place;
    - `confirmAllDue` returns the count.
- [ ] **Step 2: Run to verify fail**: `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-recurrence.test.ts src/ipc/browser-ledger.test.ts` → FAIL.
- [ ] **Step 3: Implement** per K16. Confirmation uses the browser-ledger's own create-txn path, so validation and errors stay identical.
  - i18n errors:
    - `occurrence_stale`: "Kejadian ini sudah ditangani. Daftar dimuat ulang." / "This occurrence was already handled."
    - `recurrence_interval`: "Interval harus 1–99." / "Interval must be 1–99."
    - `recurrence_until`: "Tanggal berakhir tidak boleh sebelum tanggal mulai." / "End date can't be before the start date."
- [ ] **Step 4: Run to verify pass**: the same command plus `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit**: `git commit -m "feat(ui): ipc dan browser-ledger transaksi terjadwal"`

### Task 7: UI — store `scheduled`, `TxnForm` submit/slot, `RuleSheet`, `RecurrenceFields`

**Files:**
- Create:
  - `ui/src/stores/scheduled.ts` + `.test.ts`
  - `ui/src/lib/recurrence.ts` + `.test.ts`
  - `ui/src/components/scheduled/RecurrenceFields.vue`
  - `ui/src/components/scheduled/RuleSheet.vue` + `RuleSheet.test.ts`
- Modify:
  - `ui/src/components/txn/TxnForm.vue` (+ `TxnForm.test.ts`)
  - `ui/src/stores/ui.ts`
  - `ui/src/App.vue`, or wherever `NewTxnSheet` is mounted (mount `RuleSheet` next to it)
  - `ui/src/i18n/id.ts`, `en.ts` (`scheduled.*`)

**Interfaces:**
- Consumes: Task 6 ipc; `fromDetail`, `duplicateOf`, `emptyForm`, `toDto` in `lib/txnForm.ts`.
- Produces:
  - `useScheduledStore()`:
    - state: `rules: RuleDto[]`, `loaded`, `dueCount` (computed: sum of `due.length` across non-ended rules);
    - actions: `reload()`, `create(state, rec, autoPost)`, `update(id, state, rec, autoPost)`, `remove(id)`, `confirm(id, date)`, `confirmEdited(id, date, state)`, `skip(id, date)`, `confirmAll(id)`.
    - Every action calls `reload()`. Actions that post transactions also call `useTransactionsStore().refresh()` and `useAccountsStore().load()`.
    - `confirm`/`confirmEdited` show budget alerts the same way `transactions.save` does.
    - An `occurrence_stale` error triggers `reload()` before it is rethrown.
  - `useUiStore()`:
    - `ruleEditor: RuleEditor | null`, where `type RuleEditor = {mode:'create'} | {mode:'edit', id} | {mode:'fromTxn', txnId} | {mode:'occurrence', ruleId, date}`;
    - `openRuleEditor(e)`, `closeRuleEditor()`.
  - `lib/recurrence.ts`:
    - `describeRecurrence(rec: RecurrenceDto, nextDate: string, t, locale): string`, e.g. "Tiap bulan, tgl 25", "Tiap 2 minggu, Senin", "Tiap tahun, 29 Feb", "Tiap hari";
    - `ruleName(rule: RuleDto, categoryName: (id) => string | undefined, t): string` per K13.
  - `TxnForm` props `submit?`, `dateLabel?`, slot `extra` (K15).

- [ ] **Step 1: Write the failing tests**
  - `TxnForm.test.ts`: with the `submit` prop, saving calls `submit` with the state and **not** `ipc.createTxn`, and emits `saved` with its return value; content in the `extra` slot is rendered; `dateLabel` replaces the date label.
  - `recurrence.test.ts`: the four description examples above, plus `ruleName` precedence (payee > category > note > kind label).
  - `scheduled.test.ts` (browser-ledger backend):
    - `dueCount` after creating a monthly rule starting on the 1st of the month two months back is 3;
    - `confirm` lowers `dueCount` by 1 and adds 1 txn to the transactions store;
    - an `occurrence_stale` from a second `confirm` with the same date reloads the store and rejects.
  - `RuleSheet.test.ts`:
    - mode `create` shows `RecurrenceFields` and saving calls `createRule`;
    - mode `fromTxn` prefills from the txn and the date = the second occurrence (txn date 2026-01-31 → 2026-02-28);
    - mode `occurrence` hides `RecurrenceFields`, and saving calls `confirmOccurrence(ruleId, date, form)`;
    - the "Berikutnya" preview shows 3 dates.
- [ ] **Step 2: Run to verify fail**: `pnpm --filter @nabungo/ui exec vitest run src/components/txn/TxnForm.test.ts src/lib/recurrence.test.ts src/stores/scheduled.test.ts src/components/scheduled` → FAIL.
- [ ] **Step 3: Implement** per K13–K15.
  - `RuleSheet` copies the open/focus/remount pattern from `NewTxnSheet.vue`.
  - Titles: "Jadwal baru" / "Ubah jadwal" / "Jadikan berulang" / "Catat kejadian".
  - `RecurrenceFields`:
    - Frekuensi uses `Select`;
    - "Setiap" uses `Input type=number min=1 max=99` with the unit label (hari/minggu/bulan/tahun);
    - Berakhir uses `Select` (never/on a date) + `DateInput`;
    - "Catat otomatis" uses `Switch` with the description "Dicatat tanpa konfirmasi saat jatuh tempo";
    - the preview uses `previewRecurrence`, debounced 200 ms; on error, the preview is hidden (validation is shown when saving).
- [ ] **Step 4: Run to verify pass**: the same command + `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit**: `git commit -m "feat(ui): sheet jadwal berulang dan store terjadwal"`

### Task 8: UI — `ScheduledView`, badge sidebar, "Jadikan berulang", panduan desain

**Files:**
- Modify:
  - `ui/src/views/ScheduledView.vue` (replace the placeholder) + create `ui/src/views/ScheduledView.test.ts`
  - `ui/src/components/AppSidebar.vue` (+ `AppSidebar.test.ts`)
  - `ui/src/components/txn/TxnList.vue` (+ `TxnList.test.ts`)
  - `ui/src/i18n/id.ts`, `en.ts`
  - `docs/design/ui-guidelines.md` (new §9.4b "Terjadwal")

**Interfaces:**
- Consumes: Task 7 `useScheduledStore`, `useUiStore().openRuleEditor`, `describeRecurrence`, `ruleName`.
- Produces: `data-testid`s for e2e:
  - `due-row`
  - `due-confirm`, `due-skip`, `due-menu`
  - `due-edit`, `due-confirm-all`
  - `rule-row`, `rule-menu-{id}`
  - `rule-new`
  - `nav-scheduled-badge`
  - `txn-menu-recurring`

- [ ] **Step 1: Write the failing tests**
  - `ScheduledView.test.ts`:
    - with one monthly rule starting on the 1st of the month two months back (3 due), there are 3 `due-row` rows sorted by date. Only the first has enabled `due-confirm`/`due-skip`; the other two are `disabled`.
    - clicking `due-confirm` → 2 rows remain.
    - `due-confirm-all` in the menu reads "Konfirmasi semua (2)" → 0 rows remain and the "Jatuh tempo" section disappears.
    - `rule-row` shows `describeRecurrence` and the badge "Otomatis" for an `autoPost` rule.
    - Hapus → a `ConfirmDialog`; after confirming, the rule is gone.
    - with no rules → `EmptyState`.
  - `AppSidebar.test.ts`: with `dueCount` 3, `nav-scheduled-badge` shows "3"; with 0, the badge is absent.
  - `TxnList.test.ts`: `txn-menu-recurring` calls `openRuleEditor({mode:'fromTxn', txnId})` for income/expense/transfer, and is `disabled` for opening/adjustment.
- [ ] **Step 2: Run to verify fail**: `pnpm --filter @nabungo/ui exec vitest run src/views/ScheduledView.test.ts src/components/AppSidebar.test.ts src/components/txn/TxnList.test.ts` → FAIL.
- [ ] **Step 3: Implement** per K13.
  - Rows are 48px, matching `LabelsView`.
  - Due rows: amount via `MoneyText`, date via `formatDateInput`, and per-row `withErrorToast`.
  - "Edit lalu catat…" → `openRuleEditor({mode:'occurrence', ruleId, date})`.
  - The sidebar loads `useScheduledStore().reload()` on mount (via `withErrorToast`) and renders `Badge` beside the "Terjadwal" label.
  - `ScheduledView` always reloads on mount.
  - Document the pattern in §9.4b (sections, row actions, order of handling occurrences).
- [ ] **Step 4: Run to verify pass**: `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit**: `git commit -m "feat(ui): layar Terjadwal, badge jatuh tempo, jadikan berulang"`

### Task 9: E2E, docs, verifikasi akhir

**Files:**
- Create: `ui/e2e/scheduled-7d.spec.ts`
- Modify:
  - `docs/plans/2026-10-12-v1-roadmap.md` (7d row: link to this plan + status done)
  - `PRD.md`: tech stack table row `rrule` → "subset RRULE di `nabungo-core::recurrence` (tanpa crate; clamp akhir bulan, lihat plan 7d K1)"
  - `CLAUDE.md`: one line under "Model data" about recurring rules (K3: occurrences are not materialized; `txn.status` is always `posted`) and the port `browser-recurrence.ts` ↔ `recurrence.rs`

- [ ] **Step 1: Write the e2e** (follow the setup/onboarding in `labels-7c.spec.ts`):
  - `create rule with past start, confirm/skip/confirm all`: "+ Jadwal baru" → expense 150.000 "Listrik", monthly, start = the 1st of the month 3 months ago (computed in the test) → 4 `due-row` rows → confirm the first → skip the second → "Konfirmasi semua (2)" → no `due-row` remains; the Transaksi page shows 3 "Listrik" transactions; the sidebar badge disappears.
  - `jadikan berulang from transaction`: create a transaction today → menu `txn-menu-recurring` → save → Terjadwal shows a `rule-row` with "Berikutnya" = next month, and there are 0 `due-row` rows.
  - `edit occurrence before posting`: rule start = this month's 1st (due) → "Edit lalu catat…" → change the amount to 175.000 → save → the transaction has 175.000; the `rule-row` amount is still 150.000.
- [ ] **Step 2: Run**: `pnpm --filter @nabungo/ui e2e` → all PASS (12 old + 3 new).
- [ ] **Step 3: Update docs** per the Files list.
- [ ] **Step 4: Full verification**:
  Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && git diff --exit-code ui/src/ipc/bindings.ts`
  Expected: everything green; bindings have no diff.
- [ ] **Step 5: Commit**: `git commit -m "test(e2e): transaksi terjadwal; docs: Fase 7d selesai"`
