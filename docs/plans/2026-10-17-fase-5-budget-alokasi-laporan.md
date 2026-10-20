# Fase 5 — Budget, Alokasi Modal, Laporan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** User bisa mengatur budget bulanan per kategori expense dan melihat realisasinya per periode budget, mencatat nilai akun investasi (Valuation) beserta riwayatnya, melacak porsi modal aktual terhadap target per bucket, dan membaca laporan income vs expense, breakdown kategori, serta net worth dari waktu ke waktu. Setelah fase ini semua acceptance criteria MVP (PRD §5) lulus → **Gate 1**.

**Architecture:** Aturan hitung yang murni (rollup realisasi parent ← child, total budget, validasi persen, porsi aktual vs target, kumulasi net worth) masuk modul baru `nabungo-core`: `budget.rs` dan `allocation.rs`. `nabungo-storage` hanya menyediakan agregasi SQL:
- `category_totals(from, to)`: Σ posting per kategori dalam rentang tanggal;
- `balances_as_of(date)`: saldo semua akun per tanggal;
- CRUD tabel `budget`, `allocation_bucket`, `allocation_mapping` (sudah ada sejak `0001`).

`nabungo-app` mendapat blok `impl Ledger` di `budget.rs`, `valuation.rs`, `allocation.rs`, `reports.rs`. "Bulan" di semua layar Fase 5 = **periode budget** (`period_of` + `date_range`), sehingga laporan konsisten dengan budget dan tanggal gajian. Chart digambar dengan SVG buatan sendiri (tanpa library chart), masing-masing diikuti tabel angka.

**Tech Stack:**
- Rust: crate yang sudah ada, tanpa dependensi baru.
- Vue 3 + shadcn-vue. Komponen chart SVG baru di `ui/src/components/charts/`.
- Vitest + Playwright (sama dengan Fase 4).

**Spec:** `PRD.md`:
- §4.4 Budget
- §4.5 Capital allocation
- §4.2 (Valuation, refund)
- §5: BUD-01, CAP-01..04, RPT-02, RPT-03; acceptance "Transfer tidak memengaruhi budget", "Periode budget custom"
- §6: tabel `budget`, `allocation_bucket`, `allocation_mapping`; invariant 3 & 4
- §8: laporan < 500 ms

Desain: `docs/design/ui-guidelines.md` §8.10 (batang progres), §9.5 (Budget), §9.6 (Alokasi & Laporan). Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 5.

## Global Constraints

- Arah dependensi tetap. `src-tauri` hanya bergantung pada `nabungo-app`; tipe yang dibutuhkan DTO di-re-export dari `nabungo_app`.
- Semua angka diturunkan dari `posting` milik transaksi `deleted_at IS NULL AND status = 'posted'`. Realisasi budget, porsi alokasi, dan laporan tidak pernah disimpan sebagai kolom.
- Transfer (posting tanpa kategori) tidak pernah masuk income/expense/budget. Biaya admin (posting expense berkategori) masuk. Refund (posting expense positif) mengurangi realisasi.
- Uang `i64` satuan minor; perkalian persen pakai `i128` lalu dikonversi kembali dengan `MoneyError::Overflow` bila tidak muat. Persen dalam basis point (`10000` = 100%). Pembagian dibulatkan ke bawah menuju nol (truncation) — konsisten di Rust dan `browser-ledger.ts`.
- `#![deny(clippy::float_arithmetic)]` di core tetap berlaku: tidak ada `f64` di Rust. UI boleh memakai float hanya untuk geometri SVG dan lebar batang.
- Periode budget memakai `BudgetPeriod` / `period_of` / `date_range` di core; tidak ada logika "tanggal gajian" di SQL atau UI. UI memperoleh periode saat ini lewat command `period_of(todayIso())`.
- Migrasi `0001`–`0004` tidak diubah. Satu migrasi baru `0005_allocation_names.sql`.
- Satu use case tulis = satu SQLite transaction.
- Semua command baru didaftarkan di `collect_commands!`. `ui/src/ipc/bindings.ts` di-regenerate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit, tidak diedit manual.
- Semua teks UI ada di `id` dan `en`. Setiap `ErrorCode` baru masuk `errors` di kedua file.
- Komponen hanya memanggil `ui/src/ipc/index.ts`. `browser-ledger.ts` mengimplementasikan setiap command baru dengan aturan yang sama dengan Rust; test di `browser-ledger.test.ts`.
- UI mengikuti `docs/design/ui-guidelines.md`: `PageHeader` di setiap view, kolom angka rata kanan dengan `num`, `MoneyText`, warna chart `--chart-1..5`, tanpa gradient/3D, setiap chart punya tabel angka.
- Setiap task diakhiri dengan perintah berikut, semuanya hijau:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `pnpm --filter @nabungo/ui typecheck`
  - `pnpm --filter @nabungo/ui test`

## Keputusan desain (berlaku untuk semua task)

| # | Keputusan | Alasan |
|---|-----------|--------|
| K1 | "Bulan" di Budget, Laporan, dan riwayat Alokasi = periode budget (dinamai bulan tempat periode berakhir). | Satu definisi bulan di seluruh app; user tanggal gajian 25 melihat siklus yang sama di semua layar. |
| K2 | Realisasi kategori parent = posting di parent + semua child. Total budget = Σ budget baris "teratas" (parent ber-budget, atau child ber-budget yang parent-nya tidak ber-budget); total terpakai = Σ terpakai baris yang sama. | Tidak menghitung ganda saat parent dan child sama-sama ber-budget. |
| K3 | Budget `amount = 0` adalah budget valid ("tidak boleh belanja"); menghapus budget = `set_budget(.., None)`. | Membedakan "tanpa budget" dari "budget nol". |
| K4 | Salin dari bulan lalu hanya mengisi kategori yang belum punya budget di periode tujuan dan tidak diarsip; tidak menimpa. | Aman diklik berulang. |
| K5 | Valuation dicatat sebagai "nilai baru per tanggal"; app menghitung `delta = nilai − saldo per akhir tanggal itu`. Delta 0 ditolak (`valuation_unchanged`). Valuation & adjustment tidak bisa diedit lewat form transaksi (hapus + catat ulang). | User berpikir dalam nilai, bukan selisih. |
| K6 | Bucket alokasi maksimal 2 level (root → child), sama dengan kategori. Akun boleh dipetakan ke bucket mana pun; porsi aktual parent = mapping sendiri + child. | PRD hanya mencontohkan 2 level; mencegah siklus dan UI pohon dalam. |
| K7 | Porsi dibandingkan secara global: target efektif child = target parent × target child ÷ 10000. Tabel juga menampilkan target relatif child ("60% dari Aset"). | PRD §4.5: "Saham 60% dari Aset = 42% dari total". |
| K8 | Gross aset = Σ saldo akun non-liability dengan `include_in_net_worth`, termasuk yang diarsip (sama dengan net worth). Gross ≤ 0 → semua porsi aktual 0. | Konsisten dengan sidebar. |
| K9 | Titik riwayat (net worth & alokasi) = saldo per tanggal terakhir periode (`date_range().1 − 1 hari`), 12 periode berakhir di periode yang diminta. | Satu query `balances_as_of` per titik; cukup cepat untuk target 500 ms, dioptimalkan di Fase 8 bila perlu. |
| K10 | Chart SVG buatan sendiri (Donut, Bar, Line), tanpa dependensi. | Offline, bundle kecil, mudah dites di jsdom; kebutuhan chart sederhana. |
| K11 | Pool koneksi baca tetap ditunda (roadmap Fase 2 menundanya ke Fase 5): laporan memakai `Mutex<Ledger>` yang sama. Diukur ulang di benchmark Fase 8. | Query Fase 5 sedikit dan ringan; menambah pool menambah kompleksitas tanpa bukti kebutuhan. |
| K12 | Dasbor (RPT-01, budget di Garis periode) tidak disentuh — v1/Fase 7. | Di luar scope roadmap Fase 5. |

