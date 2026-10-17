# Fase 3 — Akun & Kategori Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** User bisa onboarding (bahasa, tanggal awal periode budget, akun pertama), mengelola akun + account group (urutan drag-and-drop, arsip), melihat saldo per akun dan net worth di sidebar/dasbor, serta mengelola pohon kategori 2 level (default, CRUD, ikon/warna, merge, arsip). Backend saran kategori dari payee (CAT-03) siap dipakai form transaksi Fase 4.

**Architecture:** Aturan murni (normalisasi nama, kedalaman pohon kategori, syarat merge, net worth) masuk `nabungo-core` modul baru `catalog` dan `networth`. `nabungo-storage` mendapat repository lengkap untuk akun, account group, kategori, payee + migrasi `0003` (indeks unik case-insensitive); pelanggaran UNIQUE dipetakan ke `StorageError::Duplicate`. `nabungo-app` mendapat blok `impl Ledger` per area (`accounts.rs`, `categories.rs`, `payee.rs`, `onboarding.rs`), satu use case tulis = satu SQLite transaction. `src-tauri` menambah DTO + command tipis; UI mendapat store Pinia, layar Onboarding/Akun/Kategori, saldo di sidebar, dan kartu net worth.

**Tech Stack:** Rust 2024 (rusqlite, rusqlite_migration, time, uuid, thiserror, specta/tauri-specta yang sudah ada); Vue 3, shadcn-vue (tambah `dropdown-menu alert-dialog switch label badge collapsible tabs popover`), `@vueuse/integrations` (`useSortable`) + `sortablejs`, `@lucide/vue`, Vitest.

**Spec:** `PRD.md` §4.1, §4.3, §4.6 (payee), §5 (ACC-01, ACC-02, ACC-03, CAT-01, CAT-02, CAT-03), §6 (skema), §9 (layar Onboarding, Akun & group, Kategori, sidebar). Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 3.

## Global Constraints

- Arah dependensi tetap: `src-tauri` hanya bergantung pada `nabungo-app`; tipe core yang dibutuhkan DTO di-re-export dari `nabungo_app`.
- Aturan bisnis di Rust (core/app), bukan di UI dan bukan trigger SQL. UI boleh validasi awal, tapi backend tetap menolak.
- Saldo dan net worth selalu diturunkan dari `posting` (transaksi `deleted_at IS NULL AND status = 'posted'`), tidak pernah disimpan.
- v1 hanya IDR (D4): akun baru selalu `CurrencyCode::IDR`; DTO tidak menerima currency.
- Akun/kategori yang terpakai tidak dihapus: hapus → `in_use`, UI menawarkan arsip (akun) atau arsip/merge (kategori).
- Migrasi `0001`/`0002` tidak diubah; tambah `0003_catalog_indexes.sql` dan daftarkan di `migrations()`.
- Nama (akun, group, kategori, payee) di-trim; kosong ditolak; maksimum 80 karakter (`chars().count()`).
- Semua command baru didaftarkan di `collect_commands!`; `ui/src/ipc/bindings.ts` di-regenerate dan di-commit.
- Semua pesan UI ada di `id` dan `en`; setiap `ErrorCode` baru ada di `errors` (`id.ts` `satisfies Record<ErrorCode, string>`).
- Komponen tidak memanggil `commands.*`/`invoke()`; semuanya lewat `ui/src/ipc/index.ts`. Fallback browser (`ui/src/ipc/browser.ts`) mengimplementasikan setiap command baru di memori.
- Navigasi keyboard penuh: setiap aksi drag-and-drop punya padanan keyboard (menu "Pindah ke atas/bawah").
- Setiap task diakhiri `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test` hijau.

## Keputusan desain

- **Jenis & currency akun tidak bisa diubah setelah dibuat.** Mengubah `investment → bank` akan membuat transaksi valuation lama melanggar invariant 3. UI menampilkan jenis sebagai read-only saat edit.
- **Saldo awal bisa diedit** lewat form akun: transaksi `opening` diubah di tempat (nominal + tanggal), dibuat bila sebelumnya 0, dan di-hard-delete bila menjadi 0. Opening bukan transaksi user, jadi tidak butuh undo.
- **Arsip tidak mengeluarkan akun dari net worth.** Uangnya tetap ada; yang mengatur hanya `include_in_net_worth`. UI memperingatkan saat mengarsip akun bersaldo ≠ 0.
- **Net worth:** `assets` = Σ saldo akun non-liability, `liabilities` = −Σ saldo akun liability (tagihan kartu kredit bersaldo −500.000 → liabilities 500.000; kartu kelebihan bayar → liabilities negatif), `net = assets − liabilities`. Hanya akun `include_in_net_worth`, termasuk yang diarsip.
- **Hapus akun** hanya bila akun tidak punya posting dari transaksi selain `opening` — termasuk transaksi yang sudah di-soft-delete (supaya undo tetap valid). Saat dihapus: transaksi opening + `allocation_mapping` ikut dihapus.
- **Hapus group** memindahkan akunnya ke "Tanpa grup" (`group_id = NULL`), tidak pernah diblokir.
- **Kategori:** kedalaman maksimal 2; child wajib sejenis dengan parent; kategori yang punya child tidak bisa dipindah ke bawah parent lain. Arsip parent ikut mengarsip child; unarchive child ditolak (`archived`) bila parent masih diarsip.
- **Merge kategori** (`source → target`): sejenis, bukan diri sendiri, target aktif, source tanpa child (merge child dulu). Semua posting (termasuk transaksi terhapus), `payee.default_category_id`, dan `budget` dipindah; budget di periode yang sama dijumlahkan (rollover ikut target). Source lalu dihapus.
- **Keunikan nama case-insensitive** ditegakkan oleh indeks unik di migrasi `0003` (group, payee, kategori sesama parent/root per jenis). Nama akun boleh kembar.
- **Kategori default di-seed saat onboarding** dalam bahasa yang dipilih, hanya bila tabel `category` kosong. Setelah itu nama adalah data user.
- **CAT-03 di fase ini hanya backend:** `TxnInput.payee`, upsert payee, dan `Ledger::payee_suggestion`. Command IPC + pemakaian di form ditambahkan Fase 4 bersama form transaksi.
- **Onboarding** ditandai setting `onboarding.completed_at`; selama belum selesai, router mengarahkan semua route ke `/onboarding` (layout tanpa sidebar, shortcut global nonaktif). Tanggal awal periode juga bisa diubah di Pengaturan.
- **Ikon & warna**: backend menyimpan string apa adanya; UI memilih dari daftar tetap (`ui/src/lib/icons.ts`, `ui/src/lib/colors.ts`). Nilai tak dikenal dirender sebagai ikon/warna default.

## Review Focus

