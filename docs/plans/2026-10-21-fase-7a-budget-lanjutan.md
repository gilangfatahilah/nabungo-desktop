# Fase 7a — Budget Lanjutan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:**
- BUD-02: layar Budget menampilkan **sisa per hari** selama periode berjalan (budget vs aktual dan progress bar sudah ada dari Fase 5).
- BUD-03: **rollover** per kategori — sisa (atau kelebihan) periode ini dibawa ke periode berikutnya.
- BUD-04: **peringatan in-app** saat realisasi kategori ber-budget mencapai ≥ 80% dan ≥ 100%: toast setelah transaksi disimpan, ringkasan dan warna bar di layar Budget.

**Architecture:**
- `nabungo-core/budget.rs` mendapat aturan murni: level budget (`Ok`/`Warning`/`Exceeded`), persen terpakai, sisa hari, sisa per hari, rantai rollover. Tidak ada float: persen dihitung dengan integer `i128`.
- `nabungo-storage/budget.rs` membaca/menulis kolom `budget.rollover` yang **sudah ada** sejak `0001_init.sql` — tidak ada migrasi baru.
- `nabungo-app/budget.rs`: `budget_overview(period, today)` mengisi `carry`, `level`, `per_day`, `days_left`; use case `set_budget_rollover`; `copy_budgets` ikut menyalin flag. File baru `nabungo-app/budget_alerts.rs`: `create_txn_with_alerts` / `update_txn_with_alerts` membandingkan level sebelum dan sesudah tulis.
- `src-tauri`: DTO dan command tipis. `budget_overview` memakai `nabungo_app::local_today()`; `create_txn`/`update_txn` mengembalikan alert.
- UI: kolom Bawaan & Per hari, menu baris untuk rollover, ringkasan peringatan, `BudgetBar` membaca level dari backend, toast alert setelah simpan transaksi. `browser-ledger.ts` mencerminkan semua aturan.

**Tech Stack:** Rust (crate yang sudah ada, tanpa dependensi baru), Vue 3 + shadcn-vue (`DropdownMenu` sudah ada di `ui/src/components/ui/`), Vitest, Playwright.

**Spec:** `PRD.md` §5 (BUD-02, BUD-03, BUD-04; acceptance "Transfer tidak memengaruhi budget", "Periode budget custom"), §6 (tabel `budget`, kolom `rollover`). Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 7. Desain: `docs/design/ui-guidelines.md` §4.2 (warna Waspada/Bahaya), §9.5 (Budget).

