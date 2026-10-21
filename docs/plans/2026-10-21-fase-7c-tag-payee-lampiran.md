# Fase 7c — Tag, Payee, Lampiran Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:**
- CAT-04: **kelola tag dan payee**. Buat, ubah nama, gabungkan, hapus. Payee juga bisa diberi kategori default. Ada layar baru "Tag & Payee" dengan jumlah pemakaian dan pintasan "Lihat transaksi".
- TRX-08: **lampiran struk** (gambar/PDF) per transaksi. File disalin ke folder data `attachments/`, dan DB hanya menyimpan metadata + sha256. Lampiran ikut backup manual, restore dari file, dan ekspor.

**Architecture:**
- `nabungo-storage`:
  - Migrasi `0007_tags_attachments.sql`: indeks unik NOCASE untuk nama tag, kolom `attachment.size_bytes`/`created_at`, dan indeks `attachment`.
  - Fungsi repository CRUD tag/payee di `tag.rs`/`payee.rs`.
  - Repository baru `attachment.rs`.
  - `TxnListRow.attachment_count`.
- `nabungo-app`:
  - Use case tag di file baru `tags.rs`; use case payee diperluas di `payee.rs`.
  - File baru `attachments.rs`: deteksi jenis, salin konten ke penyimpanan berbasis sha256, hapus file yang tidak lagi dirujuk.
  - `backup.rs` dan `export.rs` diperluas agar membawa lampiran.
  - Folder lampiran selalu dikirim pemanggil sebagai `&Path`, mengikuti pola `backups/`.
- `src-tauri`:
  - `DataDir::attachments_path()`, DTO, dan command.
  - Plugin `tauri-plugin-opener` (dipanggil dari Rust saja) untuk membuka lampiran dengan aplikasi bawaan OS.
- UI:
  - View baru `LabelsView` (`/labels`).
  - Seksi lampiran di `TxnForm` (mode edit) dan ikon klip di `TxnList`.
  - Chip filter payee di `TxnFilters`.
  - Catatan enkripsi di `SecuritySection`.
  - `browser-ledger.ts` meniru semuanya; file lampiran disimulasikan.

**Tech Stack:**
- Rust, dengan dependensi baru `sha2 = "0.10"` (sudah ada di `Cargo.lock` sebagai dependensi transitif) di `crates/app` dan `tauri-plugin-opener = "2"` di `src-tauri`.
- Vue 3 + shadcn-vue (komponen yang sudah ada), Vitest, Playwright.

**Spec:**
- `PRD.md`:
  - CAT-04, TRX-08.
  - §3 (Tag, payee).
  - Skema `payee`, `tag`, `txn_tag`, `attachment`.
  - §6 invariant 5 (tag dalam transaction yang sama).
  - Layar Form transaksi ("lampiran").
  - DAT-01/DAT-02.
  - Prinsip "Data milik user".
- Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 7, baris 7c ("Lampiran disalin ke folder data; ikut backup/ekspor").
- Desain: `docs/design/ui-guidelines.md` §7.4 (sidebar), §8.4 (sheet transaksi), §8.5 (baris transaksi), §8.9 (dialog), §9.4 (Kategori, sebagai pola layar daftar).