1. Arsip akun yang masih bersaldo → net worth tidak berubah; akun hilang dari sidebar utama tapi muncul di bagian "Diarsip". Test di Task 3.
2. Kartu kredit kelebihan bayar (saldo positif) dan akun liability `include_in_net_worth = false` → angka net worth sesuai rumus di atas, tidak dobel/terbalik tanda. Test di Task 1 & 3.
3. Nama beda kapitalisasi/spasi (`"Harian "` vs `"harian"`, `"Makan"` vs `"makan"` di parent yang sama, payee `"Indomaret"` vs `"indomaret"`) → ditolak `duplicate` atau dipakai ulang (payee), bukan error `internal`. Test di Task 2, 4, 5.
4. Merge kategori ketika source & target sama-sama punya budget di periode yang sama, atau source dipakai transaksi yang sudah dihapus → tidak crash UNIQUE, undo hapus transaksi tetap menunjuk kategori valid. Test di Task 4.
5. Onboarding dipanggil dua kali / app ditutup di tengah onboarding → tidak ada kategori dobel; pemanggilan kedua `already_onboarded`; onboarding yang gagal (mis. nama akun kosong) tidak meninggalkan setting/kategori setengah jadi. Test di Task 6.

---

### Task 1: Aturan katalog & net worth di core

**Files:**
- Create: `crates/core/src/catalog.rs`, `crates/core/src/networth.rs`
- Modify: `crates/core/src/lib.rs` (modul + re-export)

**Interfaces:**
- Produces (`catalog`):
  - `pub const MAX_NAME_LEN: usize = 80;`
  - `pub enum CatalogError { EmptyName, NameTooLong, InvalidStatementDay(u8), InvalidCreditLimit, CategoryDepth, CategoryKindMismatch, CategoryHasChildren, MergeIntoSelf }` (derive `Debug, Clone, Copy, PartialEq, Eq, thiserror::Error`)
  - `pub fn normalize_name(input: &str) -> Result<String, CatalogError>` — trim; kosong → `EmptyName`; > 80 karakter → `NameTooLong`.
  - `pub fn validate_statement_day(day: Option<u8>) -> Result<Option<u8>, CatalogError>` — `1..=31`.
  - `pub fn validate_credit_limit(limit: Option<i64>) -> Result<Option<i64>, CatalogError>` — `0..=MAX_AMOUNT`.
  - `pub struct ParentInfo { pub kind: CategoryKind, pub has_parent: bool }`
  - `pub fn check_parent(kind: CategoryKind, parent: Option<ParentInfo>, has_children: bool) -> Result<(), CatalogError>` — urutan cek: `parent.has_parent` → `CategoryDepth`; `parent.kind != kind` → `CategoryKindMismatch`; `parent.is_some() && has_children` → `CategoryDepth`.
  - `pub struct MergeSide { pub id: Uuid, pub kind: CategoryKind, pub has_children: bool }`
  - `pub fn check_merge(source: MergeSide, target: MergeSide) -> Result<(), CatalogError>` — urutan: id sama → `MergeIntoSelf`; jenis beda → `CategoryKindMismatch`; `source.has_children` → `CategoryHasChildren`.
- Produces (`networth`): `pub struct NetWorth { pub assets: i64, pub liabilities: i64, pub net: i64 }` (derive `Debug, Clone, Copy, PartialEq, Eq, Default`); `pub fn net_worth(items: impl IntoIterator<Item = (AccountKind, i64)>) -> Result<NetWorth, MoneyError>` — akumulasi `i128`, `MoneyError::Overflow` bila hasil tidak muat `i64`. Pemanggil yang menyaring `include_in_net_worth`.

- [ ] **Step 1: Tulis test yang gagal**
  - `catalog::tests::normalize_name_table`: `"  BCA "` → `Ok("BCA")`; `""`, `"   "` → `EmptyName`; `"a".repeat(80)` ok; `"a".repeat(81)` → `NameTooLong`; `"é".repeat(80)` ok (hitung karakter, bukan byte).
  - `statement_day_and_credit_limit_bounds`: `Some(0)`, `Some(32)` → `InvalidStatementDay`; `Some(1)`, `Some(31)`, `None` ok; limit `Some(-1)`, `Some(MAX_AMOUNT + 1)` → `InvalidCreditLimit`; `Some(0)`, `None` ok.
  - `check_parent_rules`: root (`None`) ok; parent root sejenis ok; parent yang punya parent → `CategoryDepth`; parent beda jenis → `CategoryKindMismatch`; kategori ber-child dipindah ke bawah parent → `CategoryDepth`.
  - `check_merge_rules`: diri sendiri → `MergeIntoSelf`; beda jenis → `CategoryKindMismatch`; source ber-child → `CategoryHasChildren`; valid → `Ok`.
  - `networth::tests::assets_minus_liabilities`: `[(Bank, 1_000_000), (Ewallet, 250_000), (CreditCard, -400_000), (Loan, -1_000_000)]` → `{ assets: 1_250_000, liabilities: 1_400_000, net: -150_000 }`.
  - `overpaid_credit_card_reduces_liabilities`: `[(Bank, 100), (CreditCard, 50)]` → `{ assets: 100, liabilities: -50, net: 150 }`.
  - `empty_is_zero` dan `overflow_is_error`: dua `(Bank, i64::MAX)` → `Err(MoneyError::Overflow)`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-core catalog:: networth::` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces; re-export `CatalogError, MergeSide, ParentInfo, MAX_NAME_LEN, normalize_name, check_parent, check_merge, validate_statement_day, validate_credit_limit, NetWorth, net_worth` dari `nabungo_core`.
- [ ] **Step 4: Jalankan, pastikan lulus** — perintah yang sama → PASS.
- [ ] **Step 5: Commit** — `feat(core): catalog naming/tree rules and net worth`

---

### Task 2: Storage akun, account group, saldo, migrasi 0003

**Files:**
- Create: `crates/storage/migrations/0003_catalog_indexes.sql`, `crates/storage/src/account_group.rs`
- Modify: `crates/storage/src/lib.rs` (migrasi, `pub mod account_group;`, `StorageError::Duplicate`, helper `map_unique`), `crates/storage/src/account.rs`, `crates/storage/src/txn.rs` (pemanggil `NewAccount` di test), `crates/app/src/ledger.rs` + `crates/app/tests/ledger_balance.rs` (pemanggil `NewAccount` — disesuaikan sementara, diganti di Task 3)

**Interfaces:**
- Migrasi `0003`:
  ```sql
  CREATE UNIQUE INDEX account_group_name_nocase ON account_group (name COLLATE NOCASE);
  CREATE UNIQUE INDEX payee_name_nocase ON payee (name COLLATE NOCASE);
  CREATE UNIQUE INDEX category_child_name_nocase ON category (parent_id, name COLLATE NOCASE) WHERE parent_id IS NOT NULL;
  CREATE UNIQUE INDEX category_root_name_nocase ON category (kind, name COLLATE NOCASE) WHERE parent_id IS NULL;
  CREATE INDEX idx_txn_payee ON txn (payee_id) WHERE deleted_at IS NULL;
  ```
- `StorageError::Duplicate` (`#[error("duplicate name")]`); `pub(crate) fn map_unique(e: rusqlite::Error) -> StorageError` — `SQLITE_CONSTRAINT_UNIQUE` (extended code 2067) → `Duplicate`, selain itu `Sqlite(e)`. Dipakai di semua insert/update nama.
- `account.rs`:
  - `NewAccount { name, kind, currency, group_id: Option<Uuid>, include_in_net_worth: bool, credit_limit: Option<i64>, statement_day: Option<u8>, icon: Option<String>, color: Option<String> }` + `NewAccount::new(name: impl Into<String>, kind, currency) -> Self` (group `None`, include `true`, sisanya `None`). `insert_account` menyimpan semua kolom, `sort_order = MAX(sort_order)+1` dalam group yang sama.
  - `pub struct AccountPatch { pub name: String, pub group_id: Option<Uuid>, pub include_in_net_worth: bool, pub credit_limit: Option<i64>, pub statement_day: Option<u8>, pub icon: Option<String>, pub color: Option<String> }`
  - `pub struct AccountRow { pub id: Uuid, pub name: String, pub kind: AccountKind, pub currency: CurrencyCode, pub group_id: Option<Uuid>, pub include_in_net_worth: bool, pub credit_limit: Option<i64>, pub statement_day: Option<u8>, pub icon: Option<String>, pub color: Option<String>, pub sort_order: i64, pub archived: bool, pub balance: i64, pub opening: Option<Opening> }`; `pub struct Opening { pub txn_id: Uuid, pub amount: i64, pub date: Date }`
  - `pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountRow>>` — satu query dengan subquery saldo `GROUP BY account_id` (aturan sama dengan `account_balance`) + `LEFT JOIN` opening aktif; `ORDER BY sort_order, name`.
  - `pub fn update_account(tx: &Transaction, id: Uuid, patch: &AccountPatch, now: &str) -> Result<bool>`
  - `pub fn set_account_archived(tx: &Transaction, id: Uuid, archived: bool, now: &str) -> Result<bool>`
  - `pub fn set_account_position(tx: &Transaction, id: Uuid, group_id: Option<Uuid>, sort_order: i64, now: &str) -> Result<bool>`
  - `pub fn has_non_opening_postings(conn: &Connection, id: Uuid) -> Result<bool>` — termasuk transaksi soft-deleted.
  - `pub fn delete_account(tx: &Transaction, id: Uuid) -> Result<bool>` — hapus `txn` opening milik akun (CASCADE posting), `allocation_mapping`, lalu akun.