**Di luar plan ini:** RPT-01 "budget yang hampir habis" di dasbor memakai `level` dari plan ini, tapi dikerjakan di Fase 7e.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | Flag `rollover` disimpan di baris budget periode **P** dan berarti "sisa P dibawa ke P+1". Toggle hanya tersedia untuk kategori yang punya budget di P. Menghapus budget P ikut menghapus flag-nya. | Kolom sudah ada di skema per baris `(category, period)`; tidak butuh migrasi. |
| K2 | `carry(P)` = Σ `amount` rantai − realisasi rantai. Rantai = periode berurutan mundur dari P−1 yang **punya baris budget dengan `rollover = 1`**; berhenti di periode pertama tanpa baris atau tanpa flag. Realisasi rantai dihitung **satu query rentang** `[from(awal rantai), to(P−1)]` (penjumlahan realisasi aditif atas periode yang bersambung). | Rantai bisa panjang (bertahun-tahun) tanpa N query. |
| K3 | Carry bisa negatif (kelebihan mengurangi budget berikutnya). `available = amount + carry`. Carry hanya ditampilkan/dipakai untuk kategori yang punya budget di P; tanpa budget di P, rantai ke P+1 putus. | PRD: "rollover sisa/kelebihan". |
| K4 | `copy_budgets` menyalin flag `rollover` bersama nominal. | Rollover praktisnya setelan kategori; user menyalin budget tiap bulan. |
| K5 | Level: `available > 0` → `Exceeded` bila `spent ≥ available`, `Warning` bila `spent × 100 ≥ available × 80`, selain itu `Ok`. `available ≤ 0` → `Exceeded` bila `spent > 0` atau `available < 0`, selain itu `Ok`. Ambang **≥** sesuai PRD; `BudgetBar` dan pedoman §9.5/§4.2 diubah dari "> 80%" menjadi "≥ 80%" dan warnanya mengikuti `level` dari backend. | Satu sumber aturan; PRD menyebut ≥. |
| K6 | `percent` = `floor(max(spent,0) × 100 / available)` bila `available > 0`, selain itu `None`. | Integer, tanpa float di core. |
| K7 | `days_left` = jumlah hari dari `today` sampai hari terakhir periode, inklusif, hanya bila `today` di dalam periode; selain itu `None` dan kolom Per hari disembunyikan. `per_day = floor(max(remaining, 0) / days_left)`. Total juga punya `per_day`. | Sisa per hari hanya bermakna untuk periode berjalan. |
| K8 | Alert = baris ber-budget (di periode transaksi lama dan baru) yang levelnya **naik** dan level akhirnya bukan `Ok`. Hanya dari create/update (duplikat = create). Hapus/undo tidak memberi alert. Level dibaca sebelum dan sesudah tulis di bawah `Mutex<Ledger>` yang sama, jadi konsisten tanpa ikut di SQLite transaction tulis. | Peringatan sekali saat melewati ambang, tidak berulang di setiap belanja kecil. |
| K9 | Tanggal "hari ini" untuk `budget_overview` dibaca di `src-tauri` dengan `nabungo_app::local_today()` (pola yang sama dengan backup/ekspor) dan diteruskan sebagai parameter; `nabungo-app` tidak membaca jam. | Bisa dites. |
| K10 | Ekspor JSON/CSV budget menambah field `rollover` (bool / `0`/`1`). | DAT-02 harus memuat data user lengkap. |
| K11 | `Ledger::create_txn`/`update_txn` tidak berubah signature-nya; varian `_with_alerts` membungkusnya. | `create_txn` dipakai ±20 test dan testkit. |

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`. `src-tauri` tanpa logika bisnis.
- `nabungo-core` tanpa IO dan tanpa float (`#![deny(clippy::float_arithmetic)]`). Nominal `i64`; perkalian persen lewat `i128`.
- Migrasi `0001`–`0005` tidak diubah; plan ini tidak menambah migrasi.
- Repository tulis menerima `&Transaction`; baca boleh `&Connection`. Satu use case tulis = satu SQLite transaction.
- Realisasi selalu diturunkan dari `posting` (transaksi aktif, `posted`). Transfer tidak memengaruhi budget, carry, level, maupun alert.
- `browser-ledger.ts` harus memberi hasil yang sama dengan Rust untuk semua aturan K1–K8.
- Kode error baru (bila ada) ditambahkan ke `errors` di `ui/src/i18n/id.ts` dan `en.ts`. Plan ini hanya memakai `not_found` (entity `budget`) yang sudah ada.
- `ui/src/ipc/bindings.ts` di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- UI mengikuti `docs/design/ui-guidelines.md`: angka `num` rata kanan, sisa negatif warna Bahaya dengan tanda `−`.
- Verifikasi akhir per task Rust: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`.

## Review Focus

1. **Rantai rollover dengan celah** (Agustus rollover, September tanpa budget, Oktober ber-budget) → carry Oktober = 0. Test: Task 1 `rollover_chain_stops_at_gap`, Task 3 `carry_resets_after_period_without_budget`.
2. **Budget 0 atau available negatif karena carry** → level `Exceeded` (kecuali tidak ada realisasi pada budget 0), `percent = None`, per hari 0, bukan panic/divide-by-zero. Test: Task 1 `level_table`, `per_day_never_negative`; Task 3 `negative_carry_makes_available_negative`.
3. **Edit transaksi memindahkan tanggal ke periode lain** → alert dihitung di periode baru; periode lama yang turun tidak memberi alert. Test: Task 4 `update_moving_period_alerts_new_period_only`.
4. **Melihat periode lalu/depan** → `days_left = None`, kolom Per hari tidak tampil. Test: Task 3 `per_day_only_for_current_period`, Task 7 `kolom per hari hanya untuk periode berjalan`.
5. **Start day 25** → `days_left` dihitung ke tanggal 24 dan rentang realisasi rantai memakai batas periode, bukan bulan kalender. Test: Task 1 `days_left_inclusive_to_period_end`, Task 3 `carry_uses_custom_period_boundaries`.

---

### Task 1: Aturan budget murni di core

**Files:**
- Modify: `crates/core/src/budget.rs`, `crates/core/src/lib.rs` (re-export)

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)] pub enum BudgetLevel { Ok, Warning, Exceeded }` (urutan derive dipakai K8)
  - `pub fn budget_level(available: i64, spent: i64) -> BudgetLevel` (K5)
  - `pub fn spent_percent(available: i64, spent: i64) -> Option<u32>` (K6)
  - `pub fn days_left(today: Date, from: Date, last_day: Date) -> Option<u32>` (K7)
  - `pub fn per_day(remaining: i64, days_left: u32) -> i64` (`days_left ≥ 1`)
  - `pub struct BudgetEntry { pub amount: i64, pub rollover: bool }`
  - `pub fn rollover_chain(entries: &BTreeMap<BudgetPeriod, BudgetEntry>, period: BudgetPeriod) -> Result<Option<(BudgetPeriod, i64)>, MoneyError>` — awal rantai dan Σ amount (K2); `Ok(None)` bila P−1 tidak ada atau tanpa flag; Σ memakai `checked_add`, overflow → `Err`.
  - `BudgetLine` mendapat field `pub carry: i64`; `BudgetTotals` mendapat `pub carry: i64`. `budget_totals` menjumlah carry dari baris yang sama dengan `budget`.