**Di luar plan ini:**
- Pratinjau thumbnail gambar di app.
- Lampiran saat membuat transaksi baru (sebelum disimpan).
- Enkripsi file lampiran.
- Garbage collection file yatim di luar alur hapus lampiran (K5).
- Kelola kontak.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Lampiran = file di `<data>/attachments/<sha256>`** (hex huruf kecil, tanpa ekstensi). DB menyimpan `attachment(id, txn_id, file_name, sha256, mime, size_bytes, created_at)`. Konten yang sama dipakai bersama (dedup), dan satu file fisik bisa dirujuk banyak baris. Penulisan memakai `<sha256>.partial` lalu `rename`. File yang sudah ada tidak ditulis ulang. | Pilihan user (sesuai PRD/roadmap): DB tetap kecil. Dedup membuat backup otomatis di folder data yang sama tetap valid tanpa menyalin. |
| K2 | **Jenis file dideteksi dari magic bytes**, bukan ekstensi: JPEG `FF D8 FF` → `image/jpeg`, PNG `89 50 4E 47 0D 0A 1A 0A` → `image/png`, WebP `RIFF????WEBP` → `image/webp`, PDF `%PDF-` → `application/pdf`. Jenis lain ditolak dengan `AppError::AttachmentType` (kode `attachment_type`). Ukuran maksimum **10 MiB** (`10 * 1024 * 1024`); lebih dari itu ditolak dengan `AppError::AttachmentTooLarge` (kode `attachment_too_large`). File kosong dianggap jenis tidak dikenal. | Struk foto/scan. Batas ukuran mencegah folder data membengkak tanpa sengaja. |
| K3 | `file_name` = nama dasar file sumber. Kalau lebih dari 120 karakter, dipotong di batas karakter dengan ekstensi dipertahankan. Kalau kosong, dipakai `"lampiran"`. | Nama hanya untuk tampilan dan ekspor; isi diidentifikasi dari sha256. |
| K4 | Lampiran hanya bisa ditambah ke **transaksi aktif yang sudah tersimpan**. Transaksi terhapus memberi `NotFound("transaction")`. Mode buat baru di form menampilkan petunjuk "Simpan dulu untuk melampirkan struk". Satu pemanggilan bisa menambah beberapa file (dialog multi-pilih). Semua baris DB untuk satu pemanggilan ditulis dalam **satu** SQLite transaction. File disalin lebih dulu; kalau DB gagal, file baru yang tidak dirujuk dihapus lagi. | Menghindari state "lampiran tanpa transaksi". Undo hapus transaksi tidak menyentuh lampiran karena barisnya tidak dihapus saat soft delete. |
| K5 | **Hapus lampiran = hapus baris** (hard delete). File fisik ikut dihapus bila tidak ada baris lain, termasuk baris milik transaksi terhapus, yang merujuk sha256 yang sama. Backup lama yang merujuk file itu akan menampilkan lampiran sebagai "hilang" setelah restore. | User yang menghapus struk berharap filenya hilang (privasi), dan itu lebih penting daripada kelengkapan backup lama. |
| K6 | `list_attachments(txn_id)` mengembalikan `AttachmentRow { id, file_name, mime, size_bytes, missing: bool }`. `missing` = file `<dir>/<sha256>` tidak ada. `open_attachment_path(id, dir)` mengembalikan path, atau `AppError::AttachmentMissing` (kode `attachment_missing`) bila file tidak ada. | Folder data bisa tidak lengkap setelah restore backup lama atau pemindahan manual. |
| K7 | **Backup manual** (`backup_to(dest, attachments_dir)`): setelah `.db` ditulis, setiap sha256 yang dirujuk dan filenya ada disalin ke folder `<stem dest>-lampiran/` di samping `.db`. Folder itu dibuat hanya bila ada lampiran. Gagal menyalin satu file tidak membatalkan backup (log `warn`). **Restore dari file** (`restore_file(src, backup_dir, attachments_dir, now)`): bila folder `<stem src>-lampiran/` ada, setiap file di dalamnya yang namanya sha256 valid **dan** isinya cocok dengan sha256 tersebut disalin ke `attachments_dir` (lewati yang sudah ada). File yang tidak cocok dilewati. Backup otomatis/pre-restore/pre-migration tidak menyalin apa pun karena berbagi folder data. | Roadmap: "ikut backup". Backup manual adalah jalur pindah ke instalasi lain. |
| K8 | **Ekspor**: JSON per transaksi mendapat `attachments: [{ file_name, sha256, mime, size_bytes }]` (metadata saja, `VERSION` tetap 1 karena field tambahan). Folder CSV mendapat `attachments.csv` (header `txn_id,file_name,sha256,mime,size_bytes,file`) dan subfolder `lampiran/` berisi salinan `<sha256>.<ext>` (`jpg`/`png`/`webp`/`pdf`). Kolom `file` = path relatif `lampiran/<sha256>.<ext>`, atau kosong bila file sumber hilang (ekspor tetap jalan). Hanya lampiran milik transaksi aktif. | DAT-02 + "Data milik user". |
| K9 | Enkripsi (SEC-01) **tidak** mencakup file lampiran. `SecuritySection` menampilkan catatan tetap: "Lampiran struk disimpan sebagai file biasa di folder data dan tidak ikut terenkripsi." Konversi enkripsi tidak menyentuh `attachments/`. | Konsekuensi pilihan file di folder. User perlu tahu. |
| K10 | Membuka lampiran = command `open_attachment(id)`, yang memanggil `app.opener().open_path(path, None::<&str>)` dari Rust. Tidak ada izin opener untuk JS (capabilities tidak berubah). | Gambar dan PDF dibuka dengan viewer OS. Tidak memperluas permukaan API webview. |
| K11 | **Tag**: migrasi menambah `CREATE UNIQUE INDEX tag_name_nocase ON tag (name COLLATE NOCASE)`. Data lama aman karena `find_or_insert_tag` sudah mencari dengan NOCASE.<br>Use case:<br>• `list_tag_usage` (id, nama, jumlah transaksi **aktif**, urut nama NOCASE)<br>• `create_tag(name)`<br>• `rename_tag(id, name)`: boleh ganti huruf besar/kecil nama sendiri; nama milik tag lain → `Duplicate("tag")`<br>• `merge_tag(source, target)`: `txn_tag` dipindah dengan `INSERT OR IGNORE`, lalu source dihapus; source = target → `CatalogError::MergeIntoSelf`<br>• `delete_tag(id)`: hapus baris `txn_tag`, termasuk milik transaksi terhapus, lalu tag<br>Nama dinormalisasi dengan `normalize_tag` (buang `#` dan spasi). | CAT-04. Gabung eksplisit (bukan rename bentrok) meniru pola kategori. |
| K12 | **Payee**: <br>• `list_payee_usage`: id, nama, `default_category_id`, jumlah transaksi aktif, tanggal transaksi aktif terakhir; urut nama NOCASE<br>• `create_payee(name, default_category_id)`<br>• `update_payee(id, name, default_category_id)`: aturan rename sama dengan tag<br>• `merge_payee(source, target)`: `UPDATE txn SET payee_id = target` untuk **semua** transaksi source, termasuk yang terhapus, lalu source dihapus<br>• `delete_payee(id)`: `payee_id = NULL` pada semua transaksi, lalu payee dihapus<br>Kategori default harus ada (`NotFound("category")`) dan tidak diarsip (`Archived("category")`). | CAT-04. Transaksi terhapus ikut diperbarui agar undo tidak memulihkan FK yang menggantung. |
| K13 | Hapus tag/payee memakai dialog konfirmasi yang menyebut jumlah transaksi terdampak, tanpa undo. Tag/payee tidak pernah memengaruhi saldo. | Aksi katalog, bukan data uang. |
| K14 | UI: route `/labels` (`name: 'labels'`), item sidebar "Tag & Payee" tepat setelah "Kategori" tanpa shortcut. Header `PageHeader` memakai segmented `Tag \| Payee` dan tombol "+ Tag baru"/"+ Payee baru". Baris 48px berisi nama (`#nama` untuk tag), meta jumlah transaksi (+ kategori default untuk payee), dan menu `⋯` (Ubah, Gabungkan…, Lihat transaksi, Hapus). "Lihat transaksi" memanggil `useTransactionsStore().setQuery({ tag })`/`setQuery({ payeeId })` (filter lain direset) lalu pindah ke `/transactions`. `TxnFilters` mendapat chip `payeeId` (label nama payee). | Pola layar §9.4. Filter `payeeId` sudah ada di backend tapi belum di UI. |
| K15 | `TxnListRow.attachment_count: u32` (lampiran milik transaksi itu). `TxnList` menampilkan ikon `PaperclipIcon` (`data-testid="txn-attachment"`, aria-label "Ada lampiran") bila > 0. | Penanda di daftar (§8.5). |
| K16 | `browser-ledger.ts`: <br>• tag punya registri `state.tags` (nama) yang terisi saat transaksi disimpan dan dipakai `listTags`/`listTagUsage`, sehingga tag tak terpakai tetap tercantum seperti di Rust<br>• payee mendapat `defaultCategoryId`<br>• lampiran disimulasikan: `addAttachments(txnId)` menambah satu lampiran `struk-contoh.jpg` (`image/jpeg`, 1234 byte) per panggilan<br>• `openAttachment` tidak melakukan apa-apa<br>• aturan error sama dengan Rust | Mode browser/e2e tidak punya filesystem. Pola sama dengan simulasi enkripsi. |

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`. `src-tauri` tanpa logika bisnis.
- `nabungo-core` tanpa IO. Hashing dan deteksi jenis file ada di `nabungo-app`.
- Migrasi `0001`–`0006` tidak diubah. Migrasi baru `0007_tags_attachments.sql` didaftarkan di `migrations()` (`crates/storage/src/lib.rs`). Test yang mematok `latest_version()` (mis. di `file.rs`) dinaikkan ke 7.
- Repository tulis menerima `&Transaction`; baca boleh `&Connection`. Satu use case tulis = satu SQLite transaction.
- Folder lampiran tidak pernah diturunkan dari `Ledger.path`; selalu parameter `attachments_dir: &Path` (test memakai `tempfile::tempdir()`).
- Offline total: tidak ada aset remote. Plugin opener hanya membuka file lokal dari Rust.
- Kode error baru (`attachment_type`, `attachment_too_large`, `attachment_missing`) wajib masuk `errors` di `ui/src/i18n/id.ts` dan `en.ts`. Teks `merge_into_self` diubah menjadi generik (id: "Tidak bisa digabung dengan dirinya sendiri", en: "Cannot be merged into itself").
- `ui/src/ipc/bindings.ts` di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- UI mengikuti `docs/design/ui-guidelines.md`. Teks bahasa Indonesia di `id.ts`, padanannya di `en.ts`. Di layar, payee disebut "Payee" (kunci `txn.payee` yang sudah ada).
- Verifikasi akhir tiap task Rust: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings`.