- `account_group.rs`: `pub struct AccountGroupRow { pub id: Uuid, pub name: String, pub sort_order: i64 }`; `insert_group(tx, name: &str) -> Result<Uuid>` (sort_order = max+1); `rename_group(tx, id, name) -> Result<bool>`; `delete_group(tx, id) -> Result<bool>` (set `account.group_id = NULL` dulu); `set_group_order(tx, id, sort_order: i64) -> Result<bool>`; `group_exists(conn, id) -> Result<bool>`; `list_groups(conn) -> Result<Vec<AccountGroupRow>>` (`ORDER BY sort_order, name`).

- [ ] **Step 1: Tulis test yang gagal** (di modul masing-masing, `open_in_memory()`):
  - `account::tests::list_accounts_has_balance_and_opening`: akun A dengan opening 500.000 tgl `2026-10-01` + transfer −200.000 ke B, satu transaksi lain di A di-soft-delete → A `balance = 300_000`, `opening = Some(Opening { amount: 500_000, date: 2026-10-01, .. })`; B `opening = None`, `balance = 200_000`; akun tanpa posting `balance = 0`.
  - `update_and_archive_roundtrip`: `update_account` mengubah nama/group/include/icon → terbaca di `list_accounts`; `set_account_archived(true)` → `archived`; `false` → aktif; ID tak dikenal → `false`.
  - `delete_account_removes_opening_and_mapping`: akun dengan opening + baris `allocation_mapping` → `has_non_opening_postings = false`, `delete_account` → `true`, tabel `txn`/`posting`/`allocation_mapping` untuk akun itu kosong.
  - `soft_deleted_txn_counts_as_usage`: transaksi adjustment di akun lalu `soft_delete` → `has_non_opening_postings = true`.
  - `account_group::tests::group_crud_and_order`: insert "Harian", "Tabungan" → sort_order 0,1; rename; `set_group_order` membalik urutan → `list_groups` mengikuti.
  - `group_names_unique_case_insensitive`: insert "Harian" lalu "harian" → `Err(StorageError::Duplicate)`; rename "Tabungan" → "HARIAN" → `Duplicate`.
  - `delete_group_ungroups_accounts`: akun di group → `delete_group` → akun `group_id = None`, akun tetap ada.
  - `lib::tests::migration_0003_indexes_exist`: keempat indeks unik ada di `sqlite_schema`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-storage` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces; perbarui pemanggil `NewAccount { .. }` lama ke `NewAccount::new(..)`.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test --workspace` → PASS (test lama tetap hijau).
- [ ] **Step 5: Commit** — `feat(storage): account and account group repositories with balances`

---

### Task 3: Use case akun & group (ACC-01, ACC-02, ACC-03)

**Files:**
- Create: `crates/app/src/accounts.rs`
- Modify: `crates/app/src/ledger.rs` (hapus `create_account` lama; tambah varian `AppError`), `crates/app/src/lib.rs` (re-export), `crates/app/tests/ledger_balance.rs`

