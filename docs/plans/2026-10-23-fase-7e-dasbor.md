# Fase 7e — Dasbor lengkap & target nominal bucket Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menutup Fase 7 dengan dua pekerjaan:
- **RPT-01:** Dasbor mendapat kartu Top 5 kategori, Terjadwal jatuh tempo, dan Ringkasan alokasi. Kartu yang sudah ada (KPI, arus kas, transaksi terakhir, akun, budget perlu dilihat) tidak berubah.
- **CAP-01:** bucket alokasi bisa ditargetkan dalam **nominal** (mis. Dana darurat Rp 30.000.000), bukan hanya persen.

**Architecture:**
- `nabungo-core::allocation`: enum `Target { Percent(bp), Amount(minor) }`. Target efektif dihitung per tingkat: bucket nominal dipenuhi dulu, sisanya dibagi bucket persen (K1).
- `nabungo-storage`: migrasi `0008` menambah kolom `allocation_bucket.target_amount`.
- `nabungo-app`/`src-tauri`: field dan DTO ikut, tanpa use case baru.
- Dasbor memakai IPC yang sudah ada (`categoryBreakdown`, `listRules`, `allocationOverview`). Tidak ada command baru untuk dasbor.

**Tech Stack:** Rust (tanpa dependensi baru), Vue 3 + shadcn-vue (komponen yang sudah ada: card, toggle-group, badge), Vitest, Playwright.

**Spec:**
- `PRD.md`: RPT-01, CAP-01, §4.5 Capital allocation, tabel layar "Dashboard", §6 invariant 4.
- Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 7, baris 7e.
- Desain: `docs/design/design.md` §7.11–7.13, §8.1 (Dasbor), §8.8 (Alokasi).
- Keputusan alokasi Fase 5: K6 (maks 2 level), K7 (target global child), K9 (riwayat).