## Review Focus

1. **Dua transaksi dengan struk berisi sama** (dedup K1): menghapus lampiran di satu transaksi tidak boleh menghapus file milik transaksi lain. File baru terhapus saat rujukan terakhir hilang. Test: Task 3 `shared_content_survives_first_removal`.
2. **Ganti huruf besar/kecil nama sendiri** ("kantor" → "Kantor") harus berhasil, bukan `Duplicate`, untuk tag maupun payee. Test: Task 2 `rename_tag_case_only_change`, `update_payee_case_only_change`.
3. **Gabung/hapus payee yang dipakai transaksi terhapus**: setelah `undo_delete_txn`, transaksi itu menunjuk ke payee target (gabung) atau tanpa payee (hapus), tanpa FK menggantung. Test: Task 2 `merge_payee_updates_deleted_txns`.
4. **File lampiran hilang di disk** (restore backup lama, folder dipindah): daftar menandai `missing`, buka memberi `attachment_missing`, dan ekspor CSV tetap selesai dengan kolom `file` kosong. Test: Task 3 `missing_file_is_flagged_and_open_fails`; Task 4 `csv_export_tolerates_missing_attachment`.
5. **Restore dari backup manual di instalasi baru** membawa lampiran kembali. File di folder `-lampiran` yang isinya tidak cocok dengan namanya dilewati, tidak disalin. Test: Task 4 `manual_backup_roundtrip_restores_attachments`, `restore_skips_corrupted_attachment`.

---

### Task 1: Storage — migrasi 0007, repository tag/payee/lampiran

**Files:**
- Create: `crates/storage/migrations/0007_tags_attachments.sql`, `crates/storage/src/attachment.rs`
- Modify: `crates/storage/src/lib.rs` (`migrations()`, `pub mod attachment`), `crates/storage/src/tag.rs`, `crates/storage/src/payee.rs`, `crates/storage/src/txn_list.rs`, `crates/storage/src/export.rs`, `crates/storage/src/file.rs` (pin versi)

**Interfaces:**
- Produces (`tag.rs`):
  - `pub struct TagUsage { pub id: Uuid, pub name: String, pub txn_count: u32 }`
  - `list_tag_usage(conn: &Connection) -> Result<Vec<TagUsage>>`
  - `insert_tag(tx, name) -> Result<Uuid>`
  - `rename_tag(tx, id, name) -> Result<bool>` (`false` bila id tidak ada)
  - `merge_tag(tx, source, target) -> Result<()>`
  - `delete_tag(tx, id) -> Result<bool>`
  - UNIQUE → `StorageError::Duplicate` lewat `map_unique`
- Produces (`payee.rs`):
  - `pub struct PayeeUsage { pub id: Uuid, pub name: String, pub default_category_id: Option<Uuid>, pub txn_count: u32, pub last_date: Option<Date> }`
  - `list_payee_usage(conn) -> Result<Vec<PayeeUsage>>`
  - `insert_payee_with(tx, name, default_category_id: Option<Uuid>) -> Result<Uuid>`
  - `update_payee(tx, id, name, default_category_id) -> Result<bool>`
  - `merge_payee(tx, source, target) -> Result<()>`
  - `delete_payee(tx, id) -> Result<bool>`
  - `get_payee_name(conn, id) -> Result<Option<String>>`
- Produces (`attachment.rs`):
  - `pub struct AttachmentRecord { pub id: Uuid, pub txn_id: Uuid, pub file_name: String, pub sha256: String, pub mime: String, pub size_bytes: u64, pub created_at: String }`
  - `insert_attachment(tx, &AttachmentRecord) -> Result<()>`
  - `list_for_txn(conn, txn_id) -> Result<Vec<AttachmentRecord>>` (urut `created_at, id`)
  - `get_attachment(conn, id) -> Result<Option<AttachmentRecord>>`
  - `delete_attachment(tx, id) -> Result<bool>`
  - `sha_ref_count(conn, sha256) -> Result<u32>`
  - `all_shas(conn) -> Result<Vec<String>>` (distinct)
  - `for_active_txns(conn) -> Result<Vec<AttachmentRecord>>` (transaksi `deleted_at IS NULL`, urut `txn_id, created_at`)
