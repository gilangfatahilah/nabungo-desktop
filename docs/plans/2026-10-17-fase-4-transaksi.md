# Fase 4 — Transaksi Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** User bisa mencatat Income/Expense/Transfer dari layar mana pun dengan keyboard saja (`N` → nominal → kategori → `Ctrl+Enter`). Form mendukung split kategori, biaya admin pada transfer, payee dengan saran kategori/akun, tag, dan catatan. Layar Transaksi menampilkan daftar virtualized dengan filter, pencarian, grouping per tanggal, dan saldo berjalan per akun. Setiap transaksi bisa diedit, diduplikasi, dan dihapus dengan undo.

**Architecture:** Aturan "isi form → postings" (tanda +/−, total split = nominal, biaya admin) masuk `nabungo-core` modul baru `form`, yang menghasilkan posting spec lalu divalidasi oleh `validate` yang sudah ada.

- `nabungo-storage` mendapat:
  - repository tag;
  - `replace_txn` dan `get_txn`;
  - pencarian payee;
  - query daftar transaksi dengan filter, paginasi `LIMIT/OFFSET`, dan saldo berjalan (window function);
  - migrasi `0004` untuk indeks.
- `nabungo-app` mendapat blok `impl Ledger` di `transactions.rs`.
- `src-tauri` menambah DTO dan command tipis.
- UI mendapat:
  - store `transactions`;
  - form transaksi di `NewTxnSheet`;
  - layar Transaksi;
  - e2e Playwright yang berjalan di Vite dengan fallback `browser-ledger`.

**Tech Stack:**
- Rust: crate yang sudah ada.
- Vue 3 + shadcn-vue. Komponen tambahan: `combobox`, `tags-input`, `dropdown-menu`; `dropdown-menu` sudah ada.
- `@tanstack/vue-virtual`.
- `@playwright/test` (devDependency `ui/`).
- Vitest.

**Spec:** `PRD.md`:
- §4.2 (jenis transaksi, refund)
- §4.6 (payee, tag)
- §5: TRX-01..05, acceptance "Transfer seimbang", "Input cepat", "Hapus aman"
- §6: invariant 1, 2, 5
- §8: performa simpan < 100 ms, buka daftar < 200 ms
- §9: form transaksi, layar Transaksi, detail interaksi

Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 4.

## Global Constraints

- Arah dependensi tetap. `src-tauri` hanya bergantung pada `nabungo-app`; tipe yang dibutuhkan DTO di-re-export dari `nabungo_app`.
- Invariant transaksi ditegakkan di Rust (core `validate` + `form`), bukan di UI dan bukan trigger SQL. UI hanya menampilkan sisa split sebagai bantuan.
- Satu use case tulis = satu SQLite transaction, mencakup header, postings, payee, dan tag (invariant 5).
- Uang `i64` satuan minor, maksimum per posting `MAX_AMOUNT` (10^12). UI memperlakukan nominal sebagai integer dan memformatnya dengan `ui/src/lib/money.ts`.
- Tanggal transaksi `YYYY-MM-DD`; default form = `todayIso()` (tanggal lokal).
- Saldo selalu diturunkan dari `posting` milik transaksi `deleted_at IS NULL AND status = 'posted'`.
- Hapus transaksi = soft delete. Undo lewat toast Sonner dengan aksi "Batalkan".
- Migrasi `0001`–`0003` tidak diubah. Tambah `0004_txn_indexes.sql` dan daftarkan di `migrations()`.
- Semua command baru didaftarkan di `collect_commands!`. `ui/src/ipc/bindings.ts` di-regenerate dan di-commit.
- Semua teks UI ada di `id` dan `en`. Setiap `ErrorCode` baru masuk ke `errors`.
- Komponen hanya memanggil `ui/src/ipc/index.ts`. `browser-ledger.ts` mengimplementasikan setiap command baru dengan aturan yang sama dengan Rust (tanda, split, biaya, filter, saldo berjalan), karena e2e berjalan di atasnya.
- Navigasi keyboard penuh. Setiap aksi baris (edit, duplikasi, hapus) bisa dicapai tanpa mouse.
- Setiap task diakhiri dengan perintah berikut, semuanya hijau:
  - `cargo fmt --all`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `pnpm --filter @nabungo/ui typecheck`
  - `pnpm --filter @nabungo/ui test`

## Keputusan desain

### Form dan aturan posting

- **Form → postings** (core `form`). User selalu mengetik nominal positif, kecuali refund.
  - **Income:** setiap baris split menjadi posting `+amount` ke akun, dengan kategori income.
  - **Expense:** setiap baris split menjadi posting `−amount` ke akun, dengan kategori expense.
  - **Refund** = expense dengan nominal negatif (PRD §4.2), sehingga posting menjadi positif dan mengurangi realisasi budget. Baris split boleh berbeda tanda, tetapi tidak boleh 0.
  - **Total split = nominal.** Selisih ditolak dengan `TxnError::SplitMismatch(selisih)`. Mode non-split = satu baris berisi seluruh nominal.
  - **Transfer:**
    - Nominal harus > 0, ditolak dengan `TxnError::NotPositive`.
    - Postingnya `−amount` dari akun asal dan `+amount` ke akun tujuan, tanpa kategori.
    - Biaya admin opsional: harus > 0 (`NotPositive`), menjadi posting `−fee` di **akun asal** dengan kategori expense pilihan user.
- **Kategori default biaya admin:**
  1. kategori biaya transfer terakhir yang masih aktif;
  2. bila belum ada, kategori expense aktif bernama "Biaya admin" atau "Fees" (tanpa membedakan huruf besar/kecil);
  3. bila tidak ada, kosong.

### Edit, duplikasi, hapus