## Error code baru

Ditambahkan ke `ErrorCode` (src-tauri), `errors` di `id.ts` dan `en.ts`, dan `browser-ledger.ts`:

| Code | id | en |
|------|----|----|
| `invalid_period` | Periode tidak valid | Invalid period |
| `budget_negative` | Budget tidak boleh negatif | Budget cannot be negative |
| `budget_too_large` | Budget terlalu besar | Budget is too large |
| `budget_income_category` | Budget hanya untuk kategori pengeluaran | Budgets are for expense categories only |
| `valuation_unchanged` | Nilai sama dengan saldo saat ini | Value equals the current balance |
| `txn_not_editable` | Transaksi ini tidak bisa diubah lewat form; hapus lalu catat ulang | This transaction can't be edited with the form; delete and record it again |
| `allocation_invalid_bp` | Persentase harus antara 0 dan 100 | Percentage must be between 0 and 100 |
| `allocation_targets_exceed` | Total target satu tingkat melebihi 100% | Targets on one level exceed 100% |
| `allocation_shares_exceed` | Total porsi satu akun melebihi 100% | An account's shares exceed 100% |
| `allocation_depth` | Bucket hanya boleh dua tingkat | Buckets can only be two levels deep |
| `allocation_liability` | Akun utang tidak ikut alokasi | Liability accounts are not allocated |
| `allocation_duplicate_bucket` | Bucket yang sama dipilih dua kali | The same bucket is selected twice |

## Review Focus

1. **Transaksi terhapus/terjadwal di agregasi.** Setiap query baru (`category_totals`, `balances_as_of`, riwayat valuation) wajib menyaring `deleted_at IS NULL AND status = 'posted'`; undo hapus harus mengembalikan angka budget/laporan. → test `category_totals_ignores_deleted_and_transfers` (Task 2) dan `budget_follows_delete_and_undo` (Task 3).
2. **Kategori diarsip / di-merge yang punya realisasi.** Pengeluaran di child yang diarsip tetap ikut ke realisasi parent; kategori diarsip yang punya budget atau realisasi di periode itu tetap tampil sebagai baris. → test `archived_child_spending_rolls_up` (Task 3).
3. **Batas periode custom.** Start day 25: transaksi 24 Okt masuk "2026-10", 25 Okt masuk "2026-11"; laporan 12 periode tidak tumpang-tindih atau bolong. → test `custom_start_day_period_boundaries` (Task 3) dan `cashflow_periods_are_contiguous` (Task 12).
4. **Akun dihapus/diarsip yang punya mapping alokasi.** Hapus akun tanpa transaksi tidak boleh gagal karena FK `allocation_mapping`; akun diarsip bersaldo tetap dihitung. → test `delete_account_removes_mappings` (Task 9).
5. **Valuation di tanggal lampau dengan transaksi sesudahnya.** Delta dihitung terhadap saldo per akhir tanggal valuation, bukan saldo hari ini; saldo akhir = nilai valuation + transaksi sesudahnya. → test `valuation_in_past_uses_balance_as_of_date` (Task 6).

---

## Bagian A — Budget (BUD-01)

### Task 1: Core — periode dari string dan rollup budget

**Files:**
- Modify: `crates/core/src/period.rs` (tambah `BudgetPeriod::parse`, varian `PeriodError::InvalidKey`)
- Create: `crates/core/src/budget.rs`
- Modify: `crates/core/src/lib.rs` (re-export)

**Interfaces:**
- Produces:
  - `BudgetPeriod::parse(key: &str) -> Result<BudgetPeriod, PeriodError>` — menerima tepat `YYYY-MM`, bulan 01–12.
  - `BudgetPeriod::last_day(self, start: PeriodStartDay) -> Date` — `date_range(start).1` dikurangi 1 hari.
  - `pub enum BudgetError { Negative, TooLarge, IncomeCategory }` (thiserror).
  - `pub fn validate_budget_amount(amount: i64) -> Result<i64, BudgetError>` — `0..=MAX_AMOUNT`.
  - `pub struct BudgetCategory { pub id: Uuid, pub parent_id: Option<Uuid> }`
  - `pub struct BudgetLine { pub category_id: Uuid, pub budget: Option<i64>, pub spent: i64 }`
  - `pub struct BudgetTotals { pub budget: i64, pub spent: i64 }`
  - `pub fn rollup_spent(categories: &[BudgetCategory], direct: &HashMap<Uuid, i64>) -> HashMap<Uuid, i64>` — `direct` = Σ posting (bertanda, expense negatif); hasil = **terpakai positif** per kategori, parent mencakup child.
  - `pub fn budget_totals(categories: &[BudgetCategory], lines: &[BudgetLine]) -> Result<BudgetTotals, MoneyError>` — aturan K2.

- [ ] **Step 1: Write the failing tests**

Di `period.rs`:
```rust
#[test]
fn parse_roundtrips_key() {
    assert_eq!(BudgetPeriod::parse("2026-11").unwrap(), p(2026, 11));
    assert_eq!(p(2026, 1).key(), "2026-01");
    for bad in ["2026-13", "2026-00", "2026-1", "26-11", "2026/11", "", "2026-11-01"] {
        assert_eq!(BudgetPeriod::parse(bad), Err(PeriodError::InvalidKey), "{bad}");
    }
}

#[test]
fn last_day_is_inclusive_end() {
    let s25 = PeriodStartDay::new(25).unwrap();
    assert_eq!(p(2026, 11).last_day(s25), date!(2026 - 11 - 24));
    assert_eq!(p(2026, 2).last_day(PeriodStartDay::FIRST), date!(2026 - 02 - 28));
}
```

Di `budget.rs`:
```rust
// food (parent) ← resto (child); transport (root)
#[test]
fn spent_rolls_up_children_and_refund_reduces() {
    let direct = HashMap::from([(food, -10_000), (resto, -50_000), (transport, -30_000 + 5_000)]);
    let s = rollup_spent(&cats, &direct);
    assert_eq!(s[&food], 60_000);
    assert_eq!(s[&resto], 50_000);
    assert_eq!(s[&transport], 25_000);
}

#[test]
fn totals_do_not_double_count_parent_and_child_budgets() {
    let lines = [
        line(food, Some(100_000), 60_000),
        line(resto, Some(70_000), 50_000),   // parent ber-budget → tidak dihitung lagi
        line(transport, None, 25_000),       // tanpa budget → tidak masuk total
        line(bills, Some(50_000), 0),        // root ber-budget tanpa realisasi → ikut total
    ];
    assert_eq!(budget_totals(&cats, &lines).unwrap(), BudgetTotals { budget: 150_000, spent: 60_000 });
}

#[test]
fn child_budget_counts_when_parent_has_none() {
    let lines = [line(food, None, 60_000), line(resto, Some(70_000), 50_000)];
    assert_eq!(budget_totals(&cats, &lines).unwrap(), BudgetTotals { budget: 70_000, spent: 50_000 });
}

#[test]
fn budget_amount_bounds() {
    assert_eq!(validate_budget_amount(0), Ok(0));
    assert_eq!(validate_budget_amount(-1), Err(BudgetError::Negative));
    assert_eq!(validate_budget_amount(MAX_AMOUNT + 1), Err(BudgetError::TooLarge));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p nabungo-core period:: budget::`