- Produces: `TxnListRow.attachment_count: u32` (K15); `ExportTxn.attachments: Vec<AttachmentRecord>`.

- [ ] **Step 1: Write the failing tests**
  - `tag.rs`:
    - `tag_usage_counts_active_txns_only`: tag "kantor" dipakai 2 txn, salah satunya di-soft-delete (`deleted_at` diisi); tag "kosong" tak terpakai. Hasilnya `[("kantor", 1), ("kosong", 0)]`.
    - `rename_merge_delete_tag`:
      - `rename_tag(a, "Kantor")` → `true`;
      - `insert_tag("KANTOR")` → `Err(StorageError::Duplicate)`;
      - `merge_tag(a, b)` dengan txn yang punya kedua tag → txn itu hanya punya `b`, dan tag `a` tidak ada lagi;
      - `delete_tag(b)` → `txn_tags(txn)` kosong.
  - `payee.rs`:
    - `payee_usage_and_last_date`: payee dengan txn 10 Okt dan 12 Okt (12 Okt terhapus) → `txn_count == 1`, `last_date == Some(10 Okt)`.
    - `merge_and_delete_payee_touch_deleted_txns`:
      - txn aktif dan txn terhapus memakai source; setelah `merge_payee`, keduanya `payee_id = target` dan source hilang;
      - `delete_payee(target)` → kedua txn `payee_id IS NULL`.
  - `attachment.rs` `attachment_crud_and_ref_count`:
    - dua record dengan sha sama di txn berbeda → `sha_ref_count == 2`;
    - setelah `delete_attachment` salah satunya → 1;
    - `list_for_txn` hanya milik txn itu;
    - `for_active_txns` melewati txn terhapus.
  - `txn_list.rs` `attachment_count_on_rows`: txn dengan 2 lampiran → `attachment_count == 2`, txn lain 0.
  - `lib.rs` `migration_0007_schema`: indeks `tag_name_nocase` ada; `INSERT` tag "a" lalu "A" gagal UNIQUE; kolom `attachment.size_bytes` dan `created_at` ada.
- [ ] **Step 2: Run** `cargo test -p nabungo-storage`. Expected: FAIL kompilasi.
- [ ] **Step 3: Implement.**
  - `0007_tags_attachments.sql`:
    ```sql
    CREATE UNIQUE INDEX tag_name_nocase ON tag (name COLLATE NOCASE);
    ALTER TABLE attachment ADD COLUMN size_bytes INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE attachment ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
    CREATE INDEX idx_attachment_txn ON attachment (txn_id);
    CREATE INDEX idx_attachment_sha ON attachment (sha256);
    ```
  - `merge_tag`: `INSERT OR IGNORE INTO txn_tag SELECT txn_id, ?target FROM txn_tag WHERE tag_id = ?source`, lalu `DELETE` baris source di `txn_tag` dan `tag`.
  - `attachment_count` diambil lewat subquery `COUNT(*)` di query daftar.
- [ ] **Step 4: Run** `cargo test -p nabungo-storage`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(storage): migrasi 0007, CRUD tag/payee, repository lampiran`

### Task 2: App — use case tag dan payee (CAT-04)

**Files:**
- Create: `crates/app/src/tags.rs`
- Modify: `crates/app/src/payee.rs`, `crates/app/src/lib.rs` (`mod tags;`, re-export `TagUsage`, `PayeeUsage`)

**Interfaces:**
- Consumes: Task 1 fungsi `tag::*`, `payee::*`. Yang sudah ada: `normalize_tag` (`transactions.rs`), `normalize_name`, `AppError::duplicate_as`, `category::get_category_ref`.
- Produces (`impl Ledger`):
  - `list_tag_usage(&self) -> Result<Vec<TagUsage>, AppError>`
  - `create_tag(&mut self, name: &str) -> Result<Uuid, AppError>`
  - `rename_tag(&mut self, id: Uuid, name: &str) -> Result<(), AppError>`
  - `merge_tag(&mut self, source: Uuid, target: Uuid) -> Result<(), AppError>`
  - `delete_tag(&mut self, id: Uuid) -> Result<(), AppError>`
  - `list_payee_usage(&self) -> Result<Vec<PayeeUsage>, AppError>`
  - `create_payee(&mut self, name: &str, default_category_id: Option<Uuid>) -> Result<Uuid, AppError>`
  - `update_payee(&mut self, id: Uuid, name: &str, default_category_id: Option<Uuid>) -> Result<(), AppError>`
  - `merge_payee(&mut self, source: Uuid, target: Uuid) -> Result<(), AppError>`
  - `delete_payee(&mut self, id: Uuid) -> Result<(), AppError>`
  - ID tidak dikenal → `NotFound("tag")`/`NotFound("payee")`.

- [ ] **Step 1: Write the failing tests** (pola fixture `transactions.rs`)
  - `tags.rs`:
    - `create_tag_normalizes_and_rejects_duplicate`: `create_tag(" #Liburan ")` → nama "Liburan"; `create_tag("liburan")` → `Duplicate("tag")`; `create_tag("  ")` → `Catalog(NameEmpty)`.
    - `rename_tag_case_only_change`: "kantor" → "Kantor" `Ok`, dan `list_tag_usage` menampilkan "Kantor".
    - `rename_tag_to_other_name_is_duplicate`.
    - `merge_tag_into_self_rejected`: → `Catalog(MergeIntoSelf)`.
    - `merge_tag_moves_txns`: txn bertag "a" → setelah `merge_tag(a, b)`, `get_txn(..).tags == ["b"]`.
    - `delete_tag_removes_from_txns`.
    - `unknown_tag_not_found`.
  - `payee.rs`:
    - `update_payee_case_only_change`.
    - `payee_default_category_validated`: kategori diarsip → `Archived("category")`; id acak → `NotFound("category")`.
    - `default_category_drives_suggestion`: setelah `update_payee(.., Some(cat))`, `payee_suggestion(name).category_id == Some(cat)`.
    - `merge_payee_updates_deleted_txns`: txn memakai payee "Indomart" lalu dihapus; `merge_payee(indomart, indomaret)`; `undo_delete_txn` → `get_txn(..).payee == Some("Indomaret")`.
    - `delete_payee_clears_txns`: `get_txn(..).payee == None`, dan transaksi baru dengan nama yang sama membuat payee baru.
- [ ] **Step 2: Run** `cargo test -p nabungo-app tags:: payee::`. Expected: FAIL.
- [ ] **Step 3: Implement** per K11/K12, satu transaction per use case. Error UNIQUE dipetakan dengan `duplicate_as("tag")`/`duplicate_as("payee")`.
- [ ] **Step 4: Run** `cargo test -p nabungo-app`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(app): kelola tag dan payee (CAT-04)`