- [ ] **Step 1: Tulis test yang gagal** di modul `tests` `crates/core/src/budget.rs`:
  - `level_table`: `(available, spent) → level` untuk `(100,0)→Ok`, `(100,79)→Ok`, `(100,80)→Warning`, `(100,99)→Warning`, `(100,100)→Exceeded`, `(100,150)→Exceeded`, `(0,0)→Ok`, `(0,1)→Exceeded`, `(-50,0)→Exceeded`, `(100,-20)→Ok`, `(MAX_AMOUNT, MAX_AMOUNT)→Exceeded` (tanpa overflow).
  - `percent_table`: `(100,85)→Some(85)`, `(300,100)→Some(33)`, `(100,-5)→Some(0)`, `(0,10)→None`, `(-1,0)→None`, `(100,250)→Some(250)`.
  - `days_left_inclusive_to_period_end`: periode start 25 `2026-11` (`from=2026-10-25`, `last=2026-11-24`): today `2026-11-24`→`Some(1)`, `2026-10-25`→`Some(31)`, `2026-11-25`→`None`, `2026-10-24`→`None`.
  - `per_day_never_negative`: `per_day(310_000, 31) == 10_000`, `per_day(100, 3) == 33`, `per_day(-5_000, 4) == 0`.
  - `rollover_chain_*`: dengan entri Aug(100, rollover), Sep(200, rollover), Okt(300, no) → `rollover_chain(.., Okt) == Ok(Some((Aug, 300)))`, `rollover_chain(.., Nov) == Ok(None)` (Okt tanpa flag); `rollover_chain_stops_at_gap`: Aug(100, rollover), Sep tidak ada → `rollover_chain(.., Okt) == Ok(None)` dan Jul(50, rollover)+Aug(100, rollover)+Sep hilang tetap `None` untuk Okt.
  - Update test `totals_*` yang ada: `line()` mengisi `carry`; tambah `totals_include_carry_of_counted_rows` (parent ber-budget dengan carry 20, child ber-budget carry 5 → `carry == 20`).