**Interfaces:**
- Consumes: Task 1 (`normalize_name`, `validate_*`, `net_worth`), Task 2 (repository akun/group).
- `AppError` tambahan: `Catalog(#[from] CatalogError)`, `InUse(&'static str)`, `Duplicate(&'static str)`, `Money(#[from] MoneyError)`, `AlreadyOnboarded` (dipakai Task 6). `StorageError::Duplicate` dipetakan di titik panggil ke `AppError::Duplicate("account_group" | "category" | "payee")`.
- Produces:
  - `pub struct AccountFields { pub name: String, pub group_id: Option<Uuid>, pub include_in_net_worth: bool, pub credit_limit: Option<i64>, pub statement_day: Option<u8>, pub icon: Option<String>, pub color: Option<String>, pub opening_balance: i64, pub opening_date: Date }` + `AccountFields::new(name: impl Into<String>, opening_balance: i64, opening_date: Date) -> Self`.
  - `pub struct AccountsOverview { pub groups: Vec<AccountGroupRow>, pub accounts: Vec<AccountRow>, pub net_worth: NetWorth }`
  - di `impl Ledger`:
    - `pub fn create_account(&mut self, kind: AccountKind, fields: &AccountFields) -> Result<Uuid, AppError>` — currency IDR; `credit_limit`/`statement_day` disimpan `None` bila `kind != CreditCard`; group tak dikenal → `NotFound("account_group")`. Logika dipindah ke `pub(crate) fn insert_account_tx(tx: &Transaction, kind, fields, now: &str) -> Result<Uuid, AppError>` agar onboarding (Task 6) bisa memakainya dalam transaksinya sendiri.
    - `pub fn update_account(&mut self, id: Uuid, fields: &AccountFields) -> Result<(), AppError>` — mengubah opening sesuai Keputusan desain (validasi lewat `ValidTxn::new` draft `Opening`).
    - `pub fn archive_account(&mut self, id: Uuid)`, `pub fn unarchive_account(&mut self, id: Uuid)`, `pub fn delete_account(&mut self, id: Uuid)` → `Result<(), AppError>`; delete terpakai → `InUse("account")`.
    - `pub fn reorder_accounts(&mut self, group_id: Option<Uuid>, ordered: &[Uuid]) -> Result<(), AppError>` — set `group_id` + `sort_order = index` untuk setiap ID, satu transaksi; ID tak dikenal → `NotFound("account")` dan tidak ada yang berubah.
    - `pub fn create_group(&mut self, name: &str) -> Result<Uuid, AppError>`, `rename_group(&mut self, id: Uuid, name: &str)`, `delete_group(&mut self, id: Uuid)`, `reorder_groups(&mut self, ordered: &[Uuid])` → `Result<(), AppError>`.
    - `pub fn accounts_overview(&self) -> Result<AccountsOverview, AppError>` — net worth dari akun `include_in_net_worth` (termasuk diarsip).
  - Re-export dari `nabungo_app`: `AccountFields, AccountsOverview, AccountRow, AccountGroupRow, Opening, NetWorth, CatalogError, MoneyError, AccountKind, CategoryKind`.

- [ ] **Step 1: Tulis test yang gagal** (`accounts.rs`):
  - `create_with_attributes_and_opening`: kartu kredit `credit_limit Some(10_000_000)`, `statement_day Some(17)`, opening −2.000.000 → overview menampilkan atribut, `balance = -2_000_000`, `net_worth.liabilities = 2_000_000`.
  - `credit_fields_cleared_for_non_credit_card`: bank dengan `credit_limit Some(1)` → tersimpan `None`.
  - `name_is_normalized_and_validated`: `"  BCA  "` → `"BCA"`; `"   "` → `Catalog(EmptyName)`; `statement_day Some(32)` → `Catalog(InvalidStatementDay(32))`.
  - `edit_opening_balance`: opening 100 → update 250 (tgl baru) → `balance = 250`, satu txn opening; update 0 → tidak ada txn opening, `balance = 0`; update lagi 75 → txn opening dibuat ulang.
  - `archive_keeps_net_worth`: akun 1.000.000 diarsip → `net_worth.net` tetap 1.000.000, `accounts[..].archived = true`; `record` ke akun itu → `Archived("account")`.
  - `excluded_account_not_in_net_worth`: liability −300.000 dengan `include_in_net_worth = false` → tidak masuk `liabilities`.
  - `delete_unused_account`: akun hanya dengan opening → `delete_account` ok, hilang dari overview.
  - `delete_used_account_is_in_use`: akun dengan expense → `InUse("account")`; setelah expense di-`delete_txn` tetap `InUse("account")`.
  - `reorder_across_groups`: dua group, `reorder_accounts(Some(g2), &[b, a])` → a & b di g2 dengan sort_order 1 dan 0; ID tak dikenal → `NotFound("account")`, posisi lama utuh.
  - `group_duplicate_and_delete`: `create_group("Harian")` lalu `create_group(" harian ")` → `Duplicate("account_group")`; `delete_group` → akunnya `group_id = None`; create akun dengan group acak → `NotFound("account_group")`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-app accounts::` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces; perbarui `ledger.rs` tests dan `tests/ledger_balance.rs` ke `create_account(kind, &AccountFields::new(..))`.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** — `feat(app): account and account group use cases with net worth`

---

### Task 4: Kategori — CRUD, arsip, merge, seed default (CAT-01, CAT-02)

**Files:**
- Create: `crates/app/src/categories.rs`, `crates/app/src/defaults.rs`
- Modify: `crates/storage/src/category.rs`, `crates/app/src/ledger.rs` (hapus `create_category` lama), `crates/app/src/lib.rs`, pemanggil lama di `txn.rs`/`ledger.rs`/`tests/ledger_balance.rs`

**Interfaces:**
- Storage (`category.rs`):
  - `pub struct NewCategory { pub kind: CategoryKind, pub name: String, pub parent_id: Option<Uuid>, pub icon: Option<String>, pub color: Option<String>, pub sort_order: i64 }`; `insert_category(tx, new: &NewCategory) -> Result<Uuid>` (ganti signature lama; `map_unique`).
  - `pub struct CategoryRow { pub id: Uuid, pub parent_id: Option<Uuid>, pub kind: CategoryKind, pub name: String, pub icon: Option<String>, pub color: Option<String>, pub sort_order: i64, pub archived: bool, pub in_use: bool }` — `in_use` = ada posting/budget/payee default yang menunjuknya.
  - `list_categories(conn) -> Result<Vec<CategoryRow>>` (`ORDER BY sort_order, name`); `update_category(tx, id, name: &str, parent_id: Option<Uuid>, icon: Option<&str>, color: Option<&str>) -> Result<bool>`; `set_category_archived(tx, id, archived: bool, now: &str) -> Result<bool>` (juga child-nya bila `archived = true`); `child_count(conn, id) -> Result<i64>`; `is_referenced(conn, id) -> Result<bool>` (posting apa pun termasuk transaksi terhapus, budget, payee default); `merge_category(tx, source, target) -> Result<()>` (pindah posting & payee; budget: `INSERT … ON CONFLICT(category_id, period) DO UPDATE SET amount = amount + excluded.amount` dari baris source, lalu hapus baris source; hapus source); `delete_category(tx, id) -> Result<bool>`.
- App:
  - `pub struct CategoryFields { pub name: String, pub parent_id: Option<Uuid>, pub icon: Option<String>, pub color: Option<String> }` + `CategoryFields::named(name: impl Into<String>) -> Self`.
  - `impl Ledger`: `create_category(&mut self, kind: CategoryKind, fields: &CategoryFields) -> Result<Uuid, AppError>`; `update_category(&mut self, id: Uuid, fields: &CategoryFields)`; `archive_category(&mut self, id: Uuid)`; `unarchive_category(&mut self, id: Uuid)` (parent diarsip → `Archived("category")`); `delete_category(&mut self, id: Uuid)` (dipakai atau punya child → `InUse("category")`); `merge_category(&mut self, source: Uuid, target: Uuid)` (target diarsip → `Archived("category")`) → `Result<(), AppError>`; `list_categories(&self) -> Result<Vec<CategoryRow>, AppError>`. Parent tak dikenal → `NotFound("category")`. Aturan pohon lewat `check_parent`/`check_merge` (Task 1).
  - `defaults.rs`: `pub(crate) fn seed_default_categories(tx: &Transaction, locale: Locale) -> Result<bool, AppError>` — no-op (`false`) bila sudah ada kategori. Daftar (nama ID / EN, ikon lucide kebab-case, `sort_order` = urutan di tabel):

    | Jenis | Parent (ikon) | Child |
    |---|---|---|
    | expense | Makan & Minum / Food & Drink (`utensils`) | Restoran / Restaurants; Belanja dapur / Groceries; Kopi & jajan / Coffee & Snacks |
    | expense | Transportasi / Transportation (`car`) | Bensin / Fuel; Transportasi umum / Public Transit; Ojek online / Ride-hailing; Parkir & tol / Parking & Tolls |
    | expense | Tagihan / Bills (`receipt`) | Listrik / Electricity; Air / Water; Internet / Internet; Pulsa & data / Mobile Plan |
    | expense | Belanja / Shopping (`shopping-bag`) | Pakaian / Clothing; Elektronik / Electronics; Rumah tangga / Household |
    | expense | Kesehatan / Health (`heart-pulse`) | Dokter & obat / Doctor & Medicine; Asuransi / Insurance |
    | expense | Hiburan / Entertainment (`clapperboard`) | Langganan / Subscriptions; Hobi / Hobbies; Liburan / Travel |
    | expense | Pendidikan / Education (`graduation-cap`) | — |
    | expense | Keluarga / Family (`users`) | — |
    | expense | Biaya admin / Fees (`landmark`) | — |
    | expense | Lainnya / Other (`circle-ellipsis`) | — |
    | income | Gaji / Salary (`wallet`) | — |
    | income | Bonus / Bonus (`gift`) | — |
    | income | Bunga & dividen / Interest & Dividends (`trending-up`) | — |
    | income | Hadiah / Gifts (`hand-heart`) | — |
    | income | Lainnya / Other (`circle-ellipsis`) | — |

    Child mewarisi ikon parent. Warna `None`.

- [ ] **Step 1: Tulis test yang gagal** (`categories.rs`, kecuali disebut lain):
  - `create_child_rules`: child di bawah root sejenis ok; child di bawah child → `Catalog(CategoryDepth)`; income di bawah expense → `Catalog(CategoryKindMismatch)`; parent acak → `NotFound("category")`.
  - `sibling_names_unique_case_insensitive`: "Makan" lalu " makan " di root expense → `Duplicate("category")`; "Makan" di income ok; "Kopi" di dua parent berbeda ok.
  - `move_parent_with_children_rejected`: root ber-child di-update ke bawah root lain → `Catalog(CategoryDepth)`.
  - `archive_cascades_and_unarchive_child_needs_parent`: arsip parent → child ikut `archived`; `unarchive_category(child)` → `Archived("category")`; unarchive parent lalu child → ok.
  - `delete_rules`: kategori tak terpakai → terhapus; dipakai expense yang sudah di-`delete_txn` → `InUse("category")`; punya child → `InUse("category")`.
  - `merge_moves_everything`: expense di source (satu di-`delete_txn`), payee dengan default source, budget `2026-11` source 100.000 + target 50.000 dan `2026-12` source 30.000 → setelah merge: source tidak ada, kedua posting menunjuk target, `undo_delete_txn` berhasil, payee default = target, budget target `2026-11` = 150.000 & `2026-12` = 30.000.
  - `merge_rules`: diri sendiri → `Catalog(MergeIntoSelf)`; beda jenis → `Catalog(CategoryKindMismatch)`; source ber-child → `Catalog(CategoryHasChildren)`; target diarsip → `Archived("category")`.
  - `defaults::tests::seed_in_locale_once`: seed `Locale::En` di DB kosong → `true`, 15 root (10 expense, 5 income) + 19 child, ada root expense "Fees"; seed kedua → `false`, jumlah tetap; seed `Locale::Id` → ada "Makan & Minum" dengan child "Kopi & jajan".
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-app categories:: defaults::` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces; perbarui pemanggil `create_category(kind, name)` lama ke `create_category(kind, &CategoryFields::named(..))`.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** — `feat(app): category tree CRUD, archive, merge, and default seed`