- **Edit:**
  - `update_txn` mengganti header, postings, payee, dan tag dalam satu transaksi SQL. ID dan `created_at` tetap; `updated_at` diperbarui.
  - Transaksi `opening` tidak bisa diedit lewat sini (`opening_not_allowed`).
  - Adjustment/valuation belum punya form di fase ini; UI menonaktifkan edit/duplikasi untuk keduanya.
  - Akun/kategori yang **sudah dipakai transaksi itu** boleh tetap dipakai walau kini diarsip. Akun/kategori baru wajib aktif (`archived`).
- **Duplikasi:** membuka form create berisi nilai yang sama, dengan tanggal = hari ini.
- **Hapus:** soft delete. Toast menampilkan aksi "Batalkan" yang memanggil `undo_delete_txn`. Tombol `Delete` pada baris yang fokus melakukan hal yang sama.

### Payee, akun default, tag

- **Payee:**
  - Autocomplete dari `search_payees(query, limit=8)`: nama mengandung query (tanpa membedakan huruf besar/kecil), diurutkan menurut transaksi terakhir.
  - Saat payee dipilih atau di-blur, `payee_suggestion` mengisi kategori dan akun, **hanya bila field itu belum diubah user** di form ini.
- **Akun default form:** `payee_suggestion.accountId` → `last_used_account()` (akun dari transaksi income/expense terbaru yang masih aktif) → akun aktif non-liability pertama di urutan sidebar.
- **Tag:**
  - Nama di-trim, awalan `#` dibuang, lalu lewat `normalize_name`.
  - Unik tanpa membedakan huruf besar/kecil (indeks NOCASE di `0004`); tag yang sudah ada dipakai ulang.
  - Tag tidak pernah dihapus otomatis.

### Daftar transaksi

- **Query:** `TxnQuery` dengan field:
  - `dateFrom`/`dateTo` (inklusif)
  - `accountId`
  - `categoryId` (termasuk child-nya)
  - `tag`
  - `payeeId`
  - `amountMin`/`amountMax` (dibandingkan dengan `ABS(amount)`, inklusif)
  - `search`: substring tanpa membedakan huruf besar/kecil pada nama payee, catatan, dan memo posting; `%`/`_` di-escape.
  - `limit`/`offset`

  Urutan `date DESC, id DESC`. Hanya transaksi aktif dan `posted`.
- **Baris daftar:**
  - `amount`:
    - income/expense/adjustment/valuation/opening = Σ posting bertanda (expense negatif);
    - transfer = kaki positif (tanpa biaya).
  - `accountId` = akun utama (transfer: akun asal); `toAccountId` khusus transfer; `fee` = biaya transfer (0 bila tidak ada).
  - `categoryId` terisi hanya bila tepat satu kategori non-biaya; `splitCount` = jumlah posting berkategori non-biaya.
  - `tags`.
  - Bila `accountId` di-filter: `accountDelta` (Σ posting akun itu di transaksi ini) dan `runningBalance` (saldo akun setelah transaksi ini, dihitung atas **seluruh** riwayat akun dengan urutan `date, id`, lalu baru difilter).
  - Nama akun/kategori di-resolve UI dari store yang sudah dimuat.
- **Virtualisasi** memakai `@tanstack/vue-virtual` saja; daftar berurutan tetap tanpa sort kolom, jadi TanStack Table tidak dipakai (YAGNI). UI memuat halaman 100 baris dan mengambil halaman berikutnya saat scroll mendekati akhir. Header tanggal disisipkan sebagai item virtual.

### Input, keyboard, e2e

- **Tanggal:** `<Input type="date">` native, yang keyboard-friendly dan tidak butuh `Calendar`.
- **Input cepat (≤ 5 interaksi):**
  1. `N` membuka sheet di tab Expense dengan fokus pada nominal.
  2. Ketik nominal.
  3. `Tab` ke kategori.
  4. Ketik nama kategori lalu `Enter`.
  5. `Ctrl+Enter` simpan.

  Akun terisi otomatis. Dikunci oleh test komponen.
- **E2E (exit fase):** Playwright terhadap `pnpm --filter @nabungo/ui dev` (fallback browser-ledger). Mencakup expense, transfer dengan biaya, dan hapus + undo. Aturan Rust tetap dikunci test app; e2e mengunci alur UI. Browser Chromium Playwright ada di `~/.cache/ms-playwright`. Bila revisi tidak cocok dengan versi `@playwright/test`, jalankan `pnpm --filter @nabungo/ui exec playwright install chromium` (menulis ke `~/.cache`, bukan sistem).

## Review Focus

1. **Edit transaksi lama yang akunnya kini diarsip** (mis. hanya mengubah catatan) → berhasil. Mengganti ke akun lain yang diarsip → `archived`. Test di Task 4.
2. **Split dengan selisih 1 rupiah, atau baris split bernilai 0** → ditolak `txn_split_mismatch` / `txn_zero_amount`, tidak tersimpan sebagian. Refund (expense negatif) dengan split → saldo naik. Test di Task 1 dan Task 4.
3. **Saldo berjalan saat filter akun digabung filter lain** (mis. akun + pencarian teks), dua transaksi di tanggal sama, dan transaksi terhapus → `runningBalance` tetap saldo riil akun setelah transaksi itu dan tidak menghitung transaksi terhapus. Test di Task 3.
4. **Pencarian berisi `%`, `_`, atau huruf besar** (`"50%"`, `"INDO"`) → cocok literal tanpa membedakan huruf, bukan wildcard. Test di Task 3.
5. **Hapus lalu undo setelah daftar di-scroll ke halaman 2, atau saat filter aktif** → baris kembali dan saldo sidebar ikut kembali, tanpa baris dobel. Test di Task 6 (store).

---

### Task 1: Form → postings di core

**Files:**
- Create: `crates/core/src/form.rs`
- Modify: `crates/core/src/txn.rs` (varian `TxnError`), `crates/core/src/lib.rs` (modul + re-export)