- [ ] **Step 2:** `cargo test -p nabungo-core budget::` → FAIL (simbol belum ada).
- [ ] **Step 3: Implementasi** sesuai Interfaces. Perbandingan K5 lewat `i128` (`spent as i128 * 100 >= available as i128 * 80`).
- [ ] **Step 4:** `cargo test -p nabungo-core budget::` → PASS; fmt + clippy bersih.
- [ ] **Step 5: Commit** `feat(core): level budget, sisa per hari, rantai rollover`

### Task 2: Storage — kolom rollover

**Files:**
- Modify: `crates/storage/src/budget.rs`

**Interfaces:**
- Produces:
  - `BudgetRow` mendapat `pub rollover: bool`; `list_budgets` dan `list_all_budgets` mengisinya.
  - `pub fn set_rollover(tx: &Transaction, category_id: Uuid, period: &str, on: bool) -> Result<bool>` — `false` bila baris tidak ada.
  - `pub fn list_budgets_before(conn: &Connection, period: &str) -> Result<Vec<BudgetRow>>` — semua baris dengan `period < ?1`, urut kategori lalu periode.
  - `upsert_budget` tidak berubah: update hanya `amount`, flag lama dipertahankan; baris baru `rollover = 0`.

- [ ] **Step 1: Test gagal** `rollover_flag_survives_amount_update_and_list_before`: upsert Okt 100 → `set_rollover(Okt, true) == true` → upsert Okt 200 → `list_budgets("2026-10")[0].rollover == true`; `set_rollover` pada Nov tanpa baris → `false`; `list_budgets_before("2026-11")` berisi Okt saja; `delete_budget` lalu upsert ulang → `rollover == false`.
- [ ] **Step 2:** `cargo test -p nabungo-storage budget::` → FAIL.
- [ ] **Step 3: Implementasi.** Perbaiki pemanggil `BudgetRow { .. }` lain bila kompilasi menuntut (export).
- [ ] **Step 4:** `cargo test -p nabungo-storage` → PASS.
- [ ] **Step 5: Commit** `feat(storage): baca/tulis flag rollover budget`

### Task 3: App — overview dengan carry, level, sisa per hari; set rollover; salin flag; ekspor

**Files:**
- Modify: `crates/app/src/budget.rs`, `crates/app/src/export.rs`, `crates/app/src/lib.rs` (re-export `BudgetLevel`)

**Interfaces:**
- Consumes: Task 1 (`budget_level`, `spent_percent`, `days_left`, `per_day`, `rollover_chain`, `BudgetEntry`), Task 2 (`set_rollover`, `list_budgets_before`, `BudgetRow.rollover`).
- Produces:
  - `pub fn budget_overview(&self, period: BudgetPeriod, today: Date) -> Result<BudgetOverview, AppError>`
  - `BudgetRow` (app) mendapat: `pub rollover: bool`, `pub carry: i64` (0 bila tanpa budget/rantai), `pub level: Option<BudgetLevel>` (`None` bila tanpa budget), `pub percent: Option<u32>`, `pub per_day: Option<i64>` (`None` bila tanpa budget atau `days_left` None). `available` = `budget + carry`, `remaining` = `available − spent` dihitung di DTO seperti sekarang.
  - `BudgetOverview` mendapat `pub days_left: Option<u32>`; `totals` (dari core) membawa `carry`; tambah `pub total_per_day: Option<i64>`.
  - `pub(crate) fn overview_on(conn: &Connection, start: PeriodStartDay, period: BudgetPeriod, today: Option<Date>) -> Result<BudgetOverview, AppError>` — dipakai Task 4 (di dalam/luar transaction).
  - `pub fn set_budget_rollover(&mut self, category_id: Uuid, period: BudgetPeriod, on: bool) -> Result<(), AppError>` — aturan kategori sama dengan `set_budget` (menyalakan butuh kategori aktif & expense; mematikan boleh untuk arsip); tidak ada baris budget → `AppError::NotFound("budget")`.