**Di luar plan ini:**
- Tombol catat/lewati langsung dari kartu jatuh tempo (cukup tautan ke layar Terjadwal).
- Bucket ikut ekspor JSON (belum diekspor sejak Fase 6a; tidak berubah).
- Riwayat perubahan target (PRD §4.5: garis target selalu memakai target saat ini).

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Aturan campuran per tingkat (sesama parent).** Basis tingkat root = gross aset; basis child = target nominal efektif parent-nya. Di setiap tingkat: `sisa = max(basis − Σ nominal sibling, 0)`; target bucket nominal = nominalnya; target bucket persen = `sisa × bp ÷ 10000`. Child boleh nominal atau persen apa pun jenis parent-nya. | "Dana darurat Rp 30 jt dulu, sisanya 60/40" adalah pola yang umum. Tanpa nominal di satu tingkat, rumusnya identik dengan K7 Fase 5. |
| K2 | **Dua ruang hitung dengan fungsi tingkat yang sama.** Ruang nominal (basis root = gross, nominal = nominal) menghasilkan `target_amount` dan `diff_amount = actual − target_amount`. Ruang bp (basis root = 10000, nominal = `nominal × 10000 ÷ gross`, atau 0 bila gross ≤ 0) menghasilkan `target_bp` global dan `diff_bp`. Pembagian i128, dibulatkan ke bawah. | Tanpa nominal, ruang bp sama persis dengan K7, termasuk saat gross 0 (test lama tetap lolos). Nominal tetap tepat dalam rupiah ("kurang Rp 5 jt lagi"). |
| K3 | **Penyimpanan:** migrasi `0008_bucket_target_amount.sql`: `ALTER TABLE allocation_bucket ADD COLUMN target_amount INTEGER CHECK (target_amount IS NULL OR target_amount > 0);`. Bucket nominal disimpan dengan `target_bp = 0` dan `target_amount = n`; bucket persen dengan `target_amount = NULL`. | Kolom lama tetap valid (`CHECK target_bp BETWEEN 0 AND 10000`); tanpa membangun ulang tabel. |
| K4 | **Validasi:** nominal harus > 0 → `AllocationError::InvalidAmount` (kode `allocation_invalid_amount`). Batas Σ persen sibling ≤ 10000 (`check_sibling_targets`) hanya menjumlah bucket persen. Σ nominal boleh melebihi basis (hanya peringatan, K5). | Saldo bergerak; menolak simpan karena saldo hari ini lebih kecil dari target akan membingungkan. |
| K5 | **Peringatan:** `SiblingWarning { parent_id, total_bp }` hanya untuk tingkat yang punya ≥ 1 bucket persen dan Σ persen ≠ 10000. Peringatan baru `AmountWarning { parent_id, total_amount, base }` muncul bila gross > 0 dan Σ nominal sibling > basis (ruang nominal). Tingkat yang semuanya nominal tidak memberi `SiblingWarning`. | Dengan sibling nominal, bucket persen membagi sisa, jadi Σ persen tetap harus 100%. |
| K6 | **Riwayat porsi** tetap memakai mapping dan target saat ini (PRD §4.5). Garis target di grafik riwayat (`AllocationView`) memakai `targetBp` global dari `actuals` saat ini, bukan `bucket.targetBp`. | `bucket.targetBp` bernilai 0 untuk bucket nominal. |
| K7 | **Kartu Top 5 kategori:** 5 baris pertama dari `categoryBreakdown(periode berjalan, 'expense')` (urutan dari backend: nominal turun, lalu nama). Tiap baris: ikon/warna kategori, nama, nominal (`MoneyText`), dan batang dengan lebar = nominal ÷ nominal baris pertama. Kartu disembunyikan bila tidak ada pengeluaran. Judul menaut ke `/reports`. | RPT-01 "top 5 kategori". Lebar batang hanya presentasi; tidak ada angka turunan yang ditampilkan. |
| K8 | **Kartu Terjadwal jatuh tempo:** kejadian `rule.due` dari semua jadwal yang belum selesai, diurutkan menurut tanggal lalu nama (`ruleName` dari `lib/recurrence.ts`), maksimal 5 baris. Tiap baris: tanggal, nama, nominal bertanda sesuai jenis. Footer "Lihat semua (n)" → `/scheduled` bila n > 5. Kartu disembunyikan bila n = 0. | Tabel layar PRD "transaksi terjadwal jatuh tempo"; aksi tetap di layar Terjadwal (K13 plan 7d). |
| K9 | **Kartu Ringkasan alokasi:** bucket root dari `allocationOverview()` (urutan pohon). Tiap baris: nama, "aktual% / target%", dan `AllocationBar`. Di bawahnya: "Belum dialokasikan x%" bila `unallocatedBp > 0`, dan satu baris peringatan bila `warnings` atau `amountWarnings` tidak kosong. Kartu disembunyikan bila belum ada bucket. Judul menaut ke `/allocation`. | Tabel layar PRD "ringkasan alokasi". |
| K10 | **Tata letak:** baris baru di bawah grid transaksi terakhir, `grid grid-cols-1 gap-4 md:gap-6 @3xl/main:grid-cols-2 @5xl/main:grid-cols-3`, urutan Top kategori, Jatuh tempo, Alokasi. Baris ini juga tampil saat belum ada transaksi (empty state hanya menggantikan arus kas & transaksi terakhir), karena kartu-kartunya menyembunyikan dirinya sendiri. | Mengikuti pola grid `dashboard-01` (design.md §8.1). |
| K11 | **Store:** `useDashboardStore().load()` ikut memuat `topCategories` (`categoryBreakdown`) dan `allocation` (`allocationOverview`). Jatuh tempo memakai `useScheduledStore()` yang sudah ada (`reload()` dipanggil dari `DashboardView.load`). Semua dimuat ulang saat `ledgerVersion` berubah. | Tidak menduplikasi state jadwal; badge sidebar dan kartu selalu sama. |
| K12 | **Form bucket:** `BucketDialog` mendapat `ToggleGroup` "Persen / Nominal". Persen memakai input yang ada; nominal memakai `AmountInput` (`await commit()` sebelum submit). Hint nominal: "Nominal tetap. Bucket persen di tingkat yang sama membagi sisanya." Tabel alokasi menampilkan target bucket nominal sebagai nominal (`MoneyText`) di baris keterangan, menggantikan "x% dari {parent}"; kolom Target% tetap menampilkan `targetBp` global. | Satu dialog untuk kedua jenis target. |
| K13 | `browser-ledger.ts` meniru K1–K5 (fungsi tingkat yang sama dipanggil dua kali), dengan aturan error yang sama. Test kasus yang sama ada di kedua sisi. | Aturan turunan harus sama dengan Rust (CLAUDE.md). |

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`. `src-tauri` tanpa logika bisnis.
- `nabungo-core` tanpa IO, `#![deny(clippy::float_arithmetic)]`; hitungan alokasi memakai i128.
- Migrasi `0001`–`0007` tidak diubah; `0008` didaftarkan di `MIGRATIONS` (`crates/storage/src/lib.rs`).
- Uang `i64` satuan minor; persen dalam bp (10000 = 100%). UI tidak menghitung porsi/target, hanya menampilkan angka backend.
- `ui/src/ipc/bindings.ts` hanya lewat generate (`cargo test -p nabungo-desktop export_bindings`) dan wajib di-commit.
- Kode error baru masuk `errors` di `ui/src/i18n/id.ts` **dan** `en.ts`.
- Semua teks UI lewat i18n (id + en). UI mengikuti `docs/design/design.md`; perbarui §8.1 dan §8.8.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`, `pnpm --filter @nabungo/ui e2e`.
- Commit diakhiri `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Data lama tanpa nominal.** Semua `target_bp`/`diff_bp` dan baris root tidak berubah. Pengecualian yang disengaja (temuan review akhir): `diff_amount` child kini dihitung dari target rupiah parent, `floor(floor(gross × p) × c)`, bukan `gross × floor(p × c)`, sehingga bisa bergeser beberapa rupiah dan konsisten dengan `target_amount`; dipatok oleh `child_diff_amount_uses_parent_amount` (core + browser). Test: semua test `allocation.rs` lama lolos tanpa diubah (Task 1), ditambah `percent_only_matches_fase5_rule` di Task 5 (browser).
2. **Σ nominal melebihi total aset** (saldo turun setelah target dibuat). Bucket persen sibling bertarget 0, ada `AmountWarning`, tidak ada error. Test: Task 1 `nominal_exceeding_base_warns`.
3. **Gross 0** (akun baru, semua saldo 0). Tidak ada panik/pembagian nol; nominal `target_bp = 0`, tanpa `AmountWarning`. Test: Task 1 `zero_gross_with_nominal`.
4. **Ubah bucket dari persen ke nominal dan sebaliknya.** Kolom lama dibersihkan (`target_amount = NULL` / `target_bp = 0`), dan batas Σ persen menghitung ulang tanpa bucket yang kini nominal. Test: Task 3 `switch_target_kind_roundtrip`.
5. **Dasbor dengan data kosong.** Tanpa pengeluaran, jadwal, atau bucket, ketiga kartu baru tidak tampil dan tidak ada error. Test: Task 6 `new cards hidden without data`.