### Task 3: App — lampiran (TRX-08)

**Files:**
- Create: `crates/app/src/attachments.rs`
- Modify: `crates/app/Cargo.toml` (`sha2 = "0.10"`), `crates/app/src/lib.rs` (`mod attachments; pub use attachments::{AttachmentRow, MAX_ATTACHMENT_BYTES};`), `crates/app/src/ledger.rs` (varian `AttachmentType`, `AttachmentTooLarge`, `AttachmentMissing`)

**Interfaces:**
- Consumes: Task 1 `attachment::*`. Yang sudah ada: `txn::get_txn` (`Some(t) if !t.deleted`, pola `update_txn` di `transactions.rs`) untuk cek transaksi aktif.
- Produces:
  ```rust
  pub const MAX_ATTACHMENT_BYTES: u64 = 10 * 1024 * 1024;
  pub struct AttachmentRow { pub id: Uuid, pub file_name: String, pub mime: String, pub size_bytes: u64, pub missing: bool }
  pub(crate) fn sniff_mime(head: &[u8]) -> Option<&'static str>          // K2
  pub(crate) fn ext_for(mime: &str) -> &'static str                      // jpg/png/webp/pdf, dipakai Task 4
  pub(crate) fn sha256_file(path: &Path) -> std::io::Result<String>      // hex huruf kecil, dipakai Task 4
  impl Ledger {
      pub fn add_attachments(&mut self, txn_id: Uuid, sources: &[PathBuf], dir: &Path) -> Result<Vec<AttachmentRow>, AppError>
      pub fn list_attachments(&self, txn_id: Uuid, dir: &Path) -> Result<Vec<AttachmentRow>, AppError>
      pub fn remove_attachment(&mut self, id: Uuid, dir: &Path) -> Result<(), AppError>
      pub fn attachment_path(&self, id: Uuid, dir: &Path) -> Result<PathBuf, AppError>
  }
  ```
  `add_attachments` mengembalikan baris yang baru ditambah. Seluruh `sources` divalidasi (ukuran dan jenis) **sebelum** ada file yang disalin; satu file tidak valid membatalkan semuanya.

- [ ] **Step 1: Write the failing tests** (file fixture ditulis ke `tempdir` sebagai byte: PNG minimal = 8 byte signature + beberapa byte, PDF = `b"%PDF-1.4\n..."`, teks = `b"hello"`)
  - `sniff_detects_supported_types`: JPEG/PNG/WebP/PDF → mime yang benar; `b"hello"`, `b""`, `b"RIFF\0\0\0\0WAVE"` → `None`.
  - `add_copies_content_addressed`: menambah `struk.png` → satu baris dengan `file_name == "struk.png"`, `mime == "image/png"`, `size_bytes` = panjang file, `missing == false`; file `<dir>/<sha256>` ada dan isinya sama.
  - `add_rejects_type_size_and_deleted_txn`:
    - teks → `AttachmentType`;
    - file `MAX_ATTACHMENT_BYTES + 1` byte berawalan `%PDF-` → `AttachmentTooLarge`;
    - txn yang di-soft-delete → `NotFound("transaction")`;
    - campuran valid + tidak valid → error dan tidak ada baris maupun file baru.
  - `long_file_name_truncated_keeping_extension`: nama 200 karakter + `.pdf` → panjang ≤ 120 karakter dan berakhir `.pdf`.
  - `shared_content_survives_first_removal`: file yang sama dilampirkan ke txn A dan B → satu file fisik. Hapus lampiran A → file masih ada. Hapus lampiran B → file hilang.
  - `missing_file_is_flagged_and_open_fails`: hapus file fisik secara manual → `list_attachments` memberi `missing == true`; `attachment_path` → `AttachmentMissing`.
  - `soft_delete_keeps_attachments`: hapus txn lalu undo → lampiran masih terdaftar dan file masih ada.