- Carry per kategori: kelompokkan kategori ber-budget di P menurut awal rantai; untuk tiap awal rantai unik jalankan satu `report::category_totals(from(awal), from(P))` + `rollup_spent`.

- [ ] **Step 1: Test gagal** di `crates/app/src/budget.rs` (pakai `Kit`; ubah semua pemanggilan lama `budget_overview(p)` menjadi `budget_overview(p, <tanggal di luar periode>)` kecuali yang menguji per hari):
  - `carry_chains_over_consecutive_rollover_periods`: start 1; Makan Agu 500k rollover, Sep 500k rollover, Okt 500k; belanja Agu 300k, Sep 600k → Okt `carry == 100_000` (1.000k − 900k), `available` lewat DTO tidak dites di sini; `row.budget == Some(500_000)`.
  - `carry_resets_after_period_without_budget` (Review Focus 1).
  - `negative_carry_makes_available_negative`: Sep 100k rollover, belanja Sep 400k, Okt budget 100k tanpa belanja → `carry == -300_000`, `level == Some(Exceeded)`, `percent == None`, `per_day == Some(0)` saat today di Okt.
  - `carry_uses_custom_period_boundaries` (start 25): belanja tanggal 24 Sep masuk periode Sep, 25 Sep masuk Okt; rantai Sep→Okt menghitung hanya yang tepat.
  - `transfer_does_not_affect_carry`: transfer tanpa biaya di periode rantai tidak mengubah `carry` (biaya transfer berkategori expense tetap dihitung seperti belanja biasa).
  - `per_day_only_for_current_period`: today di Okt → `days_left == Some(n)`, `per_day == Some(floor(remaining/n))`, `total_per_day` terisi; overview Sep dengan today di Okt → `days_left == None`, semua `per_day == None`.
  - `level_and_percent_follow_spending`: budget 100k, belanja 85k → `Warning`, `Some(85)`.
  - `set_budget_rollover_rules`: tanpa budget → `NotFound("budget")`; kategori income → `Budget(IncomeCategory)`; nyala lalu `budget_overview` periode itu `rollover == true`; arsip kategori lalu matikan → OK, nyalakan → `Archived("category")`.
  - Perluas `copy_budgets_fills_only_missing`: flag rollover Okt ikut tersalin ke Nov.
  - Di `export.rs`: test ekspor yang ada memeriksa field `rollover` di JSON dan kolom `rollover` di `budgets.csv`.
- [ ] **Step 2:** `cargo test -p nabungo-app budget:: export::` → FAIL.
- [ ] **Step 3: Implementasi.** Pertahankan aturan yang ada (urutan pohon, baris arsip, total K2 Fase 5).
- [ ] **Step 4:** `cargo test -p nabungo-app` → PASS (termasuk `transfer_does_not_affect_budget`, `custom_start_day_period_boundaries` yang lama).
- [ ] **Step 5: Commit** `feat(app): rollover, level, dan sisa per hari di overview budget`

### Task 4: App — alert budget saat simpan transaksi

**Files:**
- Create: `crates/app/src/budget_alerts.rs`
- Modify: `crates/app/src/lib.rs` (mod + re-export `BudgetAlert`)