Expected: FAIL — `parse`, `last_day`, modul `budget` belum ada (compile error).

- [ ] **Step 3: Implement** `BudgetPeriod::parse`, `last_day`, dan modul `budget` sesuai Interfaces. Penjumlahan pakai `i128` → `MoneyError::Overflow` di `budget_totals`; `rollup_spent` memakai `saturating_*` cukup karena input dibatasi `MAX_AMOUNT` × jumlah posting (dokumentasikan di doc comment).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p nabungo-core`
Expected: PASS semua.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): periode dari kunci dan rollup realisasi budget"
```

### Task 2: Storage — budget dan agregasi per kategori

**Files:**
- Create: `crates/storage/src/budget.rs`
- Create: `crates/storage/src/report.rs`
- Modify: `crates/storage/src/lib.rs` (`pub mod`)

**Interfaces:**
- Produces (`budget.rs`):
  - `pub struct BudgetRow { pub category_id: Uuid, pub period: String, pub amount: i64 }`
  - `pub fn upsert_budget(tx: &Transaction, category_id: Uuid, period: &str, amount: i64) -> Result<()>` — `INSERT … ON CONFLICT(category_id, period) DO UPDATE SET amount`; `id` UUIDv7 baru saat insert.
  - `pub fn delete_budget(tx: &Transaction, category_id: Uuid, period: &str) -> Result<bool>`
  - `pub fn list_budgets(conn: &Connection, period: &str) -> Result<Vec<BudgetRow>>`
- Produces (`report.rs`):
  - `pub fn category_totals(conn: &Connection, from: Date, to_exclusive: Date) -> Result<Vec<(Uuid, i64)>>` — Σ `posting.amount` per `category_id` (tidak NULL) untuk transaksi aktif & posted dengan `from <= date < to`.
  - `pub fn balances_as_of(conn: &Connection, date: Date) -> Result<Vec<(Uuid, i64)>>` — Σ posting per akun untuk transaksi aktif & posted dengan `date <= ?`; akun tanpa posting tidak muncul.
  - `pub fn account_balance_as_of(conn: &Connection, account_id: Uuid, date: Date) -> Result<i64>`

- [ ] **Step 1: Write the failing tests** (pakai `open_in_memory()` + helper insert akun/kategori/txn seperti `txn.rs` tests)

```rust
#[test]
fn upsert_replaces_amount_and_delete_removes() { /* upsert 100 lalu 200 → list = [200]; delete → [] ; delete lagi → false */ }

#[test]
fn category_totals_ignores_deleted_and_transfers() {
    // expense -25_000 (kopi) 2026-10-02, refund +5_000 (kopi) 2026-10-03,
    // transfer BCA→Dompet 100_000, expense -9_000 (kopi) yang di-soft-delete,
    // expense -7_000 (kopi) 2026-11-01 (di luar rentang)
    let totals = category_totals(&conn, date!(2026-10-01), date!(2026-11-01)).unwrap();
    assert_eq!(totals, vec![(kopi, -20_000)]);
}

#[test]
fn balances_as_of_is_inclusive_and_skips_deleted() {
    // opening 1_000_000 2026-10-01; expense -25_000 2026-10-05; expense -1_000 2026-10-06 dihapus
    assert_eq!(account_balance_as_of(&conn, bca, date!(2026-10-04)).unwrap(), 1_000_000);
    assert_eq!(account_balance_as_of(&conn, bca, date!(2026-10-05)).unwrap(), 975_000);
    assert_eq!(balances_as_of(&conn, date!(2026-10-31)).unwrap(), vec![(bca, 975_000)]);
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-storage budget:: report::` — Expected: FAIL (modul belum ada).
- [ ] **Step 3: Implement** sesuai Interfaces. Tanggal dikirim sebagai `format_date` (`YYYY-MM-DD`) — perbandingan string aman karena lebar tetap.
- [ ] **Step 4: Run** `cargo test -p nabungo-storage` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(storage): repository budget dan agregasi per kategori/saldo per tanggal"`

### Task 3: App — use case budget

**Files:**
- Create: `crates/app/src/budget.rs` (blok `impl Ledger`)
- Modify: `crates/app/src/lib.rs` (mod, re-export `BudgetPeriod`, `BudgetError`), `crates/app/src/ledger.rs` (`AppError::Budget(#[from] BudgetError)`, `AppError::Period(#[from] PeriodError)`)

**Interfaces:**
- Consumes: Task 1 (`rollup_spent`, `budget_totals`, `validate_budget_amount`, `BudgetPeriod::parse/last_day`), Task 2 (`category_totals`, `upsert_budget`, `delete_budget`, `list_budgets`).
- Produces:
  - `pub struct BudgetOverview { pub period: BudgetPeriod, pub from: Date, pub to: Date /* inklusif */, pub rows: Vec<BudgetLine>, pub totals: BudgetTotals }`
  - `Ledger::period_of(&self, date: Date) -> Result<BudgetPeriod, AppError>` — memakai `period_start_day()`.
  - `Ledger::budget_overview(&self, period: BudgetPeriod) -> Result<BudgetOverview, AppError>` — baris = kategori expense yang tidak diarsip **plus** yang diarsip tetapi punya budget atau realisasi ≠ 0 di periode itu; urutan pohon (parent `sort_order`, lalu child-nya `sort_order`).
  - `Ledger::set_budget(&mut self, category_id: Uuid, period: BudgetPeriod, amount: Option<i64>) -> Result<(), AppError>` — `NotFound("category")`, `Budget(IncomeCategory)`, `Archived("category")` (hanya untuk set, bukan hapus).
  - `Ledger::copy_budgets(&mut self, to: BudgetPeriod) -> Result<u32, AppError>` — sumber `to.prev()`, aturan K4, kembalikan jumlah baris yang disalin.

- [ ] **Step 1: Write the failing tests** (di `budget.rs`, ledger hasil onboarding + kategori seed)