**Interfaces:**
- Produces:
  - `TxnError::SplitMismatch(i64)` (selisih = nominal − Σ split) dan `TxnError::NotPositive`.
  - Tipe dan fungsi berikut:

```rust
pub struct SplitLine { pub category_id: Uuid, pub amount: i64, pub memo: Option<String> }
pub struct Fee { pub amount: i64, pub category_id: Uuid }
pub enum FormBody {
    Income { account_id: Uuid, amount: i64, splits: Vec<SplitLine> },
    Expense { account_id: Uuid, amount: i64, splits: Vec<SplitLine> },
    Transfer { from: Uuid, to: Uuid, amount: i64, fee: Option<Fee> },
}
pub struct PostingSpec { pub account_id: Uuid, pub category_id: Option<Uuid>, pub amount: i64, pub memo: Option<String> }
impl FormBody { pub fn kind(&self) -> TxnKind }
pub fn form_postings(body: &FormBody) -> Result<Vec<PostingSpec>, TxnError>
```

- [ ] **Step 1: Write the failing tests** di `form.rs`:
  - `expense_negates_each_split`: Expense amount 75_000, splits [50_000 kat A, 25_000 kat B] → amounts `[-50_000, -25_000]` dengan kategori sesuai.
  - `income_keeps_sign`: Income 1_000 → `[+1_000]`.
  - `refund_is_negative_expense`: Expense −20_000, split [−20_000] → `[+20_000]`.
  - `split_total_must_match`: Expense 10_000, split [6_000, 3_999] → `Err(SplitMismatch(1))`.
  - `empty_splits_missing_category`: Expense dengan `splits: vec![]` → `Err(MissingCategory)`.
  - `zero_split_line_rejected`: split [10_000, 0] dengan amount 10_000 → `Err(ZeroAmount)`.
  - `transfer_with_fee`: from A, to B, 100_000, fee 6_500 kat F → `[A −100_000 none, B +100_000 none, A −6_500 F]`.
  - `transfer_requires_positive_amount_and_fee`: amount 0 atau −5 → `NotPositive`; fee 0 → `NotPositive`.
  - proptest `form_postings_validate`: Expense acak dengan 1–5 split positif yang dijumlahkan → `form_postings` → `TxnDraft` → `validate` → Ok, dan Σ amount == −total.
- [ ] **Step 2:** `cargo test -p nabungo-core form::` → FAIL (modul belum ada).
- [ ] **Step 3: Implement.** Nominal 0 pada income/expense → `ZeroAmount`. Overflow saat menjumlahkan split → `Overflow` (`checked_add`). `form_postings` tidak mengecek jenis kategori; itu tugas `validate` setelah ref di-resolve app.
- [ ] **Step 4:** `cargo test -p nabungo-core` → PASS.
- [ ] **Step 5: Commit** `feat(core): transaction form to postings with split and fee rules`

### Task 2: Storage — tag, replace/get txn, payee search, migrasi 0004

**Files:**
- Create: `crates/storage/migrations/0004_txn_indexes.sql`, `crates/storage/src/tag.rs`
- Modify:
  - `crates/storage/src/lib.rs` (`migrations()`, `pub mod tag`)
  - `crates/storage/src/txn.rs`
  - `crates/storage/src/payee.rs`

**Interfaces:**
- Consumes: `ValidTxn`, `map_unique`, `StorageError::Duplicate`.
- Produces:

```rust
// tag.rs
pub fn find_or_insert_tag(tx: &Transaction, name: &str) -> Result<Uuid>   // NOCASE reuse
pub fn set_txn_tags(tx: &Transaction, txn_id: Uuid, tags: &[Uuid]) -> Result<()>  // replace set
pub fn txn_tags(conn: &Connection, txn_id: Uuid) -> Result<Vec<String>>   // sorted by name
pub fn list_tags(conn: &Connection) -> Result<Vec<String>>                 // sorted NOCASE
// txn.rs
pub struct TxnRecord { pub id: Uuid, pub kind: TxnKind, pub date: Date, pub payee: Option<String>,
    pub note: Option<String>, pub deleted: bool, pub postings: Vec<PostingRecord> }
pub struct PostingRecord { pub account_id: Uuid, pub category_id: Option<Uuid>, pub amount: i64, pub memo: Option<String> }
pub fn get_txn(conn: &Connection, id: Uuid) -> Result<Option<TxnRecord>>   // postings in insert order
pub fn replace_txn(tx: &Transaction, id: Uuid, txn: &ValidTxn, payee_id: Option<Uuid>, now: &str) -> Result<bool> // false if missing/deleted
pub fn last_used_account(conn: &Connection) -> Result<Option<Uuid>>
pub fn last_fee_category(conn: &Connection) -> Result<Option<Uuid>>
// payee.rs
pub fn search_payees(conn: &Connection, query: &str, limit: u32) -> Result<Vec<String>>
```

- Migrasi `0004_txn_indexes.sql`:
  - `CREATE UNIQUE INDEX tag_name_nocase ON tag (name COLLATE NOCASE);`
  - `CREATE INDEX idx_txn_date_id ON txn (date DESC, id DESC) WHERE deleted_at IS NULL;`
  - `CREATE INDEX idx_txn_tag_tag ON txn_tag (tag_id);`