**Interfaces:**
- Consumes: `overview_on` (Task 3), `Ledger::create_txn`, `Ledger::update_txn`, `Ledger::get_txn`, `Ledger::period_of`.
- Produces:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct BudgetAlert { pub category_id: Uuid, pub period: BudgetPeriod, pub level: BudgetLevel, pub percent: Option<u32> }`
  - `pub fn create_txn_with_alerts(&mut self, form: &TxnForm) -> Result<(Uuid, Vec<BudgetAlert>), AppError>`
  - `pub fn update_txn_with_alerts(&mut self, id: Uuid, form: &TxnForm) -> Result<Vec<BudgetAlert>, AppError>` — periode yang diperiksa: periode tanggal lama (dari `get_txn`) ∪ periode `form.date`.
  - Urutan alert = urutan baris overview (pohon), per periode urut naik.

- [ ] **Step 1: Test gagal** di `budget_alerts.rs` (pakai `Kit`, budget Makan & Minum 100k Okt):
  - `crossing_80_alerts_once`: belanja 70k → tidak ada alert; belanja 15k → satu alert `Warning`, `percent Some(85)`; belanja 5k → tidak ada alert (tetap Warning).
  - `crossing_100_from_warning_alerts_exceeded`: dari 85k, belanja 20k → `Exceeded`, `Some(105)`.
  - `jump_straight_to_exceeded_gives_one_alert`: belanja 120k sekaligus → satu alert `Exceeded`.
  - `child_and_parent_both_alert`: Makan & Minum 60k dan anaknya Restoran 50k ber-budget; belanja Restoran 50k → dua alert berurutan pohon: Makan & Minum `Warning` `Some(83)`, Restoran `Exceeded` `Some(100)`.
  - `unbudgeted_and_transfer_never_alert`: belanja kategori tanpa budget dan transfer → `vec![]`.
  - `update_moving_period_alerts_new_period_only` (Review Focus 3): budget Okt & Nov 100k; transaksi 90k di Okt (alert Okt); edit tanggal ke Nov → alert hanya `period == Nov`.
  - `failed_save_returns_error_without_side_effect`: form tidak valid → `Err`, overview tidak berubah.
- [ ] **Step 2:** `cargo test -p nabungo-app budget_alerts::` → FAIL.
- [ ] **Step 3: Implementasi** — snapshot `HashMap<(Uuid, BudgetPeriod), BudgetLevel>` dari `overview_on(.., None)` per periode sebelum dan sesudah; alert untuk `after > before && after != Ok`.
- [ ] **Step 4:** `cargo test -p nabungo-app` → PASS.
- [ ] **Step 5: Commit** `feat(app): peringatan budget 80%/100% saat transaksi disimpan`

### Task 5: Desktop — DTO dan command

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (`collect_commands!`), `ui/src/ipc/bindings.ts` (generate)

**Interfaces:**
- Produces (TS lewat specta, camelCase):
  - `BudgetLevelDto` = `'ok' | 'warning' | 'exceeded'` (pola enum `CategoryKindDto`).
  - `BudgetRowDto` + `rollover: boolean`, `carry: number`, `available: number | null`, `level: BudgetLevelDto | null`, `percent: number | null`, `perDay: number | null`; `remaining = available − spent`.
  - `BudgetTotalDto` + `carry`, `available`, `perDay: number | null`; `remaining = available − spent`.
  - `BudgetOverviewDto` + `daysLeft: number | null`.
  - `BudgetAlertDto { categoryId: string, period: string, level: BudgetLevelDto, percent: number | null }`.
  - `SavedTxnDto { id: string, alerts: BudgetAlertDto[] }`.
  - Command: `budget_overview(period)` (tanpa parameter tanggal; pakai `local_today()`), `set_budget_rollover(category_id, period, on: bool) -> ()`, `create_txn(form) -> SavedTxnDto`, `update_txn(id, form) -> Vec<BudgetAlertDto>`.

- [ ] **Step 1: Test gagal** di `dto.rs`: perbarui `budget_overview_dto_has_neighbours_and_remaining` — baris dengan `budget 100, carry -30, spent 50` → `available Some(70)`, `remaining Some(20)`; total `available`/`remaining` ikut carry; `budget_level_serializes_snake_case` (`Exceeded` → `"exceeded"`); `budget_alert_dto_maps_period_key`.
- [ ] **Step 2:** `cargo test -p nabungo-desktop` → FAIL.
- [ ] **Step 3: Implementasi** DTO + command; daftarkan `set_budget_rollover`.
- [ ] **Step 4:** `cargo test -p nabungo-desktop` → PASS; `cargo test -p nabungo-desktop export_bindings` lalu pastikan `bindings.ts` berubah sesuai (UI belum dikompilasi ulang di task ini; typecheck dijalankan di Task 6).
- [ ] **Step 5: Commit** `feat(desktop): command rollover dan alert budget`

### Task 6: IPC facade dan browser-ledger

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser-ledger.ts`, `ui/src/ipc/browser-ledger.test.ts`, `ui/src/stores/transactions.ts` (sesuaikan tipe kembalian `createTxn`/`updateTxn` saja; toast di Task 8)