```rust
#[test]
fn transfer_does_not_affect_budget() {
    // set budget Makan 500_000; catat expense Kopi -25_000; ambil overview
    // catat transfer BCA→Dompet 200_000 dengan biaya 0; overview sama persis
    // hapus transfer; overview sama persis
}

#[test]
fn custom_start_day_period_boundaries() {
    // start day 25; expense 2026-10-24 dan 2026-10-25
    let oct = l.budget_overview(BudgetPeriod::parse("2026-10")?)?;  // 25 Sep – 24 Okt
    let nov = l.budget_overview(BudgetPeriod::parse("2026-11")?)?;  // 25 Okt – 24 Nov
    assert_eq!((oct.from, oct.to), (date!(2026-09-25), date!(2026-10-24)));
    assert_eq!(spent(&oct, kopi), 10_000);  // hanya 24 Okt
    assert_eq!(spent(&nov, kopi), 20_000);  // hanya 25 Okt
    assert_eq!(l.period_of(date!(2026-10-26))?.key(), "2026-11");
}

#[test]
fn archived_child_spending_rolls_up() { /* expense di Restoran lalu arsip Restoran → baris Restoran tetap ada, Makan & Minum mencakupnya */ }

#[test]
fn budget_follows_delete_and_undo() { /* spent turun ke 0 setelah delete_txn, kembali setelah undo_delete_txn */ }

#[test]
fn fee_counts_refund_reduces() { /* transfer dengan biaya 6_500 di "Biaya admin" → spent 6_500; refund +5_000 di Kopi mengurangi */ }

#[test]
fn set_budget_rules() {
    // income category → Err(AppError::Budget(BudgetError::IncomeCategory))
    // amount -1 → Budget(Negative); kategori diarsip → Archived("category")
    // Some(0) disimpan (K3); None menghapus baris
}

#[test]
fn copy_budgets_fills_only_missing() {
    // Okt: Makan 500k, Transport 300k, Hiburan (lalu diarsip) 100k; Nov: Makan 450k
    assert_eq!(l.copy_budgets(nov)?, 1);           // hanya Transport
    // Makan Nov tetap 450k; menyalin lagi → 0
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app budget::` — Expected: FAIL (method belum ada).
- [ ] **Step 3: Implement** sesuai Interfaces. `budget_overview`: `date_range` → `category_totals` → map `direct` → `rollup_spent` → gabung dengan `list_budgets` → `budget_totals`.
- [ ] **Step 4: Run** `cargo test -p nabungo-app` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(app): use case budget (realisasi, set, salin dari periode lalu)"`

### Task 4: IPC budget + browser-ledger

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/error.rs`, `src-tauri/src/lib.rs` (`collect_commands!`)
- Regenerate: `ui/src/ipc/bindings.ts`
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/ipc/browser-ledger.ts`, `ui/src/ipc/browser-ledger.test.ts`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`

**Interfaces:**
- Produces (command → facade di `index.ts`):
  - `period_of(date: String) -> String` → `periodOf(date: string): Promise<string>`
  - `budget_overview(period: String) -> BudgetOverviewDto` → `budgetOverview(period)`
  - `set_budget(category_id: String, period: String, amount: Option<i64>) -> ()` → `setBudget(categoryId, period, amount: number | null)`
  - `copy_budgets(period: String) -> u32` → `copyBudgets(period): Promise<number>`
- DTO (camelCase lewat serde):
  - `BudgetOverviewDto { period, prev, next, from, to, rows: Vec<BudgetRowDto>, total: BudgetTotalDto }` — `prev`/`next` kunci periode agar UI tidak menghitung bulan.
  - `BudgetRowDto { categoryId, parentId: Option<String>, budget: Option<i64>, spent: i64, remaining: Option<i64> }` — `remaining = budget − spent`.
  - `BudgetTotalDto { budget: i64, spent: i64, remaining: i64 }`
- Error: `PeriodError::InvalidKey` → `invalid_period`; `BudgetError` → `budget_*` (tabel Error code baru). Teks di `id.ts`/`en.ts`.

- [ ] **Step 1: Write the failing browser-ledger tests** (`browser-ledger.test.ts`) yang mencerminkan Task 3: `transfer does not affect budget`, `period_of with start day 25`, `copy budgets fills only missing`, `parent includes child spending`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** DTO + command + mapping error, regenerate bindings (`cargo test -p nabungo-desktop export_bindings`), facade di `index.ts`, mirror di `browser-ledger.ts` (state `budgets: {categoryId, period, amount}[]`; `periodOf`/`dateRange` port dari `period.rs`, ditulis di satu helper `browserPeriod` dalam file yang sama).
- [ ] **Step 4: Run** perintah wajib Global Constraints — Expected: semua hijau; `git diff --stat ui/src/ipc/bindings.ts` menunjukkan perubahan hasil generate.
- [ ] **Step 5: Commit** `git commit -m "feat(ipc): command budget dan mirror browser-ledger"`

### Task 5: UI — layar Budget

**Files:**
- Create: `ui/src/stores/budget.ts`, `ui/src/stores/budget.test.ts`
- Create: `ui/src/components/BudgetBar.vue`, `ui/src/components/BudgetBar.test.ts`
- Create: `ui/src/components/budget/BudgetAmountCell.vue`
- Modify: `ui/src/views/BudgetView.vue` (ganti placeholder), Create: `ui/src/views/BudgetView.test.ts`
- Modify: `ui/src/lib/date.ts` (+ test) — `formatPeriod(key: string, locale: LocaleDto): string` ("Okt 2026"/"Oct 2026"), `formatRange(from: string, to: string, locale): string` ("25 Sep – 24 Okt").
- Modify: `ui/src/i18n/id.ts`, `en.ts` (namespace `budget`)

**Interfaces:**
- Consumes: Task 4 facade.
- Produces: store `useBudgetStore()` dengan state `period: string | null`, `overview: BudgetOverviewDto | null`; aksi `init()` (periodOf(todayIso()) → load), `load(period)`, `go('prev' | 'next')`, `setAmount(categoryId, amount | null)`, `copyPrevious(): Promise<number>` — setiap aksi tulis memuat ulang dari backend.
- `BudgetBar` props `{ spent: number; budget: number }`: tinggi 6px; isi Tinta pudar ≤ 80%, Waspada > 80%, Bahaya > 100% dengan bagian lebih menonjol keluar track (§8.10); `data-state="ok|warn|over"`; `role="progressbar"` + `aria-valuenow` (persen dibulatkan).

**Isi layar (§9.5):**
- `PageHeader` judul Budget; actions: navigator `‹ Okt 2026 ›` (tombol ikon prev/next, label periode `data-testid="budget-period"`, meta rentang tanggal), tombol "Salin dari bulan lalu".
- Tabel kolom: Kategori · Budget · Terpakai · Sisa · batang+persen. Child menjorok `pl-6`, baris h-12/h-10 seperti Kategori. Angka `num` rata kanan. Sisa negatif = Bahaya dengan `−`.
- Sel Budget (`BudgetAmountCell`): tampil teks; klik/Enter → `AmountInput` inline; Enter/blur simpan, `Esc` batal, `Tab` simpan lalu fokus sel baris berikutnya; kosongkan lalu Enter = hapus budget. Baris tanpa budget menampilkan "—" (`text-muted-foreground`) dan tidak punya batang.
- Baris Total di bawah (border-t, font-medium) dari `overview.total`.
- Empty state bila tidak ada kategori expense.
- Toast setelah salin: "{n} budget disalin" / "Tidak ada budget baru untuk disalin".

- [ ] **Step 1: Write the failing tests**
  - `BudgetBar.test.ts`: `ok` di 80%, `warn` di 81%, `over` di 101%; `aria-valuenow` = 101.
  - `date.test.ts`: `formatPeriod('2026-10','id') === 'Okt 2026'`, `formatPeriod('2026-10','en') === 'Oct 2026'`, `formatRange('2026-09-25','2026-10-24','id') === '25 Sep – 24 Okt'`.
  - `BudgetView.test.ts` (browser-ledger nyata, seperti `TxnForm.test.ts`):
    - `menampilkan periode saat ini dan pindah ke periode sebelumnya`
    - `mengedit budget lewat sel lalu Enter menyimpan dan memperbarui sisa`
    - `Esc membatalkan edit`
    - `sisa negatif ditandai bahaya` (`data-state="over"`, teks diawali `−`)
    - `salin dari bulan lalu mengisi budget kosong`
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/components/BudgetBar.test.ts src/lib/date.test.ts src/views/BudgetView.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** store, komponen, view, i18n.
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ui): layar Budget dengan edit inline dan salin dari bulan lalu"`

---

## Bagian B — Valuation (CAP-04)