---

### Task 5: Payee & saran kategori (CAT-03, backend)

**Files:**
- Create: `crates/storage/src/payee.rs`, `crates/app/src/payee.rs`
- Modify: `crates/storage/src/lib.rs`, `crates/storage/src/txn.rs` (`insert_txn` menerima `payee_id`), `crates/app/src/ledger.rs` (`TxnInput.payee`, `record`), `crates/app/src/lib.rs`

**Interfaces:**
- Storage: `pub fn find_payee(conn, name: &str) -> Result<Option<Uuid>>` (`WHERE name = ?1 COLLATE NOCASE`); `pub fn insert_payee(tx, name: &str) -> Result<Uuid>`; `pub fn payee_default_category(conn, id) -> Result<Option<Uuid>>`; `pub fn last_postings_for_payee(conn, payee_id) -> Result<Vec<(Uuid /*account*/, Option<Uuid> /*category*/)>>` — posting dari transaksi aktif terbaru (`ORDER BY t.date DESC, t.created_at DESC`, posting `ORDER BY p.id`). `insert_txn(tx, txn: &ValidTxn, payee_id: Option<Uuid>, now: &str)`.
- App: `TxnInput.payee: Option<String>` (helper `single`/`transfer` mengisi `None`). `record` menormalisasi nama (kosong/spasi → tanpa payee; > 80 karakter → `Catalog(NameTooLong)`), memakai payee yang ada (case-insensitive) atau membuat baru, dalam transaksi yang sama.
  - `pub struct PayeeSuggestion { pub account_id: Option<Uuid>, pub category_id: Option<Uuid> }`
  - `impl Ledger`: `pub fn payee_suggestion(&self, name: &str) -> Result<Option<PayeeSuggestion>, AppError>` — `None` bila payee tak dikenal. `category_id` = `default_category_id` bila aktif, selain itu kategori pertama yang aktif dari transaksi terakhir; `account_id` = akun pertama yang aktif dari transaksi terakhir. Kategori/akun yang diarsip dilewati (jadi `None`).