- [ ] **Step 1: Write the failing tests:**
  - `tag::tests::tags_reuse_case_insensitive`: `find_or_insert_tag("Liburan")` lalu `("liburan")` → id sama; `list_tags` = `["Liburan"]`.
  - `tag::tests::set_txn_tags_replaces`: set [a, b] lalu [b] → `txn_tags` = `["b"]`.
  - `txn::tests::replace_keeps_id_and_created_at`: insert expense, replace dengan 2 posting baru dan payee lain → `get_txn` mengembalikan postings baru, id sama, `created_at` tidak berubah (query langsung), `updated_at` = `now` baru. Replace pada txn terhapus → `false`.
  - `txn::tests::last_used_account_ignores_transfer_and_deleted`: expense di A, transfer B→C (lebih baru), expense terhapus di D (paling baru) → `Some(A)`.
  - `txn::tests::last_fee_category_from_latest_transfer`.
  - `payee::tests::search_payees_contains_nocase_recent_first`: "Indomaret" (txn 10 Okt), "Alfamart Indo" (txn 12 Okt) → query `"indo"` → `["Alfamart Indo", "Indomaret"]`.
- [ ] **Step 2:** `cargo test -p nabungo-storage` → FAIL (fungsi belum ada).
- [ ] **Step 3: Implement.**
  - `replace_txn` meng-`UPDATE` header (kind, date, payee_id, note, updated_at), lalu `DELETE FROM posting WHERE txn_id`, lalu insert ulang. Pakai helper insert posting bersama `insert_txn`.
  - `search_payees` meng-escape `%`, `_`, `\` dengan `ESCAPE '\'`.
  - `last_used_account` hanya mengembalikan akun aktif (`archived_at IS NULL`).
- [ ] **Step 4:** `cargo test -p nabungo-storage` → PASS, termasuk test migrasi yang sudah ada.
- [ ] **Step 5: Commit** `feat(storage): tags, replace/get txn, payee search, migration 0004`

### Task 3: Storage — query daftar transaksi

**Files:**
- Create: `crates/storage/src/txn_list.rs`
- Modify: `crates/storage/src/lib.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Default)]
pub struct TxnQuery { pub date_from: Option<Date>, pub date_to: Option<Date>, pub account_id: Option<Uuid>,
    pub category_id: Option<Uuid>, pub tag: Option<String>, pub payee_id: Option<Uuid>,
    pub amount_min: Option<i64>, pub amount_max: Option<i64>, pub search: Option<String>,
    pub limit: u32, pub offset: u32 }
pub struct TxnListRow { pub id: Uuid, pub kind: TxnKind, pub date: Date, pub payee: Option<String>,
    pub note: Option<String>, pub amount: i64, pub account_id: Uuid, pub to_account_id: Option<Uuid>,
    pub category_id: Option<Uuid>, pub split_count: u32, pub fee: i64, pub tags: Vec<String>,
    pub account_delta: Option<i64>, pub running_balance: Option<i64> }
pub struct TxnPage { pub rows: Vec<TxnListRow>, pub total: u64 }
pub fn list_txns(conn: &Connection, q: &TxnQuery) -> Result<TxnPage>
```

- [ ] **Step 1: Write the failing tests** di `txn_list.rs`. Fixture: akun A, B. Kategori Makan (child Kopi), Gaji. Txn (urut waktu):
  - opening A +1_000_000 (1 Okt)
  - expense A Kopi −25_000 payee "Kopi Kenangan" tag "kantor" (2 Okt)
  - income A Gaji +5_000_000 (2 Okt)
  - transfer A→B 200_000 fee 6_500 Biaya (3 Okt)
  - expense A split Makan −30_000 / Kopi −20_000 note "makan 50% diskon" (4 Okt)
  - expense A −99 (5 Okt, lalu dihapus)

  Test:
  - `default_order_and_total`: urutan date DESC, id DESC; `total` = 5 (yang terhapus tidak ikut).
  - `transfer_row_shape`: `amount` 200_000, `account_id` A, `to_account_id` B, `fee` 6_500, `category_id` None, `split_count` 0.
  - `split_row_shape`: `amount` −50_000, `category_id` None, `split_count` 2.
  - `category_filter_includes_children`: filter Makan → kopi-2-Okt dan split; filter Kopi → keduanya juga (split punya Kopi).
  - `running_balance_full_history`: filter A → baris 4 Okt `running_balance` = 1_000_000 − 25_000 + 5_000_000 − 206_500 − 50_000 = 5_718_500; baris transfer `account_delta` −206_500.
  - `running_balance_with_search_filter`: filter A + search "diskon" → 1 baris, `running_balance` tetap 5_718_500.
  - `search_literal_percent_and_case`: search "50%" → 1 baris; search "50_" → 0; search "KENANGAN" → 1.
  - `amount_range_uses_abs`: min 20_000 max 60_000 → kopi (25_000) dan split (50_000).
  - `tag_and_payee_and_date_filters`: tag "KANTOR" (NOCASE) → 1; `date_from=date_to=2026-10-02` → 2.
  - `pagination`: limit 2 offset 2 → baris ke-3 dan ke-4; `total` tetap 5.
  - `#[ignore] perf_50k_first_page`: 50.000 expense acak di 3 akun → `list_txns` limit 100 dengan filter akun < 200 ms (dijalankan dengan `cargo test --release -p nabungo-storage -- --ignored perf_`).
- [ ] **Step 2:** `cargo test -p nabungo-storage txn_list::` → FAIL.
- [ ] **Step 3: Implement.**
  - Satu SQL dengan CTE. `filtered` memilih id transaksi yang lolos filter. Bila `account_id` ada, `running` menghitung `SUM(delta) OVER (ORDER BY date, id ROWS UNBOUNDED PRECEDING)` atas seluruh transaksi aktif akun itu.
  - Hasilnya di-join, lalu `ORDER BY date DESC, id DESC LIMIT ? OFFSET ?`. `total` dari `COUNT(*)` atas `filtered`.
  - Tag dikumpulkan per baris dengan query kedua (`WHERE txn_id IN (...)`), bukan `GROUP_CONCAT`, supaya nama tag berisi koma aman.