### Task 6: App — catat valuation dan riwayat nilai

**Files:**
- Create: `crates/app/src/valuation.rs`
- Modify: `crates/app/src/transactions.rs` (`update_txn` menolak valuation/adjustment), `crates/app/src/ledger.rs` (`AppError::ValuationUnchanged`, `AppError::NotEditable`), `crates/app/src/lib.rs`

**Interfaces:**
- Consumes: Task 2 `account_balance_as_of`.
- Produces:
  - `pub struct ValuationPoint { pub txn_id: Uuid, pub date: Date, pub change: i64, pub value: i64 }`
  - `Ledger::record_valuation(&mut self, account_id: Uuid, date: Date, value: i64) -> Result<Uuid, AppError>` — akun harus ada & tidak diarsip; `|value| <= MAX_AMOUNT` (lewat `TxnError::AmountTooLarge` dari validasi posting delta, plus cek eksplisit value); delta 0 → `ValuationUnchanged`; ditulis lewat `write_txn` dengan `TxnKind::Valuation`, satu posting tanpa kategori → core menolak akun non-investment (`TxnError::ValuationNonInvestment`).
  - `Ledger::valuation_history(&self, account_id: Uuid) -> Result<Vec<ValuationPoint>, AppError>` — valuation aktif, terbaru dulu; `value` = saldo per akhir tanggal itu.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn valuation_records_delta_against_balance() {
    // akun investment "RDN" opening 10_000_000 (2026-10-01)
    l.record_valuation(rdn, date!(2026-10-31), 10_750_000)?;
    assert_eq!(l.balance(rdn)?, 10_750_000);
    let h = l.valuation_history(rdn)?;
    assert_eq!((h[0].change, h[0].value), (750_000, 10_750_000));
}

#[test]
fn valuation_in_past_uses_balance_as_of_date() {
    // opening 10_000_000 (10-01); transfer masuk 1_000_000 (10-20)
    l.record_valuation(rdn, date!(2026-10-10), 9_500_000)?;   // delta −500_000
    assert_eq!(l.balance(rdn)?, 10_500_000);
}

#[test]
fn valuation_rules() {
    // akun bank → Err(Txn(ValuationNonInvestment)); nilai sama → ValuationUnchanged;
    // akun diarsip → Archived("account"); akun tak dikenal → NotFound("account")
}

#[test]
fn valuation_not_editable_via_form() {
    // update_txn(valuation_id, form expense) → Err(AppError::NotEditable); delete_txn tetap bisa
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app valuation::` — Expected: FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces.
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(app): valuation akun investasi dan riwayat nilai"`

### Task 7: IPC + UI — dialog nilai investasi

**Files:**
- Modify: `src-tauri/src/{dto,commands,error,lib}.rs`, `ui/src/ipc/{bindings,index,browser,browser-ledger,browser-ledger.test}.ts`, `ui/src/i18n/{id,en}.ts`
- Create: `ui/src/components/ValuationDialog.vue`, `ui/src/components/ValuationDialog.test.ts`
- Modify: `ui/src/components/AccountList.vue` (item menu "Perbarui nilai…" khusus `kind === 'investment'`), `ui/src/stores/accounts.ts` (aksi `recordValuation` yang memuat ulang overview)

**Interfaces:**
- Produces:
  - `record_valuation(account_id: String, date: String, value: i64) -> String` → `recordValuation(accountId, date, value)`
  - `valuation_history(account_id: String) -> Vec<ValuationPointDto>` → `valuationHistory(accountId)`; `ValuationPointDto { txnId, date, change, value }`
  - Error: `valuation_unchanged`, `txn_not_editable`.
- Dialog: judul "Perbarui nilai {akun}"; field Nilai saat ini (`AmountInput`, prefix Rp, default saldo sekarang) + Tanggal (`DateInput`, default hari ini); tombol Simpan; di bawahnya daftar riwayat (tanggal · nilai · perubahan bertanda dengan warna Masuk/Keluar), maksimal tinggi 240px dengan scroll. Error tampil inline (`role="alert"`).

- [ ] **Step 1: Write the failing tests** — browser-ledger: `valuation records delta`, `valuation rejects non-investment`; `ValuationDialog.test.ts`: `menyimpan nilai baru memperbarui saldo dan riwayat`, `nilai sama menampilkan error valuation_unchanged`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts src/components/ValuationDialog.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** command, bindings, facade, mirror, dialog, item menu.
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ui): perbarui nilai akun investasi dengan riwayat"`

---

## Bagian C — Alokasi modal (CAP-01..03)

### Task 8: Core — validasi dan perhitungan alokasi

**Files:**
- Create: `crates/core/src/allocation.rs`; Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub enum AllocationError { InvalidBasisPoints, TargetsExceed { total: i64 }, SharesExceed { total: i64 }, Depth, LiabilityAccount, DuplicateBucket }`
  - `pub fn check_bp(bp: i64) -> Result<i64, AllocationError>` — `0..=10000`.
  - `pub fn check_sibling_targets(targets: impl IntoIterator<Item = i64>) -> Result<(), AllocationError>` — Σ ≤ 10000.
  - `pub fn check_shares(shares: &[(Uuid, i64)]) -> Result<(), AllocationError>` — tiap bp valid, bucket unik, Σ ≤ 10000.
  - `pub struct Bucket { pub id: Uuid, pub parent_id: Option<Uuid>, pub target_bp: i64 }`
  - `pub struct Mapping { pub account_id: Uuid, pub bucket_id: Uuid, pub share_bp: i64 }`
  - `pub struct AccountBalance { pub id: Uuid, pub kind: AccountKind, pub balance: i64, pub include_in_net_worth: bool }`
  - `pub struct BucketActual { pub bucket_id: Uuid, pub target_bp: i64 /* global, K7 */, pub actual: i64, pub actual_bp: i64, pub diff_bp: i64, pub diff_amount: i64 }`
  - `pub struct Unallocated { pub account_id: Uuid, pub remaining_bp: i64, pub amount: i64 }`
  - `pub struct SiblingWarning { pub parent_id: Option<Uuid>, pub total_bp: i64 }`
  - `pub struct AllocationReport { pub gross: i64, pub liabilities: i64, pub buckets: Vec<BucketActual>, pub unallocated: Vec<Unallocated>, pub unallocated_total: i64, pub unallocated_bp: i64, pub warnings: Vec<SiblingWarning> }`
  - `pub fn allocation_report(buckets: &[Bucket], mappings: &[Mapping], accounts: &[AccountBalance]) -> Result<AllocationReport, MoneyError>`

Rumus (semua `i128`, truncation):
```
gross            = Σ balance akun non-liability & include_in_net_worth
porsi(akun→b)    = balance × share_bp / 10000
actual(b)        = Σ porsi ke b + Σ actual(child b)
actual_bp(b)     = gross > 0 ? actual × 10000 / gross : 0
target global    = root: target_bp ; child: target_bp(parent) × target_bp / 10000
diff_bp          = actual_bp − target global
diff_amount      = actual − gross × target global / 10000
remaining_bp     = 10000 − Σ share_bp akun ; amount = balance × remaining_bp / 10000
warnings         = setiap kelompok sibling (termasuk root) yang Σ target_bp ≠ 10000
urutan buckets   = urutan input (pemanggil mengirim urutan pohon)
```

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn nested_target_is_relative_to_parent() {
    // Aset 7000 → Saham 6000, Emas 4000; Liquid 3000
    let r = allocation_report(&buckets, &[], &[]).unwrap();
    assert_eq!(target(&r, saham), 4200);
    assert_eq!(target(&r, emas), 2800);
}

#[test]
fn split_mapping_and_liabilities_excluded() {
    // RDN 10_000_000 → Saham 9000, Liquid 1000; BCA 5_000_000 → Liquid; Kartu kredit −2_000_000
    let r = allocation_report(&buckets, &maps, &accts).unwrap();
    assert_eq!(r.gross, 15_000_000);
    assert_eq!(r.liabilities, 2_000_000);
    assert_eq!(actual(&r, saham), 9_000_000);
    assert_eq!(actual(&r, aset), 9_000_000);          // parent = child
    assert_eq!(actual(&r, liquid), 6_000_000);
    assert_eq!(bp(&r, liquid), 4000);
    assert_eq!(diff_amount(&r, liquid), 6_000_000 - 4_500_000);
}

#[test]
fn unmapped_and_partial_accounts_are_unallocated() {
    // GoPay 300_000 tanpa mapping; RDN 1_000_000 dipetakan 6000 saja
    assert_eq!(r.unallocated_total, 300_000 + 400_000);
    assert_eq!(r.unallocated.len(), 2);
}

#[test]
fn warnings_for_levels_not_100() { /* root Σ 9000 → warning parent None total 9000; child Σ 10000 → tidak ada */ }

#[test]
fn zero_gross_gives_zero_bp() { /* semua saldo 0 → actual_bp 0, tidak panik */ }

#[test]
fn share_and_target_checks() {
    assert_eq!(check_bp(10001), Err(AllocationError::InvalidBasisPoints));
    assert_eq!(check_sibling_targets([7000, 3001]), Err(AllocationError::TargetsExceed { total: 10001 }));
    assert_eq!(check_shares(&[(a, 6000), (a, 1000)]), Err(AllocationError::DuplicateBucket));
    assert_eq!(check_shares(&[(a, 6000), (b, 5000)]), Err(AllocationError::SharesExceed { total: 11000 }));
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-core allocation::` — Expected: FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces dan rumus.
- [ ] **Step 4: Run** `cargo test -p nabungo-core` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(core): perhitungan porsi alokasi modal vs target"`