- [ ] **Step 1: Tulis test yang gagal** (`app/src/payee.rs`):
  - `record_reuses_payee_case_insensitive`: dua expense dengan payee `"Indomaret"` dan `" indomaret "` → satu baris `payee`, kedua txn menunjuknya; payee `"  "` → `payee_id NULL`.
  - `suggestion_from_last_txn`: expense di akun A kategori Makan (10 Okt) lalu akun B kategori Belanja (12 Okt) → saran `{ account_id: B, category_id: Belanja }`; transaksi 12 Okt di-`delete_txn` → saran kembali ke A/Makan.
  - `default_category_wins_and_archived_skipped`: set `default_category_id` lewat SQL → saran memakainya; kategori itu diarsip → jatuh ke kategori transaksi terakhir; akun terakhir diarsip → `account_id: None`.
  - `unknown_payee_has_no_suggestion` → `Ok(None)`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-app payee::` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** — `feat(app): payee upsert on record and category suggestion`

---

### Task 6: Onboarding & tanggal awal periode budget

**Files:**
- Create: `crates/app/src/onboarding.rs`
- Modify: `crates/app/src/settings.rs` (period start day), `crates/app/src/lib.rs`

**Interfaces:**
- Consumes: `insert_account_tx` (Task 3), `seed_default_categories` (Task 4), `PeriodStartDay` (core).
- Produces:
  - `settings.rs`: key `budget.period_start_day`; `pub fn period_start_day(&self) -> Result<PeriodStartDay, AppError>` (hilang/tak valid → `PeriodStartDay::FIRST`); `pub fn set_period_start_day(&mut self, day: PeriodStartDay) -> Result<(), AppError>`. Re-export `PeriodStartDay, PeriodError` dari `nabungo_app`.
  - `pub struct OnboardingInput { pub locale: Locale, pub period_start_day: PeriodStartDay, pub account_kind: AccountKind, pub account: AccountFields }`
  - `impl Ledger`: `pub fn is_onboarded(&self) -> Result<bool, AppError>` (key `onboarding.completed_at` ada); `pub fn complete_onboarding(&mut self, input: &OnboardingInput) -> Result<Uuid, AppError>` — sudah onboarded → `AlreadyOnboarded`; satu transaksi: set locale, set start day, seed kategori (locale), buat akun, set `onboarding.completed_at = now_utc()`; mengembalikan ID akun.

- [ ] **Step 1: Tulis test yang gagal**
  - `settings::tests::period_start_day_default_and_roundtrip`: default `1`; set 25 → 25; nilai `'99'` dan `'abc'` di tabel → `1`.
  - `onboarding::tests::complete_onboarding_sets_everything`: `is_onboarded = false`; complete (`En`, 25, bank "BCA" opening 1.000.000) → `is_onboarded = true`, `ui_settings().locale = En`, `period_start_day = 25`, ada kategori "Salary", akun BCA bersaldo 1.000.000.
  - `second_call_is_already_onboarded`: panggilan kedua → `AlreadyOnboarded`; jumlah kategori & akun tidak berubah.
  - `failed_onboarding_leaves_nothing`: nama akun `"  "` → `Catalog(EmptyName)`; `is_onboarded = false`, kategori 0, setting `ui.locale` & `budget.period_start_day` tidak ada; panggilan ulang dengan input valid berhasil.
  - `existing_categories_are_kept`: kategori "Kopi" dibuat sebelum onboarding → setelah onboarding hanya ada "Kopi" (seed dilewati).
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-app onboarding:: settings::` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test --workspace` → PASS.
- [ ] **Step 5: Commit** — `feat(app): onboarding with category seed and budget period start day`

---

### Task 7: IPC — DTO, command, kode error, bindings

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/error.rs`, `src-tauri/src/lib.rs`, `ui/src/ipc/bindings.ts` (generate), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts` (kunci `errors` baru)

**Interfaces:**
- DTO (`#[serde(rename_all = "camelCase")]`, ID = `String` UUID, tanggal = `String` `YYYY-MM-DD`, uang = `i64`):
  - `AccountKindDto` (`snake_case`, 7 varian), `CategoryKindDto` (`income | expense`) + `From` dua arah.
  - `AccountFieldsDto { name, groupId: Option<String>, includeInNetWorth: bool, creditLimit: Option<i64>, statementDay: Option<u8>, icon: Option<String>, color: Option<String>, openingBalance: i64, openingDate: String }`
  - `AccountDto { id, name, kind, groupId, includeInNetWorth, creditLimit, statementDay, icon, color, sortOrder: i64, archived: bool, balance: i64, openingBalance: i64, openingDate: Option<String> }` (`openingBalance` 0 bila tidak ada opening)
  - `AccountGroupDto { id, name, sortOrder }`, `NetWorthDto { assets, liabilities, net }`, `AccountsOverviewDto { groups, accounts, netWorth }`
  - `CategoryFieldsDto { name, parentId: Option<String>, icon, color }`, `CategoryDto { id, parentId, kind, name, icon, color, sortOrder, archived, inUse }`
  - `OnboardingDto { locale: LocaleDto, periodStartDay: u8, accountKind: AccountKindDto, account: AccountFieldsDto }`
- `ErrorCode` baru (snake_case): `in_use`, `duplicate`, `already_onboarded`, `invalid_id`, `invalid_date`, `invalid_start_day`, `name_empty`, `name_too_long`, `invalid_statement_day`, `invalid_credit_limit`, `category_depth`, `category_kind_mismatch`, `category_has_children`, `merge_into_self`. `in_use`/`duplicate`/`invalid_id` membawa `entity`. `AppError::Money` → `internal`. Pemetaan `CatalogError → ErrorCode` tanpa wildcard (pola `From<TxnError>`). ID tak bisa di-parse → `invalid_id` (entity = nama entitas); tanggal tak valid → `invalid_date`; `PeriodError` → `invalid_start_day`.
- Command (semua `Result<_, IpcError>`, nama Rust = snake_case berikut, binding camelCase):
  `get_onboarding_status() -> bool`, `complete_onboarding(input: OnboardingDto) -> String`, `get_period_start_day() -> u8`, `set_period_start_day(day: u8)`,
  `accounts_overview() -> AccountsOverviewDto`, `create_account(kind: AccountKindDto, fields: AccountFieldsDto) -> String`, `update_account(id: String, fields: AccountFieldsDto)`, `archive_account(id)`, `unarchive_account(id)`, `delete_account(id)`, `reorder_accounts(group_id: Option<String>, ids: Vec<String>)`,
  `create_account_group(name: String) -> String`, `rename_account_group(id, name)`, `delete_account_group(id)`, `reorder_account_groups(ids: Vec<String>)`,
  `list_categories() -> Vec<CategoryDto>`, `create_category(kind: CategoryKindDto, fields: CategoryFieldsDto) -> String`, `update_category(id, fields)`, `archive_category(id)`, `unarchive_category(id)`, `delete_category(id)`, `merge_category(source: String, target: String)`.