- [ ] **Step 4:** `cargo test -p nabungo-storage` → PASS. Perf test ignored dijalankan sekali; catat waktunya di ledger.
- [ ] **Step 5: Commit** `feat(storage): filtered transaction list with running balance`

### Task 4: App — use case transaksi

**Files:**
- Create: `crates/app/src/transactions.rs`
- Modify:
  - `crates/app/src/ledger.rs`: `TxnInput.tags: Vec<String>`. `record` menyimpan tag. `resolve` menerima daftar ref yang boleh diarsip.
  - `crates/app/src/lib.rs`: re-export.

**Interfaces:**
- Consumes: `form_postings`, `FormBody`, storage Task 2–3.
- Produces:

```rust
pub struct TxnForm { pub body: FormBody, pub date: Date, pub payee: Option<String>, pub note: Option<String>, pub tags: Vec<String> }
pub struct TxnDetail { pub id: Uuid, pub kind: TxnKind, pub date: Date, pub payee: Option<String>, pub note: Option<String>,
    pub tags: Vec<String>, pub postings: Vec<PostingRecord> }
impl Ledger {
    pub fn create_txn(&mut self, form: &TxnForm) -> Result<Uuid, AppError>;
    pub fn update_txn(&mut self, id: Uuid, form: &TxnForm) -> Result<(), AppError>;
    pub fn get_txn(&self, id: Uuid) -> Result<TxnDetail, AppError>;        // NotFound("transaction") if missing/deleted
    pub fn list_txns(&self, q: &TxnQuery) -> Result<TxnPage, AppError>;
    pub fn search_payees(&self, query: &str) -> Result<Vec<String>, AppError>;  // limit 8
    pub fn list_tags(&self) -> Result<Vec<String>, AppError>;
    pub fn last_used_account(&self) -> Result<Option<Uuid>, AppError>;
    pub fn default_fee_category(&self) -> Result<Option<Uuid>, AppError>;
}
```

Re-export `FormBody`, `SplitLine`, `Fee`, `TxnKind`, `TxnQuery`, `TxnListRow`, `TxnPage`, `PostingRecord` dari `nabungo_app`.

- [ ] **Step 1: Write the failing tests** di `transactions.rs`:
  - `create_expense_with_split_tags_and_payee`: saldo turun 75_000. `get_txn` mengembalikan 2 posting, payee "Indomaret", tags `["belanja", "kantor"]` dari input `["#kantor", " belanja "]`.
  - `split_mismatch_writes_nothing`: `Err(Txn(SplitMismatch(1)))`. Jumlah txn, payee, dan tag tidak berubah.
  - `transfer_with_fee_moves_and_charges`: A 1_000_000, transfer ke B 200_000 fee 6_500 → saldo A 793_500, B 200_000.
  - `refund_split_raises_balance`.
  - `update_replaces_and_keeps_id`: ubah expense menjadi income kategori Gaji → saldo sesuai, id sama, tag diganti.
  - `update_on_archived_account_keeps_working`: arsip akun A setelah txn dibuat. Update hanya catatan → Ok. Update ke akun lain yang diarsip → `Archived("account")`.
  - `update_opening_rejected` → `OpeningNotAllowed`. `update_deleted` → `NotFound("transaction")`.
  - `delete_undo_roundtrip_restores_balance_and_list` (memakai `delete_txn`/`undo_delete_txn` yang sudah ada + `list_txns`).
  - `default_fee_category_falls_back_to_seed_name`: setelah onboarding (seed "Biaya admin") → kategori itu. Setelah transfer dengan fee kategori lain → kategori lain.
- [ ] **Step 2:** `cargo test -p nabungo-app transactions::` → FAIL.
- [ ] **Step 3: Implement.**
  - `create_txn`/`update_txn`: `form_postings` → resolve ref. Untuk update, ref yang dipakai versi lama boleh diarsip: kumpulkan id akun/kategori dari `get_txn` lama. Lalu `ValidTxn::new` → `resolve_payee` → insert/replace → tag (`normalize_tag` lalu `find_or_insert_tag`, dedupe) → `set_txn_tags`, semuanya dalam satu `self.conn.transaction()`.
  - `normalize_tag(&str) -> Result<String, AppError>`: trim, buang `#` di depan, `normalize_name`.