---

### Task 1: Core — `Target` dan aturan campuran

**Files:**
- Modify: `crates/core/src/allocation.rs`, `crates/core/src/lib.rs` (re-export `Target`, `AmountWarning`, `check_target`)

**Interfaces:**
- Produces:
  - `pub enum Target { Percent(i64), Amount(i64) }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `Bucket { id, parent_id, target: Target }` (menggantikan `target_bp`)
  - `pub fn check_target(t: Target) -> Result<Target, AllocationError>`: Percent → `check_bp`; Amount ≤ 0 → `AllocationError::InvalidAmount`
  - `BucketActual` + field `target_amount: i64`
  - `pub struct AmountWarning { pub parent_id: Option<Uuid>, pub total_amount: i64, pub base: i64 }`
  - `AllocationReport` + field `amount_warnings: Vec<AmountWarning>`

- [ ] **Step 1: Ubah helper test `bucket(id, parent, bp)` menjadi target `Percent(bp)`, lalu tulis test baru**

```rust
#[test] fn nominal_bucket_takes_amount_first() {
    // root: darurat Amount(30_000_000), invest Percent(6000), liquid Percent(4000); 1 bank 50_000_000
    assert_eq!((row(&r, darurat).target_amount, row(&r, darurat).target_bp), (30_000_000, 6000));
    assert_eq!((row(&r, invest).target_amount, row(&r, invest).target_bp), (12_000_000, 2400));
    assert_eq!((row(&r, liquid).target_amount, row(&r, liquid).target_bp), (8_000_000, 1600));
    assert!(r.warnings.is_empty() && r.amount_warnings.is_empty());
}
#[test] fn child_percent_of_nominal_parent() {
    // root darurat Amount(30_000_000) → child a Percent(5000); gross 60_000_000
    assert_eq!((row(&r, a).target_amount, row(&r, a).target_bp), (15_000_000, 2500));
}
#[test] fn nominal_exceeding_base_warns() {
    // root: x Amount(30_000_000), y Amount(30_000_000), z Percent(10000); gross 50_000_000
    assert_eq!(r.amount_warnings, vec![AmountWarning { parent_id: None, total_amount: 60_000_000, base: 50_000_000 }]);
    assert_eq!((row(&r, z).target_amount, row(&r, z).target_bp), (0, 0));
    assert_eq!(row(&r, x).diff_amount, row(&r, x).actual - 30_000_000);
}
#[test] fn zero_gross_with_nominal() {
    // root: x Amount(1_000_000), z Percent(10000); tanpa akun
    assert_eq!(row(&r, x).target_bp, 0);
    assert_eq!(row(&r, z).target_bp, 10000);
    assert!(r.amount_warnings.is_empty());
}
#[test] fn percent_warning_ignores_nominal() {
    // root: x Amount(1_000), z Percent(10000) → tanpa warning; parent p Percent(10000) dengan child hanya Amount → tanpa warning
    assert!(r.warnings.is_empty());
}
#[test] fn target_checks() {
    assert_eq!(check_target(Target::Amount(0)), Err(AllocationError::InvalidAmount));
    assert_eq!(check_target(Target::Amount(-5)), Err(AllocationError::InvalidAmount));
    assert_eq!(check_target(Target::Percent(10001)), Err(AllocationError::InvalidBasisPoints));
    assert_eq!(check_target(Target::Amount(1)), Ok(Target::Amount(1)));
}
```

- [ ] **Step 2: Jalankan `cargo test -p nabungo-core allocation::`.** Diharapkan: gagal kompilasi (`Target`/`target_amount` belum ada).

- [ ] **Step 3: Implementasi.** Satu fungsi privat per tingkat, dipanggil di ruang nominal dan ruang bp (K2):

```rust
/// Target tiap bucket satu tingkat dari `base` (K1): nominal dulu, persen membagi sisanya.
fn level_targets(base: i128, siblings: &[&Bucket], nominal: impl Fn(i64) -> i128) -> Vec<i128>
```

Hitung target root dengan basis gross/10000, lalu target child dengan basis = target parent di ruang yang sama. `target_bp` = hasil ruang bp, `target_amount` = hasil ruang nominal, `diff_bp = actual_bp − target_bp`, `diff_amount = actual − target_amount`. `SiblingWarning` hanya untuk tingkat dengan ≥ 1 bucket persen (Σ persen ≠ 10000). `AmountWarning` (gross > 0, Σ nominal > basis ruang nominal) diurutkan menurut kemunculan parent seperti `warnings`.

- [ ] **Step 4: Jalankan `cargo test -p nabungo-core allocation::`.** Diharapkan: semua lolos, termasuk test lama yang tidak diubah selain helper `bucket`.

- [ ] **Step 5: Commit** `feat(core): target nominal bucket alokasi (aturan campuran per tingkat)`

### Task 2: Storage — migrasi 0008 dan kolom `target_amount`

**Files:**
- Create: `crates/storage/migrations/0008_bucket_target_amount.sql` (isi K3)
- Modify: `crates/storage/src/lib.rs` (`MIGRATIONS`), `crates/storage/src/allocation.rs`

**Interfaces:**
- Consumes: `nabungo_core::Target` (Task 1)
- Produces: `NewBucket { parent_id, name, target: Target, color, sort_order }` dan `BucketRow { id, parent_id, name, target: Target, color, sort_order }`. Percent ditulis `(bp, NULL)`, Amount ditulis `(0, n)`; baca: `target_amount` non-NULL → `Amount`.

- [ ] **Step 1: Tulis test** di `allocation.rs`:
  - `bucket_target_roundtrip`: insert `Amount(30_000_000)` → `get_bucket` mengembalikan `Target::Amount(30_000_000)`; update ke `Percent(2500)` → `Target::Percent(2500)`, dan kolom `target_amount` (query mentah) `NULL`.
  - `target_amount_check_rejects_zero`: `UPDATE allocation_bucket SET target_amount = 0` → error constraint.
- [ ] **Step 2: Jalankan `cargo test -p nabungo-storage allocation::`.** Diharapkan: gagal kompilasi.
- [ ] **Step 3: Implementasi** migrasi, daftar `MIGRATIONS`, `ROW_SQL`/`RawBucket`, insert/update.
- [ ] **Step 4: Jalankan `cargo test -p nabungo-storage`.** Diharapkan: lolos (termasuk test migrasi/`user_version` yang sudah ada; bila ada test yang mematok jumlah migrasi, naikkan ke 8).
- [ ] **Step 5: Commit** `feat(storage): migrasi 0008 target nominal bucket`

### Task 3: App — `BucketFields.target`

**Files:**
- Modify: `crates/app/src/allocation.rs`

**Interfaces:**
- Consumes: Task 1–2
- Produces: `BucketFields { name, parent_id, target: Target, color }`. `AllocationOverview.report` membawa `target_amount` dan `amount_warnings`.

- [ ] **Step 1: Ubah helper test `fields(name, parent, bp)` menjadi `Target::Percent(bp)`, lalu tulis test:**
  - `create_nominal_bucket_and_overview`: bucket `Amount(500_000)` + bucket `Percent(10000)`, akun bank 1.000.000. `allocation_overview()` memberi target_amount 500.000 dan 500.000, target_bp 5000 dan 5000; `create_bucket` kedua (Percent 10000) **tidak** ditolak `TargetsExceed`.
  - `nominal_must_be_positive`: `create_bucket(Amount(0))` → `AppError::Allocation(AllocationError::InvalidAmount)`.
  - `switch_target_kind_roundtrip`: root A `Percent(6000)`, B `Percent(4000)`; update A → `Amount(1_000)`, lalu buat C `Percent(6000)` berhasil (Σ persen = 10000); update A kembali ke `Percent(1)` → `TargetsExceed { total: 10001 }`.
- [ ] **Step 2: Jalankan `cargo test -p nabungo-app allocation::`.** Diharapkan: gagal kompilasi.
- [ ] **Step 3: Implementasi.** `check_siblings(conn, id, parent, target: Target)`: `check_target(target)?`, lalu `check_sibling_targets` atas bp sibling ber-`Percent` (+ target ini bila `Percent`). `core_buckets` memetakan `row.target`.
- [ ] **Step 4: Jalankan `cargo test -p nabungo-app`.** Diharapkan: lolos.
- [ ] **Step 5: Commit** `feat(app): bucket alokasi dengan target nominal`

### Task 4: Desktop — DTO, kode error, bindings

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/error.rs`, `ui/src/ipc/bindings.ts` (generate), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`

**Interfaces:**
- Produces (TS, camelCase):
  - `BucketDto.targetAmount: number | null`. `targetBp` tetap ada (0 untuk nominal).
  - `BucketFieldsDto.targetAmount: number | null`: non-null → `Target::Amount`, `targetBp` diabaikan; null → `Target::Percent(targetBp)`.
  - `BucketActualDto.targetAmount: number`
  - `AmountWarningDto { parentId: string | null, totalAmount: number, base: number }`
  - `AllocationOverviewDto.amountWarnings: AmountWarningDto[]`
  - `ErrorCode::AllocationInvalidAmount` → `"allocation_invalid_amount"`; i18n `errors.allocation_invalid_amount`: id `'Target nominal harus lebih dari 0'`, en `'Target amount must be greater than 0'`.

- [ ] **Step 1: Tulis test** di `dto.rs`: `bucket_fields_amount_wins` (`targetAmount: Some(5)` + `targetBp: 7000` → `Target::Amount(5)`), dan di `error.rs` tambahkan assert `InvalidAmount` → `AllocationInvalidAmount` ke `allocation_errors_map`.
- [ ] **Step 2: Jalankan `cargo test -p nabungo-desktop`.** Diharapkan: gagal kompilasi.
- [ ] **Step 3: Implementasi** DTO dan mapping error; generate `cargo test -p nabungo-desktop export_bindings`; tambah kunci i18n.
- [ ] **Step 4: Jalankan `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `pnpm --filter @nabungo/ui typecheck`.** Diharapkan: lolos. Typecheck mungkin gagal di `browser-ledger.ts`/`BucketDialog.vue` karena field baru wajib; perbaikan minimal (`targetAmount: null`) boleh di sini, perilakunya di Task 5.
- [ ] **Step 5: Commit** `feat(desktop): DTO target nominal bucket`