- [ ] **Step 1: Tulis test yang gagal** (`error.rs`, `dto.rs`):
  - `catalog_errors_map`: `AppError::Catalog(CatalogError::CategoryDepth)` → `category_depth`; `AppError::InUse("account")` → `{ code: in_use, entity: "account" }`; `AppError::Duplicate("category")` → `duplicate` + entity.
  - `dto::tests::account_fields_parse`: `openingDate: "2026-13-01"` → `invalid_date`; `groupId: "nope"` → `invalid_id` entity `account_group`; DTO valid → `AccountFields` dengan `Date` & `Uuid` benar.
  - `account_kind_dto_serializes_snake_case`: `AccountKindDto::CreditCard` → `"credit_card"`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-desktop` → gagal compile.
- [ ] **Step 3: Implementasi** DTO, konversi (`TryFrom<AccountFieldsDto> for AccountFields` dst.), command, pendaftaran di `collect_commands!`; tambah terjemahan `errors.*` di `id.ts` & `en.ts` (mis. `in_use: '{entity} masih dipakai'`, `duplicate: 'Nama {entity} sudah ada'`, `category_depth: 'Kategori hanya boleh dua tingkat'`, `merge_into_self: 'Kategori tidak bisa digabung dengan dirinya sendiri'`).
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test -p nabungo-desktop` (termasuk `export_bindings` → `bindings.ts` ter-update) lalu `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 5: Commit** — `feat(desktop): IPC commands for accounts, categories, onboarding`

---

### Task 8: UI fondasi — facade IPC, fallback browser, store, onboarding

**Files:**
- Create: `ui/src/ipc/browser-ledger.ts`, `ui/src/stores/accounts.ts`, `ui/src/stores/categories.ts`, `ui/src/lib/accounts.ts`, `ui/src/lib/categories.ts`, `ui/src/lib/icons.ts`, `ui/src/lib/colors.ts`, `ui/src/views/OnboardingView.vue`, `ui/src/components/AmountInput.vue`, test `*.test.ts` untuk lib & store & guard
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/router.ts` (+ `router.test.ts`), `ui/src/App.vue`, `ui/src/views/SettingsView.vue`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `ui/package.json`

**Interfaces:**
- Facade: satu fungsi per command Task 7 dengan nama camelCase yang sama (`accountsOverview()`, `createAccount(kind, fields)`, …, `getOnboardingStatus()`, `completeOnboarding(input)`), semuanya lewat `unwrap`; re-export tipe DTO.
- `browser-ledger.ts`: implementasi in-memory semua command baru (tanpa validasi domain kecuali `in_use` untuk akun/kategori yang punya saldo/dipakai = tidak pernah di browser, jadi hapus selalu berhasil); `getOnboardingStatus` awal `false`; `completeOnboarding` mengisi beberapa kategori default contoh.
- `lib/accounts.ts`: `export interface GroupSection { group: AccountGroupDto | null; accounts: AccountDto[]; subtotal: number }`; `export function groupAccounts(o: AccountsOverviewDto, opts: { archived: boolean }): GroupSection[]` — urut group `sortOrder`, "Tanpa grup" (`group: null`) terakhir dan hanya bila berisi; `archived: false` membuang akun diarsip, `true` hanya akun diarsip dalam satu section `group: null`; subtotal = jumlah integer `balance`.
- `lib/categories.ts`: `export interface CategoryNode { category: CategoryDto; children: CategoryDto[] }`; `export function buildTree(list: CategoryDto[], kind: CategoryKindDto, opts: { archived: boolean }): CategoryNode[]`; `export function mergeTargets(list: CategoryDto[], source: CategoryDto): CategoryDto[]` (sejenis, aktif, bukan source).
- `lib/icons.ts`: `export const ICONS: Record<string, Component>` (semua ikon di tabel default Task 4 + `banknote, credit-card, piggy-bank, smartphone, building-2, coins, hand-coins`) dan `export function iconFor(name: string | null): Component` (fallback `circle`). `lib/colors.ts`: `export const COLORS = ['slate','red','orange','amber','green','teal','sky','blue','violet','pink'] as const` + `export function colorClass(c: string | null): string`.
- Store: `useAccountsStore` (`overview`, `load()`, dan aksi pembungkus tiap command yang memanggil `load()` setelah sukses); `useCategoriesStore` (`list`, `load()`, aksi serupa).
- `AmountInput.vue`: `v-model: number`, menampilkan teks, memanggil `parseAmount` saat blur/Enter, emit error lewat `errorMessage`.
- Router: route `{ path: '/onboarding', name: 'onboarding', meta: { bare: true } }`; `export function onboardingGuard(isOnboarded: () => Promise<boolean>): NavigationGuard` — belum onboarded → redirect `onboarding`; sudah → `onboarding` dialihkan ke `dashboard`. `RouteName` + `nav` i18n untuk `onboarding`.
- `App.vue`: `route.meta.bare` → render `RouterView` saja (tanpa sidebar, palette, sheet) dan abaikan shortcut global.
- `OnboardingView.vue`: satu halaman 3 bagian — bahasa (ganti langsung via `settings.setLocale`), tanggal awal periode (Select 1–28, default 1), akun pertama (nama, jenis, saldo awal `AmountInput`); submit `completeOnboarding` → `router.replace('/')`.
- `SettingsView.vue`: tambah Select "Tanggal awal periode budget" (1–28).
- Dependensi: `CI=true pnpm dlx shadcn-vue@latest add dropdown-menu alert-dialog switch label badge collapsible tabs popover --yes`; `pnpm --filter @nabungo/ui add @vueuse/integrations@^15 sortablejs` (+ `@types/sortablejs` dev). Pin versi lebih lama bila ditolak `minimumReleaseAge`.

- [ ] **Step 1: Tulis test yang gagal**
  - `lib/accounts.test.ts`: overview 2 group + 1 akun tanpa group + 1 akun diarsip → `archived: false` menghasilkan 3 section berurutan, section `null` terakhir, akun diarsip tidak muncul, subtotal benar (termasuk saldo negatif); `archived: true` → 1 section berisi akun diarsip; group kosong tetap tampil dengan subtotal 0.
  - `lib/categories.test.ts`: `buildTree` memisah jenis, child di bawah parent, urut `sortOrder`; child diarsip tidak tampil saat `archived: false`; `mergeTargets` tidak memuat source, jenis lain, atau kategori diarsip.
  - `router.test.ts`: 10 route; `onboardingGuard(async () => false)` mengarahkan `/accounts` → `{ name: 'onboarding' }`; `async () => true` mengarahkan `/onboarding` → `{ name: 'dashboard' }` dan membiarkan `/accounts`.
  - `stores/accounts.test.ts` (fallback browser): `createAccountGroup('Harian')` lalu `createAccount('bank', …)` → `overview.groups` & `overview.accounts` ter-update tanpa `load()` manual.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui test` → FAIL.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck` → PASS. Cek manual `pnpm --filter @nabungo/ui dev`: app terbuka di `/onboarding`, selesai → dasbor, tanpa error console.