- [ ] **Step 4:** `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** `feat(app): create, update, get, list transactions with tags and fee default`

### Task 5: IPC — DTO, command, error code

**Files:**
- Modify:
  - `src-tauri/src/dto.rs`
  - `src-tauri/src/commands.rs`
  - `src-tauri/src/lib.rs` (`collect_commands!`)
  - `src-tauri/src/error.rs` (`TxnSplitMismatch`, `TxnNotPositive`)
  - `ui/src/ipc/bindings.ts` (generate)
  - `ui/src/i18n/id.ts`, `en.ts` (`errors.txn_split_mismatch`, `errors.txn_not_positive`)

**Interfaces:**
- Produces command (nama snake_case Rust → camelCase di bindings):
  - `create_txn(form: TxnFormDto) -> String`
  - `update_txn(id: String, form: TxnFormDto) -> ()`
  - `get_txn(id) -> TxnDetailDto`
  - `list_txns(query: TxnQueryDto) -> TxnPageDto`
  - `delete_txn(id)`
  - `undo_delete_txn(id)`
  - `payee_suggestion(name) -> Option<PayeeSuggestionDto>`
  - `search_payees(query) -> Vec<String>`
  - `list_tags() -> Vec<String>`
  - `txn_form_defaults() -> TxnFormDefaultsDto { lastAccountId: Option<String>, feeCategoryId: Option<String> }`
- DTO (serde `camelCase`, tanggal string `YYYY-MM-DD`, ID string):
  - `TxnFormDto { kind: TxnFormKindDto ('income'|'expense'|'transfer'), date, accountId, toAccountId?, amount, splits: SplitDto[] {categoryId, amount, memo?}, fee?: FeeDto {amount, categoryId}, payee?, note?, tags }`
  - `TxnDetailDto { id, kind: TxnKindDto (6 jenis), date, payee?, note?, tags, postings: PostingDto[] {accountId, categoryId?, amount, memo?} }`
  - `TxnQueryDto` (semua field opsional + `limit`, `offset`)
  - `TxnRowDto` (field `TxnListRow` camelCase)
  - `TxnPageDto { rows, total }`
- `TxnFormDto` → `TxnForm`: `kind` income/expense membutuhkan `splits`; transfer membutuhkan `toAccountId`, bila tidak ada → `invalid_id("account")`. ID salah → `invalid_id`, tanggal salah → `invalid_date`.

- [ ] **Step 1: Write the failing tests** di `dto.rs`:
  - `txn_form_dto_maps_transfer_with_fee`
  - `txn_form_dto_transfer_without_to_is_invalid_id`
  - `txn_row_serializes_camel_case` (`runningBalance`, `toAccountId`, `splitCount`)

  Di `error.rs`: `split_mismatch_maps`.
- [ ] **Step 2:** `cargo test -p nabungo-desktop` → FAIL.
- [ ] **Step 3: Implement.** Command tipis mengikuti pola `commands.rs` yang ada. Lalu `cargo test -p nabungo-desktop export_bindings`.
- [ ] **Step 4:** `cargo test --workspace` dan `pnpm --filter @nabungo/ui typecheck` → PASS. Typecheck memastikan kunci `errors` baru ada.
- [ ] **Step 5: Commit** `feat(desktop): IPC commands for transactions` (termasuk `bindings.ts`)

### Task 6: UI — facade IPC, browser-ledger, lib form, store transaksi

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/ipc/browser-ledger.ts`
- Create:
  - `ui/src/lib/txnForm.ts` + `.test.ts`
  - `ui/src/stores/transactions.ts` + `.test.ts`
  - `ui/src/ipc/browser-ledger.test.ts` (bila belum ada; bila ada, tambahkan)

**Interfaces:**
- Produces (`ui/src/ipc/index.ts`): `createTxn`, `updateTxn`, `getTxn`, `listTxns`, `deleteTxn`, `undoDeleteTxn`, `payeeSuggestion`, `searchPayees`, `listTags`, `txnFormDefaults` + re-export tipe DTO.
- `ui/src/lib/txnForm.ts`:

```ts
export type FormKind = 'income' | 'expense' | 'transfer'
export interface SplitRow { categoryId: string | null; amount: number; memo: string }
export interface TxnFormState { kind: FormKind; date: string; accountId: string | null; toAccountId: string | null;
  amount: number; split: boolean; splits: SplitRow[]; categoryId: string | null;
  feeEnabled: boolean; fee: number; feeCategoryId: string | null; payee: string; note: string; tags: string[] }
export function emptyForm(kind: FormKind, d: { accountId: string | null; feeCategoryId: string | null; today: string }): TxnFormState
export function toDto(s: TxnFormState): TxnFormDto            // non-split → one split {categoryId, amount}
export function fromDetail(d: TxnDetailDto): TxnFormState     // expense: amount = −Σ, split when >1 categorized posting
export function duplicateOf(d: TxnDetailDto, today: string): TxnFormState
export function splitRemainder(s: TxnFormState): number       // amount − Σ split rows (display only)
export function isEditable(kind: TxnKindDto): boolean         // income/expense/transfer
```

- `ui/src/stores/transactions.ts` (`useTransactionsStore`):
  - state `query: TxnQueryDto` tanpa limit/offset, `rows: TxnRowDto[]`, `total`, `loading`
  - `reload()` (offset 0, limit 100)
  - `loadMore()` (append; no-op bila `rows.length >= total` atau sedang loading)
  - `setQuery(patch)`
  - `remove(id)`: `deleteTxn` → `reload` + `accounts.load()` → toast dengan aksi `actions.undo` → `undoDeleteTxn` → `reload` + `accounts.load()`
  - `save(id | null, state)`: create/update → `reload` + `accounts.load()`

- [ ] **Step 1: Write the failing tests:**
  - `txnForm.test.ts`:
    - `toDto` non-split expense → `splits: [{categoryId, amount}]`.
    - `fromDetail` transfer dengan fee → `feeEnabled`, `fee`, `feeCategoryId`, `toAccountId` benar.
    - `fromDetail` refund expense (posting +20_000) → `amount` −20_000.
    - `duplicateOf` mengganti tanggal.
    - `splitRemainder`.
  - `browser-ledger.test.ts`, mencerminkan test Rust Task 3–4:
    - transfer fee → saldo A 793_500;
    - split mismatch → error `txn_split_mismatch`;
    - `listTxns` filter akun + search → `runningBalance` riwayat penuh;
    - search `"50%"` literal.
  - `transactions.test.ts`:
    - `loadMore` menambah halaman kedua tanpa dobel;
    - `remove` lalu undo setelah `loadMore` (2 halaman) → `rows` berisi baris itu tepat sekali dan `accounts.load` dipanggil 2×;
    - `setQuery` mereset offset.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui exec vitest run src/lib/txnForm.test.ts src/stores/transactions.test.ts src/ipc/browser-ledger.test.ts` → FAIL.
- [ ] **Step 3: Implement.** Browser-ledger menyimpan txn + postings + tags di memori. Validasi tanda/split/fee disalin dari core (`SplitMismatch`, `NotPositive`, `ZeroAmount`), lalu `listTxns` difilter dan diurutkan di memori. Toast undo memakai `toast(message, { action: { label, onClick } })` dari `vue-sonner`.
- [ ] **Step 4:** test di atas + `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit** `feat(ui): transaction IPC facade, browser ledger parity, form helpers and store`