### Task 9: Storage + App — bucket, mapping, ringkasan dan riwayat alokasi

**Files:**
- Create: `crates/storage/migrations/0005_allocation_names.sql`; Modify: `crates/storage/src/lib.rs` (`migrations()`, test indeks)
- Create: `crates/storage/src/allocation.rs`
- Modify: `crates/storage/src/account.rs` (`delete_account` menghapus mapping akun lebih dulu)
- Create: `crates/app/src/allocation.rs`; Modify: `crates/app/src/{ledger,lib}.rs` (`AppError::Allocation(#[from] AllocationError)`)

Migrasi `0005`:
```sql
-- 0005: nama bucket alokasi unik tanpa membedakan huruf (root dan sesama parent).
CREATE UNIQUE INDEX allocation_bucket_root_name_nocase ON allocation_bucket (name COLLATE NOCASE)
  WHERE parent_id IS NULL;
CREATE UNIQUE INDEX allocation_bucket_child_name_nocase ON allocation_bucket (parent_id, name COLLATE NOCASE)
  WHERE parent_id IS NOT NULL;
```

**Interfaces:**
- Produces (storage):
  - `pub struct BucketRow { pub id: Uuid, pub parent_id: Option<Uuid>, pub name: String, pub target_bp: i64, pub color: Option<String>, pub sort_order: i64 }`
  - `insert_bucket(tx, &NewBucket) -> Result<Uuid>` (UNIQUE → `StorageError::Duplicate` lewat `map_unique`), `update_bucket(tx, id, &NewBucket) -> Result<bool>`, `delete_bucket(tx, id) -> Result<bool>` (hapus mapping-nya dulu), `list_buckets(conn) -> Result<Vec<BucketRow>>` (urutan pohon), `child_bucket_count(conn, id) -> Result<i64>`
  - `list_mappings(conn) -> Result<Vec<Mapping>>`, `replace_account_mappings(tx, account_id, &[(Uuid, i64)]) -> Result<()>`
- Produces (app):
  - `pub struct BucketFields { pub name: String, pub parent_id: Option<Uuid>, pub target_bp: i64, pub color: Option<String> }`
  - `Ledger::create_bucket(&mut self, f: &BucketFields) -> Result<Uuid, AppError>` — nama lewat `normalize_name`; parent harus root (`Allocation(Depth)`); Σ target sibling baru ≤ 10000; `Duplicate("bucket")`.
  - `Ledger::update_bucket(&mut self, id: Uuid, f: &BucketFields) -> Result<(), AppError>` — sama; bucket yang punya child tidak boleh diberi parent (`Depth`); Σ sibling dihitung tanpa dirinya.
  - `Ledger::delete_bucket(&mut self, id: Uuid) -> Result<(), AppError>` — punya child → `InUse("bucket")`.
  - `Ledger::set_account_allocation(&mut self, account_id: Uuid, shares: &[(Uuid, i64)]) -> Result<(), AppError>` — akun liability → `Allocation(LiabilityAccount)`; bucket tak dikenal → `NotFound("bucket")`; `check_shares`; list kosong = hapus semua mapping.
  - `pub struct AllocationOverview { pub buckets: Vec<BucketRow>, pub mappings: Vec<Mapping>, pub report: AllocationReport }`
  - `Ledger::allocation_overview(&self) -> Result<AllocationOverview, AppError>` — saldo sekarang (sama dengan `accounts_overview`).
  - `pub struct AllocationPoint { pub period: BudgetPeriod, pub date: Date, pub gross: i64, pub buckets: Vec<(Uuid, i64 /* actual_bp */)> }` — hanya bucket root.
  - `Ledger::allocation_history(&self, end: BudgetPeriod, count: u8) -> Result<Vec<AllocationPoint>, AppError>` — K9, mapping & target saat ini, terlama dulu.
  - Helper privat bersama Task 12: `fn period_points(&self, end: BudgetPeriod, count: u8) -> Result<Vec<(BudgetPeriod, Date, Vec<AccountBalance>)>, AppError>`.

- [ ] **Step 1: Write the failing tests**
  - storage: `migration_0005_indexes_exist`; `bucket_names_unique_nocase_per_parent`; `delete_bucket_removes_mappings`.
  - app:
    - `bucket_depth_limited_to_two`
    - `sibling_targets_cannot_exceed_100`
    - `delete_bucket_with_children_is_in_use`
    - `liability_account_cannot_be_allocated`
    - `overview_matches_core_report` (RDN 90/10 seperti Task 8)
    - `delete_account_removes_mappings` (akun baru tanpa transaksi, dipetakan, lalu `delete_account` sukses)
    - `history_recomputes_from_postings` (expense di periode lalu mengubah porsi titik lalu, bukan titik sekarang)
- [ ] **Step 2: Run** `cargo test -p nabungo-storage allocation:: && cargo test -p nabungo-app allocation::` — Expected: FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces.
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(app): bucket & mapping alokasi, ringkasan dan riwayat porsi"`

### Task 10: IPC alokasi + browser-ledger

**Files:** sama dengan Task 4.