- [ ] **Step 5: Commit** — `feat(ui): onboarding flow, account/category stores and IPC facade`

---

### Task 9: UI layar Akun, saldo di sidebar, net worth di dasbor

**Files:**
- Create: `ui/src/components/AccountFormDialog.vue`, `ui/src/components/IconColorPicker.vue`, `ui/src/components/ConfirmDialog.vue`, `ui/src/components/MoneyText.vue`
- Modify: `ui/src/views/AccountsView.vue`, `ui/src/components/AppSidebar.vue`, `ui/src/views/DashboardView.vue`, `ui/src/App.vue` (`accounts.load()` setelah onboarded), i18n `id.ts`/`en.ts`

**Interfaces:**
- Consumes: `useAccountsStore`, `groupAccounts`, `iconFor`/`colorClass`, `AmountInput` (Task 8).
- `MoneyText.vue`: props `amount: number`, `kind?: 'neutral' | 'expense' | 'income'`; merender `formatIdr` dengan token warna semantik yang sudah ada; saldo negatif akun liability → `expense`.
- `AccountFormDialog.vue`: props `account?: AccountDto` (edit) / tanpa (buat); field nama, jenis (Select, read-only saat edit), group (Select + "Tanpa grup"), saldo awal (`AmountInput`), tanggal saldo awal (`<input type="date">`, default hari ini), `includeInNetWorth` (Switch), `creditLimit` + `statementDay` (hanya kartu kredit), ikon & warna (`IconColorPicker`). `Ctrl+Enter` simpan. Error backend tampil inline via `errorMessage`.
- `AccountsView.vue`: tombol "Akun baru" & "Grup baru"; per group: header (nama, subtotal, menu Ubah nama/Hapus/Pindah atas/bawah), daftar akun drag-and-drop via `useSortable` (juga lintas group → `reorderAccounts(targetGroupId, idsBaru)`); menu per akun: Ubah, Pindah ke atas/bawah, Arsipkan, Hapus. Hapus yang gagal `in_use` → `ConfirmDialog` "Akun ini punya transaksi. Arsipkan saja?" → `archiveAccount`. Arsip akun bersaldo ≠ 0 → konfirmasi bahwa saldo tetap dihitung di net worth. Bagian "Diarsip" (Collapsible) dengan aksi Pulihkan.
- `AppSidebar.vue`: di bawah grup navigasi, blok net worth (`netWorth.net`) lalu `Collapsible` per group (dari `groupAccounts(…, { archived: false })`) berisi akun + `MoneyText` saldo, subtotal di header group.
- `DashboardView.vue`: kartu net worth: aset, liabilitas, net (ganti placeholder versi core).

- [ ] **Step 1: Tulis test yang gagal** — `components/AccountFormDialog.test.ts` (Vue Test Utils + fallback browser): field kartu kredit hanya muncul untuk jenis `credit_card`; mode edit menonaktifkan Select jenis; submit dengan nama kosong menampilkan pesan `errors.name_empty` tanpa memanggil `createAccount`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui exec vitest run src/components/AccountFormDialog.test.ts` → FAIL.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — test di atas + `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck` → PASS. Cek manual di `pnpm dev`: buat 2 group & 3 akun (satu kartu kredit), seret akun antar group, restart app → urutan bertahan; net worth sidebar = dasbor; hapus akun bertransaksi → ditawari arsip.
- [ ] **Step 5: Commit** — `feat(ui): accounts screen with groups, drag-and-drop, sidebar balances, net worth`

---

### Task 10: UI layar Kategori

**Files:**
- Create: `ui/src/components/CategoryFormDialog.vue`, `ui/src/components/MergeCategoryDialog.vue`
- Modify: `ui/src/views/CategoriesView.vue`, `ui/src/App.vue` (`categories.load()` setelah onboarded), i18n `id.ts`/`en.ts`

**Interfaces:**
- Consumes: `useCategoriesStore`, `buildTree`, `mergeTargets`, `IconColorPicker`, `ConfirmDialog` (Task 8–9).
- `CategoriesView.vue`: `Tabs` Pengeluaran / Pemasukan; pohon `buildTree` (parent + child terindentasi, ikon & warna); tombol "Kategori baru" dan aksi "Tambah sub-kategori" per parent; menu per kategori: Ubah, Gabungkan…, Arsipkan, Hapus. Hapus gagal `in_use` → `ConfirmDialog` dengan pilihan Arsipkan atau Gabungkan. Bagian "Diarsip" dengan Pulihkan.
- `CategoryFormDialog.vue`: props `kind`, `category?`, `parentId?`; field nama, parent (Select root sejenis + "Tanpa induk"; nonaktif bila kategori punya child), ikon & warna.
- `MergeCategoryDialog.vue`: props `source: CategoryDto`; Select target dari `mergeTargets`; teks konfirmasi "Semua transaksi dan budget {source} dipindah ke {target}. {source} akan dihapus."

- [ ] **Step 1: Tulis test yang gagal** — `components/MergeCategoryDialog.test.ts`: opsi target tidak memuat source/kategori diarsip/jenis lain; konfirmasi memanggil `store.merge(source.id, target.id)` sekali.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui exec vitest run src/components/MergeCategoryDialog.test.ts` → FAIL.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — test di atas + seluruh suite UI + typecheck → PASS. Cek manual di `pnpm dev`: kategori default tampil dalam bahasa onboarding; buat child, gabungkan, arsip parent (child ikut), hapus kategori terpakai → ditawari arsip/gabung.
- [ ] **Step 5: Commit** — `feat(ui): categories screen with tree, merge, and archive`

---

### Task 11: Verifikasi exit Fase 3 & dokumentasi

**Files:**
- Modify: `docs/plans/2026-10-12-v1-roadmap.md` (Fase 3 → selesai + "Keputusan saat eksekusi"; catatan CAT-03 UI pindah ke Fase 4), `CLAUDE.md` (bila ada konvensi baru, mis. `StorageError::Duplicate` dan blok `impl Ledger` per area)

- [ ] **Step 1: Jalankan seluruh gate** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui build` → semua hijau; `git status` tidak menunjukkan diff di `ui/src/ipc/bindings.ts`.
- [ ] **Step 2: Cek manual exit criteria** di `pnpm dev` dengan DB baru (hapus/rename `~/.local/share/nabungo/nabungo.db` dulu, setelah dikonfirmasi user): onboarding → dasbor; hapus akun terpakai diblokir dan menawarkan arsip; hapus kategori terpakai diblokir dan menawarkan arsip/merge (acceptance "hapus aman"); semua aksi bisa dilakukan tanpa mouse.
- [ ] **Step 3: Perbarui roadmap & CLAUDE.md**.
- [ ] **Step 4: Commit** — `docs: mark Fase 3 done`