### Task 7: UI — form transaksi (create, edit, duplikasi)

**Files:**
- Add shadcn: `CI=true pnpm dlx shadcn-vue@latest add combobox tags-input --yes` (periksa diff: Google Fonts & `cn`, lihat CLAUDE.md).
- Create:
  - `ui/src/components/txn/TxnForm.vue` + `TxnForm.test.ts`
  - `ui/src/components/txn/CategoryCombobox.vue`
  - `ui/src/components/txn/PayeeCombobox.vue`
  - `ui/src/components/txn/SplitEditor.vue`
- Modify:
  - `ui/src/components/NewTxnSheet.vue`
  - `ui/src/stores/ui.ts`: `txnEditor: { mode: 'create' | 'edit' | 'duplicate'; id?: string } | null`, `openNewTxn()`, `openEditTxn(id)`, `openDuplicateTxn(id)`; `newTxnOpen` menjadi computed dari `txnEditor`.
  - `ui/src/i18n/id.ts`, `en.ts` (`txn.*`; hapus `txn.formComingSoon`)

**Interfaces:**
- Consumes: Task 6 (`emptyForm`, `toDto`, `fromDetail`, `duplicateOf`, `splitRemainder`, store `save`), `AmountInput` (`commit()` sebelum submit, seperti fix Fase 3), stores accounts/categories.
- Produces: `TxnForm.vue` props `{ initial: TxnFormState; id: string | null }`, emits `saved`, `cancel`.

Detail form:
- **Tabs Income/Expense/Transfer.** Pindah tab mempertahankan nominal, tanggal, payee, catatan, dan tag; kategori di-reset.
- **Field urut Tab:**
  1. nominal (`AmountInput`, autofocus);
  2. kategori (`CategoryCombobox`, kind sesuai tab, child ditampilkan `Parent › Child`, arsip disembunyikan);
  3. akun (`Select`; transfer: dari + ke);
  4. tanggal;
  5. payee (`PayeeCombobox`: ketik bebas, saran dari `searchPayees`);
  6. tag (`TagsInput`, saran `listTags`);
  7. catatan.
- **Split:** tombol "Split" mengganti field kategori dengan `SplitEditor` (baris kategori + nominal + memo, tambah/hapus baris, sisa = `splitRemainder` ditampilkan berwarna bila ≠ 0).
- **Transfer:** toggle "Biaya admin" (Switch) → nominal fee + kategori, default `feeCategoryId`.
- **Ctrl+Enter** atau tombol Simpan → `await amountInput.commit()` untuk setiap `AmountInput` → `store.save(id, state)`. Error backend ditampilkan inline lewat `errorMessage`.
- **Sukses:** create menutup sheet dan menampilkan toast `txn.saved`; untuk create, fokus kembali ke elemen sebelum sheet dibuka.

- [ ] **Step 1: Write the failing tests** (`TxnForm.test.ts`, jsdom, browser-ledger dengan onboarding + akun BCA + kategori seed):
  - `quick_expense_in_five_interactions`:
    1. `ui.openNewTxn()`;
    2. ketik "25rb" di nominal;
    3. `Tab`;
    4. ketik "Kopi" lalu `Enter` di combobox;
    5. `Ctrl+Enter`.

    Hasilnya `listTxns()` berisi 1 expense −25_000 kategori Kopi akun BCA. Hitung interaksi = 5.
  - `split_mismatch_shows_inline_error_and_keeps_sheet_open`.
  - `transfer_fee_defaults_category`: toggle fee → kategori terisi "Biaya admin".
  - `payee_suggestion_fills_untouched_category_only`: transaksi sebelumnya payee "Starbucks" kat Kopi. Form baru: pilih payee "Starbucks" → kategori = Kopi. Bila user sudah memilih kategori lain sebelumnya → tidak ditimpa.
  - `edit_mode_prefills_and_updates`: `openEditTxn(id)` → nominal terisi → ubah → simpan → `getTxn` berubah, id sama.
  - `duplicate_uses_today`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui exec vitest run src/components/txn` → FAIL.
- [ ] **Step 3: Implement.** Sheet `side="right"` lebar `sm:max-w-lg`. Untuk edit/duplikasi, sheet memuat `getTxn` lalu `fromDetail`/`duplicateOf`; create memakai `emptyForm('expense', defaults)`.

  Default akun (urut prioritas):
  1. `txnFormDefaults().lastAccountId`, bila akun itu masih aktif;
  2. akun aktif non-liability pertama.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui test` dan `typecheck` → PASS.
- [ ] **Step 5: Commit** `feat(ui): keyboard-first transaction form with split, fee, payee and tags`

### Task 8: UI — layar Transaksi

**Files:**
- Add dependency: `pnpm --filter @nabungo/ui add @tanstack/vue-virtual`
- Create:
  - `ui/src/components/txn/TxnFilters.vue`
  - `ui/src/components/txn/TxnList.vue` + `TxnList.test.ts`
  - `ui/src/lib/txnList.ts` + `.test.ts`
- Modify:
  - `ui/src/views/TransactionsView.vue` (ganti placeholder)
  - `ui/src/i18n/id.ts`, `en.ts`

**Interfaces:**
- Consumes: store `transactions`, `ui.openEditTxn`/`openDuplicateTxn`, `MoneyText`, `isEditable`.
- Produces: `ui/src/lib/txnList.ts`:

```ts
export type ListItem = { type: 'date'; date: string; key: string } | { type: 'row'; row: TxnRowDto; key: string }
export function groupByDate(rows: TxnRowDto[]): ListItem[]   // header per tanggal baru, urutan dipertahankan
```