**Interfaces:**
- Produces:
  - `list_allocation() -> AllocationOverviewDto` → `allocationOverview()`
  - `create_bucket(fields: BucketFieldsDto) -> String`, `update_bucket(id, fields) -> ()`, `delete_bucket(id) -> ()`
  - `set_account_allocation(account_id: String, shares: Vec<ShareDto>) -> ()`
  - `allocation_history(end_period: String, count: u8) -> Vec<AllocationPointDto>`
- DTO: `BucketDto { id, parentId, name, targetBp, color, sortOrder }`, `BucketFieldsDto { name, parentId, targetBp, color }`, `ShareDto { bucketId, shareBp }`, `MappingDto { accountId, bucketId, shareBp }`, `BucketActualDto { bucketId, targetBp, actual, actualBp, diffBp, diffAmount }`, `UnallocatedDto { accountId, remainingBp, amount }`, `WarningDto { parentId, totalBp }`, `AllocationOverviewDto { buckets: Vec<BucketDto>, mappings: Vec<MappingDto>, gross, liabilities, actuals: Vec<BucketActualDto>, unallocated: Vec<UnallocatedDto>, unallocatedTotal, unallocatedBp, warnings: Vec<WarningDto> }`, `AllocationPointDto { period, date, gross, buckets: Vec<{ bucketId, actualBp }> }`.
- Error: `allocation_*` sesuai tabel.

- [ ] **Step 1: Write the failing browser-ledger tests**: `nested target is relative`, `split mapping and unallocated`, `rejects shares over 100%`, `rejects third level bucket`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement** (port rumus Task 8 dengan `Math.trunc` pada bilangan bulat; nilai uang tetap di bawah 2^53 karena `MAX_AMOUNT` × 10000 < 2^53 — tulis komentar ini di kode).
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ipc): command alokasi modal dan mirror browser-ledger"`

### Task 11: UI — layar Alokasi + chart Donut & Line

**Files:**
- Create: `ui/src/lib/percent.ts` (+ test): `parsePercent(text: string): number | null` → bp (terima `60`, `60,5`, `60.5`, `60%`; maksimal 2 desimal; selain itu `null`), `formatBp(bp: number, locale: LocaleDto): string` (`4200` → `42%`, `4250` → `42,5%` id / `42.5%` en).
- Create: `ui/src/components/charts/DonutChart.vue` (+ test) — props `{ segments: { key: string; label: string; value: number; color: string }[]; label: string }`; SVG `viewBox 0 0 120 120`, lingkaran tebal 16, segmen `<path data-key>` proporsional, nilai ≤ 0 dilewati; `role="img"` + `aria-label`.
- Create: `ui/src/components/charts/LineChart.vue` (+ test) — props `{ labels: string[]; series: { key: string; label: string; color: string; values: number[]; dashed?: boolean }[]; label: string; format: (v: number) => string }`; satu `<polyline data-key>` per seri; sumbu y min/max otomatis termasuk 0; label sumbu x = `labels` (setiap label ke-n bila sempit).
- Create: `ui/src/stores/allocation.ts` (+ test), `ui/src/components/allocation/BucketDialog.vue`, `ui/src/components/allocation/AccountAllocationDialog.vue`, `ui/src/components/allocation/AllocationBar.vue`
- Modify: `ui/src/views/AllocationView.vue`; Create: `ui/src/views/AllocationView.test.ts`
- Modify: `ui/src/i18n/{id,en}.ts` (namespace `allocation`)

**Isi layar (§9.6):**
- `PageHeader` "Alokasi"; actions: tombol "Bucket baru".
- Peringatan (bila `warnings` tidak kosong): baris teks Waspada di atas tabel, satu per tingkat: "Target {parent|tingkat utama} berjumlah {x}%, bukan 100%".
- Baris ringkasan: Donut (porsi aktual bucket root + "Belum dialokasikan" warna `--chart-5`) di kiri, legenda di kanan; di bawahnya meta "Total aset {gross} · Utang {liabilities} (tidak dihitung)".
- Tabel: Bucket · Target · Aktual % · Aktual Rp · Selisih (poin) · Selisih Rp · `AllocationBar` (target = garis penanda 2px, aktual = isi). Child menjorok dengan meta "60% dari Aset". Baris "Belum dialokasikan" di akhir (tanpa target). Menu ⋯ per bucket (edit, sub-bucket baru untuk root, hapus).
- Seksi "Pemetaan akun": daftar akun aset (tidak diarsip, atau diarsip bersaldo ≠ 0) dengan ringkasan mapping ("Saham 90% · Liquid 10%" atau "Belum dipetakan" berwarna Waspada) dan tombol "Atur" → `AccountAllocationDialog`: baris (Select bucket, input persen) + "Tambah baris", sisa "{x}% belum dialokasikan".
- Seksi "Riwayat porsi": `LineChart` porsi aktual bucket root 12 periode + garis putus-putus target saat ini per bucket; tabel periode × bucket di bawahnya.
- Empty state bila belum ada bucket: judul "Belum ada bucket alokasi", deskripsi singkat, tombol "Bucket baru".

- [ ] **Step 1: Write the failing tests**
  - `percent.test.ts`: `parsePercent('60') === 6000`, `parsePercent('60,5') === 6050`, `parsePercent('12.25%') === 1225`, `parsePercent('1,234')` → `null`, `parsePercent('abc')` → `null`; `formatBp(4250,'id') === '42,5%'`.
  - `DonutChart.test.ts`: tiga segmen 50/30/20 → tiga `path[data-key]`; segmen 0 tidak dirender.
  - `LineChart.test.ts`: dua seri × 12 titik → dua `polyline` dengan 12 pasangan koordinat; seri `dashed` punya `stroke-dasharray`.
  - `AllocationView.test.ts` (browser-ledger nyata):
    - `empty state lalu membuat bucket`
    - `peringatan muncul saat target root bukan 100%`
    - `memetakan akun memperbarui porsi aktual dan menghapus dari belum dialokasikan`
    - `child menampilkan target relatif terhadap parent`
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/lib/percent.test.ts src/components/charts src/views/AllocationView.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ui): layar Alokasi dengan donut, tabel selisih, pemetaan akun, riwayat"`

---

## Bagian D — Laporan (RPT-02, RPT-03)

### Task 12: App — laporan cashflow, breakdown kategori, net worth

**Files:**
- Create: `crates/app/src/reports.rs`; Modify: `crates/app/src/lib.rs`

**Interfaces:**
- Consumes: Task 2 `category_totals`, `balances_as_of`; Task 1 `rollup_spent`; Task 9 `period_points`; core `net_worth`.
- Produces:
  - `pub struct CashflowPoint { pub period: BudgetPeriod, pub from: Date, pub to: Date /* inklusif */, pub income: i64, pub expense: i64, pub net: i64 }` — income = Σ posting kategori income; expense = −Σ posting kategori expense (refund mengurangi, biaya admin ikut); transfer/valuation/adjustment/opening tidak ikut.
  - `Ledger::cashflow_report(&self, end: BudgetPeriod, count: u8) -> Result<Vec<CashflowPoint>, AppError>` — terlama dulu.
  - `pub struct CategoryAmount { pub category_id: Uuid, pub amount: i64 }`
  - `Ledger::category_breakdown(&self, period: BudgetPeriod, kind: CategoryKind) -> Result<Vec<CategoryAmount>, AppError>` — rollup ke kategori **root**, nilai positif (expense: terpakai; income: diterima), buang yang 0, urut nominal turun lalu nama.
  - `pub struct NetWorthPoint { pub period: BudgetPeriod, pub date: Date, pub assets: i64, pub liabilities: i64, pub net: i64 }`
  - `Ledger::net_worth_history(&self, end: BudgetPeriod, count: u8) -> Result<Vec<NetWorthPoint>, AppError>` — K9; akun `include_in_net_worth` (termasuk diarsip); valuation otomatis ikut karena berupa posting.
  - `count` di-clamp ke `1..=24` (berlaku juga untuk `allocation_history`); dicatat di doc comment.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn cashflow_periods_are_contiguous() {
    // start day 25; 12 periode berakhir 2026-11
    let r = l.cashflow_report(BudgetPeriod::parse("2026-11")?, 12)?;
    assert_eq!(r.len(), 12);
    assert_eq!(r[0].period.key(), "2025-12");
    for w in r.windows(2) { assert_eq!(w[0].to.next_day().unwrap(), w[1].from); }
}