- [ ] **Step 2: Run** `cargo test -p nabungo-app attachments::`. Expected: FAIL.
- [ ] **Step 3: Implement** per K1–K6. Saat validasi, ukuran dibaca dari `metadata().len()` dan head 16 byte untuk sniff. Konten di-hash saat disalin lewat stream (`std::io::copy` ke hasher + file `.partial`), tanpa memuat seluruh file ke memori. Rollback file baru sesuai K4.
- [ ] **Step 4: Run** `cargo test -p nabungo-app`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(app): lampiran struk berbasis sha256 (TRX-08)`

### Task 4: App — lampiran ikut backup manual, restore, ekspor

**Files:**
- Modify: `crates/app/src/backup.rs` (`backup_to`, `restore_file`), `crates/app/src/export.rs` (`export_json*`, `export_csv`), pemanggil di `src-tauri/src/commands.rs` sementara diberi argumen folder (Task 5 merapikan)

**Interfaces:**
- Consumes: Task 1 `all_shas`, `for_active_txns`, `ExportTxn.attachments`; Task 3 `ext_for`, `sha256_file`.
- Produces (signature baru):
  - `backup_to(&self, dest: &Path, attachments_dir: &Path) -> Result<(), AppError>`
  - `restore_file(&mut self, src: &Path, backup_dir: &Path, attachments_dir: &Path, now: OffsetDateTime) -> Result<(), AppError>`
  - `export_csv(&self, parent: &Path, today: Date, attachments_dir: &Path) -> Result<PathBuf, AppError>`
  - helper `attachments_sidecar(db: &Path) -> PathBuf` = `<parent>/<stem>-lampiran`

- [ ] **Step 1: Write the failing tests**
  - `backup.rs`:
    - `manual_backup_roundtrip_restores_attachments`: ledger A dengan satu lampiran → `backup_to(dest, dirA)` membuat `<stem>-lampiran/<sha>`. Ledger B (fixture baru, `dirB` kosong) → `restore_file(dest, .., dirB, now)` → `list_attachments` pada txn itu `missing == false` dan isi file sama.
    - `backup_without_attachments_has_no_sidecar`.
    - `restore_skips_corrupted_attachment`: isi file di sidecar diubah → tidak disalin ke `dirB`, lampiran `missing == true`, restore tetap `Ok`.
  - `export.rs`:
    - `json_export_has_all_collections` diperluas: txn berlampiran memiliki `attachments[0].sha256` dan `file_name`; txn lain `attachments == []`.
    - `csv_export_includes_attachments`: `attachments.csv` header `txn_id,file_name,sha256,mime,size_bytes,file`, dan `lampiran/<sha>.png` ada.
    - `csv_export_tolerates_missing_attachment`: file fisik dihapus → ekspor `Ok`, kolom `file` kosong.
    - Daftar file folder ekspor di test yang ada diperbarui (`attachments.csv` selalu ditulis, header saja bila kosong; subfolder `lampiran/` hanya bila ada file).
- [ ] **Step 2: Run** `cargo test -p nabungo-app backup:: export::`. Expected: FAIL.
- [ ] **Step 3: Implement** per K7/K8. Penyalinan sidecar dan lampiran ekspor bersifat best-effort per file (`tracing::warn!`). Hanya kegagalan membuat folder yang menjadi error `Io`.
- [ ] **Step 4: Run** `cargo test --workspace`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(app): lampiran ikut backup manual, restore, dan ekspor`

### Task 5: Tauri — DTO, command, opener, bindings

**Files:**
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-opener = "2"`), `src-tauri/src/lib.rs` (`.plugin(tauri_plugin_opener::init())`, `collect_commands!`), `src-tauri/src/paths.rs` (`attachments_path()`), `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/error.rs` (kode baru), `ui/src/ipc/bindings.ts` (generate), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts` (errors)

**Interfaces:**
- Consumes: Task 2–4.
- Produces (TS, camelCase dari specta):
  - Tipe:
    - `TagUsageDto { id: string; name: string; txnCount: number }`
    - `PayeeUsageDto { id: string; name: string; defaultCategoryId: string | null; txnCount: number; lastDate: string | null }`
    - `AttachmentDto { id: string; fileName: string; mime: string; sizeBytes: number; missing: boolean }`
    - `TxnRowDto.attachmentCount: number`
  - Command tag:
    - `listTagUsage(): TagUsageDto[]`
    - `createTag(name): string`
    - `renameTag(id, name): null`
    - `mergeTag(sourceId, targetId): null`
    - `deleteTag(id): null`
  - Command payee:
    - `listPayeeUsage(): PayeeUsageDto[]`
    - `createPayee(name, defaultCategoryId: string | null): string`
    - `updatePayee(id, name, defaultCategoryId): null`
    - `mergePayee(sourceId, targetId): null`
    - `deletePayee(id): null`
  - Command lampiran:
    - `addAttachments(txnId): AttachmentDto[]`: `async`, dialog multi-pilih dengan filter `["jpg","jpeg","png","webp","pdf"]`; batal → `[]`
    - `listAttachments(txnId): AttachmentDto[]`
    - `removeAttachment(id): null`
    - `openAttachment(id): null`
  - `ErrorCode::{AttachmentType, AttachmentTooLarge, AttachmentMissing}` → `attachment_type`, `attachment_too_large`, `attachment_missing`.

- [ ] **Step 1: Write the failing tests**
  - `paths.rs` `attachments_dir_under_data`: `d.attachments_path() == d.path.join("attachments")`.
  - Test pemetaan error yang ada di `error.rs` (bila ada) diperluas untuk tiga varian baru; bila tidak ada, buat `attachment_errors_map_to_codes`.
- [ ] **Step 2: Run** `cargo test -p nabungo-desktop`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - Dialog multi-pilih ditambahkan sebagai varian baru `Dialog::OpenMany` di `pick_path`, atau fungsi saudara yang memakai `blocking_pick_files`; pola `spawn_blocking` dipertahankan.
  - `add_attachments` membuat folder lampiran (`create_dir_all`) sebelum memanggil ledger.
  - `backup_now`, `restore_backup_from_file`, dan `export_csv` meneruskan `state.data_dir.attachments_path()`.
  - Tambah teks `errors.attachment_*` di id/en, lalu ubah teks `merge_into_self` menjadi generik (Global Constraints).
- [ ] **Step 4: Run** `cargo test -p nabungo-desktop export_bindings && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck`. Expected: PASS; `bindings.ts` berubah sesuai Interfaces.
- [ ] **Step 5: Commit** `feat(desktop): command tag/payee/lampiran, opener untuk membuka lampiran`