### Task 5: UI Alokasi — parity browser-ledger, dialog, tabel

**Files:**
- Modify: `ui/src/ipc/browser-ledger.ts` (+ test alokasi yang sudah ada, mis. `browser-ledger.test.ts`), `ui/src/components/allocation/BucketDialog.vue`, `ui/src/views/AllocationView.vue` (+ test), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/design.md` §8.8

**Interfaces:**
- Consumes: DTO Task 4
- Produces: i18n `allocation.targetKind: { percent: 'Persen', amount: 'Nominal' }`, `allocation.targetAmountHint: 'Nominal tetap. Bucket persen di tingkat yang sama membagi sisanya.'`, `allocation.warnAmountRoot: 'Target nominal tingkat utama {total} melebihi total aset {base}'`, `allocation.warnAmountChild: 'Target nominal di {name} {total} melebihi target {name} {base}'`. `warnRoot`/`warnChild` menjadi "Target persen …". Padanan en.

- [ ] **Step 1: Tulis test browser-ledger** dengan angka yang sama seperti Task 1: `nominal_bucket_takes_amount_first`, `nominal_exceeding_base_warns`, `percent_only_matches_fase5_rule` (pohon Aset 7000 → Saham 6000/Emas 4000, Liquid 3000: target 7000/4200/2800/3000), dan `createBucket({targetAmount: 0})` → `allocation_invalid_amount`.
- [ ] **Step 2: Tulis test view**: `AllocationView` menampilkan nominal bucket (`bucket-target-amount-<id>` berisi `30.000.000`) dan teks `allocation-warnings` untuk `amountWarnings`; `BucketDialog` dengan toggle Nominal mengirim `{ targetAmount: 30000000, targetBp: 0 }`.
- [ ] **Step 3: Jalankan `pnpm --filter @nabungo/ui exec vitest run src/ipc src/views/AllocationView.test.ts`.** Diharapkan: gagal.
- [ ] **Step 4: Implementasi** K12/K13. Garis target riwayat memakai `actuals.get(b.id)?.targetBp` (K6). Perbarui design.md §8.8 (dialog Persen/Nominal, baris keterangan nominal, peringatan nominal).
- [ ] **Step 5: Jalankan `pnpm --filter @nabungo/ui test` dan `typecheck`.** Diharapkan: lolos.
- [ ] **Step 6: Commit** `feat(ui): target nominal bucket di layar Alokasi`

### Task 6: UI Dasbor — Top kategori, Jatuh tempo, Ringkasan alokasi

**Files:**
- Create: `ui/src/components/dashboard/TopCategoriesCard.vue`, `DueScheduledCard.vue`, `AllocationSummaryCard.vue`
- Modify: `ui/src/stores/dashboard.ts`, `ui/src/views/DashboardView.vue`, `ui/src/views/DashboardView.test.ts`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/design.md` §8.1 (sketsa + poin kartu baru)