**Interfaces:**
- Produces:
  - `createTxn(form): Promise<SavedTxnDto>`, `updateTxn(id, form): Promise<BudgetAlertDto[]>`, `setBudgetRollover(categoryId, period, on): Promise<void>`; re-export tipe baru.
  - `state.budgets` item mendapat `rollover: boolean`; `budgetOverview` memakai `todayIso()` untuk `daysLeft`.
  - `transactions` store `save()` tetap mengembalikan `string` (id) dan menyimpan alert terakhir di `lastAlerts` sementara (dipakai Task 8).

- [ ] **Step 1: Test gagal** di `describe('browser ledger budget (paritas Rust)')` — cerminan test Task 3/4 dengan nama yang sama: `carry berantai`, `carry reset setelah periode tanpa budget`, `carry negatif → exceeded tanpa percent`, `per hari hanya periode berjalan`, `setBudgetRollover tanpa budget → not_found`, `salin budget ikut flag rollover`, `alert sekali saat melewati 80% lalu 100%`, `edit pindah periode → alert periode baru saja`, `transfer tidak memberi alert`. Pakai `vi.setSystemTime` untuk hari ini.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts` → FAIL.
- [ ] **Step 3: Implementasi** (port aturan K1–K8; level/percent pakai integer, `Math.floor`).
- [ ] **Step 4:** test file itu PASS; `pnpm --filter @nabungo/ui typecheck` PASS.
- [ ] **Step 5: Commit** `feat(ui): IPC rollover dan alert budget, paritas browser-ledger`

### Task 7: Layar Budget

**Files:**
- Modify: `ui/src/components/BudgetBar.vue`, `ui/src/components/BudgetBar.test.ts`, `ui/src/views/BudgetView.vue`, `ui/src/stores/budget.ts`, `ui/src/stores/budget.test.ts`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/ui-guidelines.md` (§4.2 baris Waspada "budget ≥ 80%", §9.5 sketsa + aturan kolom)
- Create: `ui/src/views/BudgetView.test.ts`

**Interfaces:**
- Consumes: DTO Task 5, `setBudgetRollover` Task 6.
- Produces:
  - `BudgetBar` props `{ spent: number; budget: number; level: BudgetLevelDto }` — `data-state` = `ok`/`warn`/`over` dari `level`, lebar isian tetap dari persen.
  - Store budget: `setRollover(categoryId: string, on: boolean)` (pola `then()`).
  - Kolom: Kategori | Budget | Bawaan | Terpakai | Sisa | Per hari (hanya bila `daysLeft !== null`) | bar + persen. Bawaan menampilkan `—` bila `carry === 0` dan baris periode lalu tanpa rollover; selain itu nominal bertanda.
  - Menu baris (ikon `Ellipsis`, `DropdownMenuCheckboxItem` "Bawa sisa ke periode berikutnya") hanya pada baris ber-budget.
  - Ringkasan di bawah header bila ada level non-ok: teks Waspada/Bahaya dengan ikon (bukan warna saja).
  - i18n `budget.*` baru: `carry`, `perDay`, `rollover`, `rowMenu` (`'Aksi untuk {name}'`), `summaryWarning` (`'{n} kategori hampir habis'`), `summaryExceeded` (`'{n} kategori terlampaui'`), `alertWarning` (`'Budget {name} sudah {percent}% terpakai'`), `alertExceeded` (`'Budget {name} terlampaui ({percent}%)'`), `alertExceededNoPercent` (`'Budget {name} terlampaui'`). EN setara.