### Task 6: IPC + browser-ledger parity + store

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser-ledger.ts`
- Create: `ui/src/stores/labels.ts`, `ui/src/stores/labels.test.ts`
- Test: `ui/src/ipc/browser-ledger.test.ts`

**Interfaces:**
- Consumes: Task 5 types.
- Produces:
  - Wrapper `ipc.*` untuk semua command Task 5 (pola `unwrap(api().x(...))`).
  - Store Pinia `useLabelsStore()` dengan:
    - state `tags: TagUsageDto[]`, `payees: PayeeUsageDto[]`
    - `reload()`
    - `createTag`, `renameTag`, `mergeTag`, `deleteTag`
    - `createPayee`, `updatePayee`, `mergePayee`, `deletePayee`
  - Setiap aksi tulis memanggil `reload()` dan `useTransactionsStore().reload()`.

- [ ] **Step 1: Write the failing tests**
  - `browser-ledger.test.ts`:
    - `tag tak terpakai tetap terdaftar dan jumlah hanya transaksi aktif`.
    - `ganti nama tag beda huruf saja berhasil; nama tag lain → duplicate`.
    - `gabung tag memindah transaksi; gabung ke diri sendiri → merge_into_self`.
    - `hapus payee mengosongkan payee transaksi termasuk yang terhapus` (undo menunjukkan payee `null`).
    - `kategori default payee diarsip → archived`.
    - `lampiran simulasi`: `addAttachments(txn)` → satu `AttachmentDto` `struk-contoh.jpg`; `listTxns` → `attachmentCount === 1`; `removeAttachment` → 0; txn tidak dikenal → `not_found`.
  - `labels.test.ts`: `mergeTag` memanggil ipc lalu memuat ulang `tags`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/ipc src/stores/labels.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** per K16. `normalizeTags` menambah ke `state.tags` (NOCASE, nama pertama menang). `listTags` membaca `state.tags`, sehingga perilakunya kini sama dengan Rust yang mencantumkan tag tak terpakai.
- [ ] **Step 4: Run** `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): ipc dan store tag/payee/lampiran, parity browser-ledger`

### Task 7: UI — layar Tag & Payee, chip filter payee

**Files:**
- Create: `ui/src/views/LabelsView.vue`, `ui/src/views/LabelsView.test.ts`, `ui/src/components/LabelFormDialog.vue`, `ui/src/components/MergeLabelDialog.vue`
- Modify: `ui/src/router.ts`, `ui/src/components/AppSidebar.vue`, `ui/src/components/txn/TxnFilters.vue` (+ test), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/ui-guidelines.md` (§7.4 item sidebar, §9.4 tambahkan "Tag & Payee"), `ui/src/router.test.ts`/`AppSidebar.test.ts` bila mematok daftar route/nav

**Interfaces:**
- Consumes: Task 6 store `useLabelsStore`, store `useCategoriesStore` (pilihan kategori default memakai `CategoryCombobox` yang sudah ada), `useTransactionsStore().setQuery`.
- Produces:
  - `LabelFormDialog` props `{ open, kind: 'tag' | 'payee', label?: TagUsageDto | PayeeUsageDto }`. Field kategori default hanya untuk payee.
  - `MergeLabelDialog` props `{ open, kind, source }`. Isinya `Select` target dari daftar lain dan teks penjelasan, mengikuti pola `MergeCategoryDialog`.

Teks (id / en):
- `nav.labels`: "Tag & Payee" / "Tags & Payees"
- `labels.tags`: "Tag" / "Tags"; `labels.payees`: "Payee" / "Payees"
- `labels.newTag`: "Tag baru" / "New tag"; `labels.newPayee`: "Payee baru" / "New payee"
- `labels.editTag`: "Ubah tag" / "Edit tag"; `labels.editPayee`: "Ubah payee" / "Edit payee"
- `labels.name`: "Nama" / "Name"
- `labels.defaultCategory`: "Kategori default" / "Default category"; `labels.noDefault`: "Tanpa kategori default" / "No default category"
- `labels.usage`: "{n} transaksi" / "{n} transactions"
- `labels.viewTxns`: "Lihat transaksi" / "View transactions"
- `labels.mergeTitle`: "Gabungkan {name}" / "Merge {name}"; `labels.mergeTarget`: "Gabungkan ke" / "Merge into"
- `labels.mergeBody`: "Semua transaksi {source} dipindah ke {target}. {source} akan dihapus." / "All {source} transactions move to {target}. {source} will be deleted."
- `labels.deleteTitle`: "Hapus {name}?" / "Delete {name}?"
- `labels.deleteBody`: "{name} dilepas dari {n} transaksi. Saldo tidak berubah." / "{name} will be removed from {n} transactions. Balances don't change."
- `labels.empty`: "Belum ada {what}" / "No {what} yet"
- `txn.filters.payee` (label chip): "Payee: {name}" / "Payee: {name}"

- [ ] **Step 1: Write the failing tests**
  - `LabelsView.test.ts`:
    - Tab Tag menampilkan `#kantor` dengan "2 transaksi".
    - Tab Payee menampilkan payee beserta nama kategori default.
    - "Lihat transaksi" pada tag memanggil `setQuery` dengan `{ tag: 'kantor' }` dan router pindah ke `/transactions`.
    - Hapus → dialog konfirmasi memuat jumlah transaksi → konfirmasi memanggil `store.deleteTag(id)`.
    - Gabungkan → pilih target → `store.mergeTag(source, target)`.
    - Submit form payee dengan kategori default memanggil `store.updatePayee(id, name, catId)`.
    - Error `duplicate` tampil di dialog.
  - `TxnFilters.test.ts`: query `payeeId` aktif menampilkan chip "Payee: Indomaret"; tombol hapus chip mengosongkan `payeeId`.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/views/LabelsView.test.ts src/components/txn/TxnFilters.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** per K13/K14. Label chip payee diambil dari `useLabelsStore().payees` (muat bila kosong). Tab disimpan di query route (`?tab=payee`) agar kembali dari Transaksi tetap di tab yang sama.
- [ ] **Step 4: Run** `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): layar Tag & Payee, chip filter payee`

### Task 8: UI — lampiran di form transaksi, ikon di daftar, catatan enkripsi