**Interfaces:**
- Consumes: `ipc.categoryBreakdown`, `ipc.allocationOverview`, `useScheduledStore()` (`rules`, `reload`), `ruleName` dari `@/lib/recurrence`, `AllocationBar`, `MoneyText`
- Produces:
  - `useDashboardStore` + `topCategories: Ref<CategoryAmountDto[]>` (maks `TOP_LIMIT = 5`), `allocation: Ref<AllocationOverviewDto | null>`
  - konstanta `DUE_LIMIT = 5` di `DueScheduledCard.vue`
  - testid: `dashboard-top-categories` / `top-category-row`, `dashboard-due` / `dashboard-due-row` / `dashboard-due-more`, `dashboard-allocation` / `allocation-summary-row`
  - i18n `dashboard`: `topCategories: 'Top kategori pengeluaran'`, `due: 'Terjadwal jatuh tempo'`, `dueMore: 'Lihat semua ({n})'`, `allocation: 'Ringkasan alokasi'`, `allocationRow: '{actual} / target {target}'`, `allocationWarning: 'Target alokasi perlu dicek'`, `unallocated: 'Belum dialokasikan {percent}'`. Padanan en.

- [ ] **Step 1: Tulis test** di `DashboardView.test.ts` (data lewat browser-ledger seperti test yang ada):
  - `top categories shows at most 5 in backend order`: pengeluaran di 6 kategori expense dengan root berbeda dicatat di periode berjalan → 5 `top-category-row`, baris pertama = nominal terbesar.
  - `due card lists occurrences and links to scheduled`: 6 jadwal dengan `form.date` = 7 hari lalu (frekuensi bulanan) → 5 `dashboard-due-row` dan `dashboard-due-more` berisi `(6)`.
  - `allocation summary shows root buckets`: root `Percent(10000)` + child → 1 `allocation-summary-row` (child tidak tampil).
  - `new cards hidden without data`: setelah onboarding saja (state baru) → tidak ada `dashboard-top-categories`, `dashboard-due`, `dashboard-allocation`.