- [ ] **Step 1: Test gagal:**
  - `BudgetBar.test.ts`: ganti ekspektasi ke prop `level` (`warning` → `data-state="warn"` walau 80% tepat; `exceeded` pada 100% tepat → `over`).
  - `BudgetView.test.ts` (browser-ledger + `vi.setSystemTime`): `kolom per hari hanya untuk periode berjalan` (pindah ke periode berikutnya → header Per hari hilang); `rollover lewat menu baris menampilkan bawaan di periode berikutnya`; `ringkasan menghitung kategori hampir habis dan terlampaui`; `sisa negatif berwarna destructive dengan tanda minus`.
  - `budget.test.ts`: `setRollover memuat ulang overview`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui exec vitest run src/components/BudgetBar.test.ts src/views/BudgetView.test.ts src/stores/budget.test.ts` → FAIL.
- [ ] **Step 3: Implementasi** + update pedoman.
- [ ] **Step 4:** test di atas PASS; `pnpm --filter @nabungo/ui typecheck` PASS.
- [ ] **Step 5: Commit** `feat(ui): kolom bawaan & per hari, rollover, ringkasan peringatan budget`

### Task 8: Toast alert setelah simpan transaksi + E2E

**Files:**
- Create: `ui/src/lib/budgetAlerts.ts`, `ui/src/lib/budgetAlerts.test.ts`
- Modify: `ui/src/stores/transactions.ts`, `ui/src/stores/transactions.test.ts`, `ui/e2e/budget-allocation.spec.ts`

**Interfaces:**
- Consumes: `BudgetAlertDto`, i18n `budget.alert*` (Task 7), store kategori (nama).
- Produces: `export function showBudgetAlerts(alerts: BudgetAlertDto[], names: Map<string, string>): void` — `warning` → `toast.warning`, `exceeded` → `toast.error`; dipanggil `transactions.save()` setelah `refresh()`; hapus `lastAlerts` sementara dari Task 6.

- [ ] **Step 1: Test gagal:**
  - `budgetAlerts.test.ts`: mock `vue-sonner`; satu alert warning 85% → `toast.warning('Budget Makan & Minum sudah 85% terpakai')`; exceeded tanpa percent → `toast.error('Budget Makan & Minum terlampaui')`; daftar kosong → tidak ada toast.
  - `transactions.test.ts`: `save memunculkan alert budget yang melewati ambang` (browser-ledger, budget 100k, simpan 85k → `toast.warning` dipanggil sekali).
  - E2E `budget alert and rollover`: set budget Makan & Minum, catat expense 90% lewat form → toast terlihat; nyalakan rollover; periode berikutnya menampilkan Bawaan.
- [ ] **Step 2:** vitest file terkait → FAIL; `pnpm --filter @nabungo/ui e2e` → test baru FAIL.
- [ ] **Step 3: Implementasi.**
- [ ] **Step 4:** `pnpm --filter @nabungo/ui test`, `typecheck`, `e2e` PASS.
- [ ] **Step 5: Commit** `feat(ui): toast peringatan budget setelah transaksi disimpan`

### Task 9: Verifikasi penuh dan dokumen

**Files:**
- Modify: `docs/plans/2026-10-12-v1-roadmap.md` (tabel: 7a selesai; catatan keputusan K1–K11 singkat di bagian Fase 7)

- [ ] **Step 1:** `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` → semua PASS.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui build && pnpm --filter @nabungo/ui e2e` → PASS; `git diff --exit-code ui/src/ipc/bindings.ts` bersih setelah `cargo test -p nabungo-desktop export_bindings`.
- [ ] **Step 3:** Update roadmap; commit `docs: Fase 7a selesai`.
- [ ] **Step 4 (manual, user):** `pnpm dev` — set budget, rollover, catat expense melewati 80%/100%, cek toast dan kolom di jendela Tauri dengan DB nyata.