**Files:**
- Create: `ui/src/components/txn/AttachmentList.vue`, `ui/src/components/txn/AttachmentList.test.ts`
- Modify: `ui/src/components/txn/TxnForm.vue` (+ test), `ui/src/components/txn/TxnList.vue` (+ test), `ui/src/components/settings/SecuritySection.vue` (+ test), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/ui-guidelines.md` (§8.4 field lampiran, §8.5 ikon klip)

**Interfaces:**
- Consumes: Task 6 `ipc.addAttachments`, `listAttachments`, `removeAttachment`, `openAttachment`; `TxnRowDto.attachmentCount`.
- Produces: `AttachmentList` props `{ txnId: string | null }`:
  - `null` → hanya teks petunjuk `txn.attachments.saveFirst`.
  - Selain itu memuat daftar, menampilkan tombol "Lampirkan file", dan per baris: ikon (`ImageIcon`/`FileTextIcon`), nama, ukuran (KB/MB, 1 desimal, locale), status "File hilang" bila `missing`, tombol Buka (nonaktif bila `missing`) dan Hapus (`ConfirmDialog`).
  - Setelah tambah atau hapus: memanggil `useTransactionsStore().reload()`.

Teks (id / en):
- `txn.attachments.title`: "Lampiran" / "Attachments"
- `txn.attachments.add`: "Lampirkan file" / "Attach file"
- `txn.attachments.saveFirst`: "Simpan dulu untuk melampirkan struk" / "Save first to attach a receipt"
- `txn.attachments.open`: "Buka" / "Open"
- `txn.attachments.missing`: "File hilang" / "File missing"
- `txn.attachments.removeTitle`: "Hapus lampiran {name}?" / "Delete attachment {name}?"
- `txn.attachments.removeBody`: "File dihapus dari folder data." / "The file is deleted from the data folder."
- `txn.hasAttachment` (aria-label ikon): "Ada lampiran" / "Has attachment"
- `security.attachmentsNote`: "Lampiran struk disimpan sebagai file biasa di folder data dan tidak ikut terenkripsi." / "Receipt attachments are stored as plain files in the data folder and are not encrypted."
- `errors.attachment_type`: "Hanya gambar (JPG, PNG, WebP) atau PDF" / "Only images (JPG, PNG, WebP) or PDF"
- `errors.attachment_too_large`: "Ukuran file maksimal 10 MB" / "Files can be at most 10 MB"
- `errors.attachment_missing`: "File lampiran tidak ditemukan di folder data" / "Attachment file not found in the data folder" (ditambahkan di Task 5)

- [ ] **Step 1: Write the failing tests**
  - `AttachmentList.test.ts`:
    - `txnId: null` → teks `saveFirst`, tanpa tombol.
    - Daftar dari spy `listAttachments` menampilkan `struk.png` dan `1,2 KB`.
    - Lampiran `missing` menampilkan "File hilang" dengan tombol Buka nonaktif.
    - Klik "Lampirkan file" memanggil `addAttachments('t1')`, dan error `attachment_too_large` tampil di `role="alert"`.
    - Hapus → konfirmasi → `removeAttachment(id)`.
  - `TxnForm.test.ts`: mode edit menampilkan seksi lampiran (`data-testid="txn-attachments"`); mode buat menampilkan petunjuk `saveFirst`.
  - `TxnList.test.ts`: baris dengan `attachmentCount > 0` memiliki `[data-testid="txn-attachment"]`, baris lain tidak.
  - `SecuritySection.test.ts`: teks `security.attachmentsNote` tampil.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui exec vitest run src/components`. Expected: FAIL.
- [ ] **Step 3: Implement.** Seksi lampiran berada di bagian detail opsional `TxnForm` dan ikut terbuka otomatis bila transaksi punya lampiran. Urutan Tab field yang sudah ada tidak berubah (§8.4); seksi ditempatkan setelah catatan. Perbarui pedoman §8.4/§8.5.
- [ ] **Step 4: Run** `pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): lampiran struk di form transaksi, ikon di daftar`

### Task 9: E2E, verifikasi penuh, roadmap

**Files:**
- Create: `ui/e2e/labels-7c.spec.ts`
- Modify: `docs/plans/2026-10-12-v1-roadmap.md`

- [ ] **Step 1: Write the E2E tests**
  - `kelola tag dan payee`:
    - catat expense dengan tag "kantor" dan payee "Indomart";
    - di Tag & Payee, ubah nama tag menjadi "Kantor";
    - gabungkan payee "Indomart" ke payee baru "Indomaret" (buat dulu);
    - "Lihat transaksi" pada "Indomaret" → halaman Transaksi dengan chip "Payee: Indomaret" dan baris expense tersebut bertag `#Kantor`.
  - `lampiran simulasi`: buka transaksi untuk edit → "Lampirkan file" → `struk-contoh.jpg` tampil → tutup sheet → baris punya ikon `txn-attachment` → hapus lampiran → ikon hilang.
- [ ] **Step 2: Run** `pnpm --filter @nabungo/ui e2e`. Expected: PASS. Buktikan RED dengan menonaktifkan sementara aksi gabung di `MergeLabelDialog` dan tombol tambah di `AttachmentList`, lalu pulihkan.
- [ ] **Step 3: Verifikasi penuh:** `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui build`. Setelah `cargo test -p nabungo-desktop export_bindings`, `git status` tidak boleh menunjukkan diff `bindings.ts`. Expected: semua PASS.
- [ ] **Step 4: Roadmap.**
  - Baris Fase 7 di tabel ringkasan menyebut 7c → `2026-10-21-fase-7c-tag-payee-lampiran.md (selesai)`; baris 7c di seksi Fase 7 ditandai (selesai).
  - Tambah blok "Keputusan 7c":
    - K1/K5: file berbasis sha256 dengan dedup; hapus file saat rujukan terakhir hilang.
    - K7: sidecar `-lampiran/` untuk backup manual.
    - K9: lampiran tidak terenkripsi.
- [ ] **Step 5: Commit** `test(e2e): tag/payee dan lampiran; docs: Fase 7c selesai`