- [ ] **Step 2: Jalankan `pnpm --filter @nabungo/ui exec vitest run src/views/DashboardView.test.ts`.** Diharapkan: gagal.
- [ ] **Step 3: Implementasi** K7–K11. Nominal kartu jatuh tempo memakai `MoneyText` dengan tone dari jenis template (expense keluar, income masuk, transfer netral), seperti baris jadwal di `ScheduledView`.
- [ ] **Step 4: Jalankan `pnpm --filter @nabungo/ui test` dan `typecheck`.** Diharapkan: lolos.
- [ ] **Step 5: Commit** `feat(ui): Dasbor lengkap — top kategori, jatuh tempo, ringkasan alokasi (RPT-01)`

### Task 7: E2E, dokumen, verifikasi akhir

**Files:**
- Create: `ui/e2e/dashboard-7e.spec.ts`
- Modify: `docs/plans/2026-10-12-v1-roadmap.md` (7e selesai, keputusan 7e ringkas, Fase 7 selesai), `PRD.md` §4.5 (satu poin aturan target nominal K1; CAP-01 tidak berubah)

- [ ] **Step 1: Tulis e2e** `dasbor lengkap dan bucket nominal`: onboarding (BCA 5.000.000) → catat pengeluaran 50.000 → buat jadwal bulanan bertanggal mulai hari ini (otomatis jatuh tempo) → di Alokasi buat bucket "Dana darurat" Nominal 2.970.000 dan "Investasi" Persen 100 → total aset 4.950.000, Target% Dana darurat `60%`, Investasi `40%` → Dasbor menampilkan `dashboard-top-categories` (1 baris), `dashboard-due` (1 baris), `dashboard-allocation` (2 baris).
- [ ] **Step 2: Jalankan `pnpm --filter @nabungo/ui e2e`.** Diharapkan: lolos semua.
- [ ] **Step 3: Perbarui roadmap dan PRD.**
- [ ] **Step 4: Verifikasi akhir** (daftar di Global Constraints). Diharapkan: semua lolos, `git status` bersih setelah commit, `bindings.ts` tanpa diff setelah generate ulang.
- [ ] **Step 5: Commit** `test(e2e): dasbor lengkap dan bucket nominal; docs: Fase 7e selesai`