Detail layar:
- **Filter bar:**
  - pencarian (debounce 250 ms);
  - rentang tanggal (2× `Input type="date"`);
  - akun (`Select`, termasuk "Semua");
  - kategori (`CategoryCombobox` gabungan income + expense);
  - tag (`Select` dari `listTags`);
  - payee: tidak ada kontrol terpisah; cari payee lewat kotak pencarian (mencakup nama payee). `TxnQueryDto.payeeId` tetap ada di backend untuk layar Detail akun/laporan nanti;
  - nominal min/max (`AmountInput`);
  - tombol "Reset".

  Query tersimpan di store, bukan di URL.
- **Baris:** ikon jenis, payee atau nama kategori, kategori (atau `txn.splitCount`), akun (transfer: `A → B` dengan ikon panah, nominal netral), tag sebagai `Badge`, nominal `MoneyText` (expense merah, income hijau, transfer netral). Bila filter akun aktif: kolom saldo berjalan.
- **Header tanggal:** format lokal (`12 Okt 2026`).
- **Keyboard:** baris `tabindex=0`. `Enter` = edit, `Ctrl+D` = duplikasi, `Delete` = hapus (undo toast); panah atas/bawah memindah fokus. Menu `DropdownMenu` per baris memuat Edit, Duplikasi, Hapus. Edit/Duplikasi nonaktif bila `!isEditable(kind)`.
- **Kosong:** pesan `txn.empty` + tombol "+ Transaksi".

- [ ] **Step 1: Write the failing tests:**
  - `txnList.test.ts`: `groupByDate` dengan 3 baris di 2 tanggal → `[date, row, row, date, row]`.
  - `TxnList.test.ts`, mount dengan store berisi baris dari browser-ledger:
    - transfer ditampilkan `BCA → Dompet`, nominal tanpa kelas `text-expense`;
    - kolom saldo berjalan muncul hanya saat `query.accountId` terisi;
    - `Delete` pada baris fokus → `store.remove` dipanggil dengan id;
    - `Enter` → `ui.txnEditor` = `{ mode: 'edit', id }`;
    - baris `opening` → item menu Edit `disabled`.
- [ ] **Step 2:** vitest file di atas → FAIL.
- [ ] **Step 3: Implement.**
  - `useVirtualizer` dengan `estimateSize` 56 (baris) / 32 (header) dan `overscan` 10.
  - `loadMore` saat item virtual terakhir ≥ `items.length − 20`.
  - Di jsdom virtualizer tidak mengukur. Test memakai `TxnList` dengan prop `virtual=false`, yang merender semua item; prop ini hanya untuk test dan didokumentasikan di komponen.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui test`, `typecheck`, `build` → PASS.
- [ ] **Step 5: Commit** `feat(ui): transactions screen with filters, virtualized list, running balance, undo`

### Task 9: E2E, CI, dokumentasi — exit Fase 4

**Files:**
- Create: `ui/playwright.config.ts`, `ui/e2e/transactions.spec.ts`
- Modify:
  - `ui/package.json`: devDependency `@playwright/test`; script `"e2e": "playwright test"`.
  - `ui/vite.config.ts`: `test.exclude` menambah `e2e/**` agar vitest tidak memungut spec Playwright.
  - `.github/workflows/ci.yml`: job `ui` menambah `pnpm --filter @nabungo/ui exec playwright install --with-deps chromium` dan `pnpm --filter @nabungo/ui e2e`.
  - `docs/plans/2026-10-12-v1-roadmap.md`: Fase 4 → selesai + "Keputusan saat eksekusi".
  - `CLAUDE.md`: perintah e2e, aturan `form_postings`, prop `virtual`.
  - `.gitignore`: `ui/test-results/`, `ui/playwright-report/`.

**Interfaces:**
- Consumes: semua task sebelumnya. Browser-ledger mulai kosong di setiap page load, jadi setiap test melakukan onboarding lewat UI.

- [ ] **Step 1: Write the e2e tests** (`playwright.config.ts`: `webServer: { command: 'pnpm dev', port: 5173, reuseExistingServer: true }`, `use.baseURL`, project chromium):
  - Helper `onboard(page)`: isi form onboarding (nama "BCA", saldo 1.000.000) → dasbor.
  - `expense via keyboard`: tekan `n` → ketik `25rb` → `Tab` → ketik `Kopi` → `Enter` → `Control+Enter` → buka `#/transactions` → baris "Kopi" dengan `−Rp 25.000` terlihat; saldo BCA di sidebar `Rp 975.000`.
  - `transfer with fee`: buat akun "Dompet" lewat layar Akun → `n` → tab Transfer → 200.000 dari BCA ke Dompet → aktifkan biaya 6.500 → simpan → sidebar: BCA `Rp 793.500`, Dompet `Rp 200.000`.
  - `delete and undo`: dari daftar, fokus baris expense → `Delete` → baris hilang dan saldo kembali → klik "Batalkan" di toast → baris kembali dan saldo `Rp 975.000`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui e2e` → semua test PASS. Test yang gagal adalah bug di task sebelumnya; perbaiki dengan test unit RED→GREEN dulu, bukan dengan melemahkan e2e.
- [ ] **Step 3:** Gate penuh: fmt, clippy, `cargo test --workspace`, typecheck, test, build, e2e. `git status` tidak menunjukkan diff `bindings.ts`.
- [ ] **Step 4:** Cek manual di `pnpm dev` (jendela Tauri, DB nyata user). Hanya baca dan catat; jangan hapus DB tanpa izin. Item yang dicek:
  - input expense dengan keyboard;
  - transfer + biaya;
  - undo;
  - scroll daftar.

  Bila tidak bisa dijalankan di sesi eksekusi, catat sebagai ruling dan sebutkan di roadmap.
- [ ] **Step 5:** Perbarui roadmap dan CLAUDE.md.
- [ ] **Step 6: Commit** `test(e2e): transaction flows; docs: mark Fase 4 done`