#[test]
fn cashflow_excludes_transfers_and_valuation() {
    // gaji +8_000_000, kopi −25_000, refund +5_000, transfer 200_000 + biaya 6_500, valuation +750_000
    let p = &l.cashflow_report(okt, 1)?[0];
    assert_eq!((p.income, p.expense, p.net), (8_000_000, 26_500, 7_973_500));
}

#[test]
fn breakdown_rolls_up_to_root_sorted() {
    // Restoran 50_000 + Kopi 25_000 (child Makan & Minum), Transportasi 60_000
    let b = l.category_breakdown(okt, CategoryKind::Expense)?;
    assert_eq!(amounts(&b), vec![(makan, 75_000), (transport, 60_000)]);
}

#[test]
fn net_worth_history_follows_postings_and_valuation() {
    // opening BCA 1_000_000 (Sep), kartu kredit −200_000 (Okt), valuation RDN +500_000 (Nov; RDN opening 0 di Sep)
    let h = l.net_worth_history(nov, 3)?;
    assert_eq!(nets(&h), vec![1_000_000, 800_000, 1_300_000]);
    assert_eq!(h[1].liabilities, 200_000);
}
```

- [ ] **Step 2: Run** `cargo test -p nabungo-app reports::` — Expected: FAIL.
- [ ] **Step 3: Implement** sesuai Interfaces.
- [ ] **Step 4: Run** `cargo test --workspace` — Expected: PASS.
- [ ] **Step 5: Commit** `git commit -m "feat(app): laporan cashflow, breakdown kategori, net worth over time"`

### Task 13: IPC laporan + browser-ledger

**Files:** sama dengan Task 4.

**Interfaces:**
- Produces:
  - `cashflow_report(end_period: String, count: u8) -> Vec<CashflowPointDto>` → `cashflowReport(endPeriod, count)`; `CashflowPointDto { period, from, to, income, expense, net }`
  - `category_breakdown(period: String, kind: CategoryKindDto) -> Vec<CategoryAmountDto>` → `categoryBreakdown(period, kind)`; `CategoryAmountDto { categoryId, amount }`
  - `net_worth_history(end_period: String, count: u8) -> Vec<NetWorthPointDto>` → `netWorthHistory(endPeriod, count)`; `NetWorthPointDto { period, date, assets, liabilities, net }`

- [ ] **Step 1: Write the failing browser-ledger tests**: `cashflow excludes transfers and valuation`, `breakdown rolls up to root`, `net worth history per period end`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc/browser-ledger.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ipc): command laporan dan mirror browser-ledger"`

### Task 14: UI — layar Laporan + BarChart

**Files:**
- Create: `ui/src/components/charts/BarChart.vue` (+ test) — props `{ labels: string[]; series: { key: string; label: string; color: string; values: number[] }[]; label: string; format: (v: number) => string; selected?: number }`; emit `select(index)`; batang berkelompok per label, `rect[data-key][data-index]`, kelompok terpilih diberi latar `--muted`; setiap kelompok bisa difokus (`tabindex=0`, Enter/Space = select, panah kiri/kanan pindah).
- Create: `ui/src/stores/reports.ts` (+ test)
- Modify: `ui/src/views/ReportsView.vue`; Create: `ui/src/views/ReportsView.test.ts`
- Modify: `ui/src/i18n/{id,en}.ts` (namespace `reports`)

**Isi layar (§9.6):**
- `PageHeader` "Laporan"; meta "12 periode terakhir".
- Seksi "Pemasukan vs pengeluaran": `BarChart` dua seri (Masuk `--chart-2`, Keluar `--chart-3`), label `formatPeriod`; tabel di bawahnya: Periode · Masuk · Keluar · Bersih (Bersih negatif berwarna Bahaya). Klik/Enter pada batang atau baris tabel memilih periode untuk breakdown; default periode saat ini.
- Seksi "Breakdown {periode}": Tabs segmented Pengeluaran/Pemasukan; daftar kategori root dengan ikon+warna kategori, nominal `num`, persen dari total, dan batang horizontal tipis (`--chart-1`). Empty state per tab bila kosong.
- Seksi "Kekayaan bersih": `LineChart` (Task 11) seri Bersih (`--chart-1`) + Aset (`--chart-2`, putus-putus) + Utang (`--chart-3`, putus-putus); tabel Periode · Aset · Utang · Bersih.

- [ ] **Step 1: Write the failing tests**
  - `BarChart.test.ts`: 2 seri × 3 label → 6 `rect[data-key]`; tinggi proporsional (nilai 2× → tinggi 2×); Enter pada kelompok ke-1 emit `select(1)`.
  - `ReportsView.test.ts` (browser-ledger nyata):
    - `menampilkan 12 periode dan tabel cashflow`
    - `memilih periode memperbarui breakdown`
    - `tab pemasukan menampilkan kategori income`
    - `kekayaan bersih menampilkan tabel per periode`
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/components/charts/BarChart.test.ts src/views/ReportsView.test.ts` — Expected: FAIL.
- [ ] **Step 3: Implement**.
- [ ] **Step 4: Run** perintah wajib — Expected: hijau.
- [ ] **Step 5: Commit** `git commit -m "feat(ui): layar Laporan (cashflow, breakdown kategori, kekayaan bersih)"`

---

## Bagian E — Penutup

### Task 15: E2E Gate 1 + roadmap

**Files:**
- Create: `ui/e2e/budget-allocation.spec.ts`
- Modify: `docs/plans/2026-10-12-v1-roadmap.md` (Fase 5 → selesai, "Keputusan saat eksekusi" berisi K1–K12 yang relevan + ruling eksekusi)

**Tests (Playwright, pakai helper `onboard` gaya `transactions.spec.ts`):**
- `budget follows expense`: buka `#/budget`, set budget "Makan & Minum" 100.000 lewat sel (klik, ketik, Enter), catat expense Kopi 25rb lewat `N`, kembali ke Budget → Terpakai `25.000`, Sisa `75.000`; catat transfer → angka tidak berubah.
- `allocation shows actual share`: buat bucket "Liquid" 100%, petakan BCA 100% → baris Liquid Aktual `100%`, tidak ada baris akun di "Belum dipetakan".
- `reports show current period`: setelah expense, `#/reports` → baris periode saat ini kolom Keluar `25.000`.

- [ ] **Step 1: Write** spec.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui e2e` — Expected: 6/6 PASS (3 lama + 3 baru).
- [ ] **Step 3: Update roadmap**; jalankan perintah wajib + `pnpm --filter @nabungo/ui build`.
- [ ] **Step 4: Commit** `git commit -m "test(e2e): budget, alokasi, laporan; roadmap Fase 5 selesai"`
