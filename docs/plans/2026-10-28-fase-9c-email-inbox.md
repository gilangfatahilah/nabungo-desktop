# Fase 9c — Email Capture: Inbox Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EML-03 bisa dipakai dari aplikasi. Hasil sync email (9b) tampil di layar **Inbox**. Tab "Perlu ditinjau" berisi capture `pending` dengan draft transaksi. Dari sana user bisa:
- **Catat** sekali klik bila draft lengkap.
- **Tinjau**: edit di sheet form transaksi, lalu catat.
- **Abaikan** satu per satu atau massal sampai tanggal tertentu, dengan undo.
- **Hubungkan** ke transaksi manual yang mirip (duplicate hint).

Tab "Gagal dibaca" menampilkan log unparsed (hanya baca). Tidak ada transaksi tercatat tanpa aksi user.

**Architecture:**
- `nabungo-app` (`capture.rs`): use case daftar inbox. Isinya draft + kandidat duplikat per capture, ringkasan jumlah, abaikan massal, dan batal-abaikan. `budget_alerts.rs` mendapat `confirm_capture_with_alerts`.
- `src-tauri`: command tipis + DTO.
- `ui`:
  - route `/inbox` dan store `inbox`.
  - komponen di `components/inbox/`.
  - fallback in-memory di `browser-ledger.ts` + hook seed untuk e2e.

**Tech Stack:** Rust (rusqlite, tauri-specta), Vue 3 + shadcn-vue (Tabs, Sheet, Dialog, DropdownMenu, Badge), Pinia, vitest, Playwright.

**Spec:**
- `PRD.md`: EML-03 (§5), skema email capture (§6), Privasi (§8).
- Brief `docs/plans/Brief Auto-Capture Transaksi via IMAP.md`: §Review flow di UI, milestone 5–6.
- Plan 9a `docs/plans/2026-10-27-fase-9a-email-capture-backend.md`:
  - K9: draft `capture_draft`.
  - K10: `confirm_capture`.
  - K11: kandidat duplikat.
- Plan 9b `docs/plans/2026-10-28-fase-9b-email-capture-tauri.md`: K11 event `EmailSyncDone`, K14 mode browser.

**Di luar plan ini (9d):**
- Rule editor (EML-04) dan "buat rule dari email unparsed".
- Re-parse unparsed.
- Mengubah akun/kategori default rule.

Tab "Gagal dibaca" di 9c hanya menampilkan data.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **9c dipecah**: 9c = Inbox review (EML-03); 9d = rule editor (EML-04). Baris roadmap diperbarui. | Masing-masing sebesar satu sub-fase. |
| K2 | **Navigasi**: route `/inbox` (`RouteName` `'inbox'`), item sidebar grup *Kelola* setelah Terjadwal. Ikon lucide `Inbox`, badge = jumlah pending (pola Terjadwal), tanpa shortcut `G`. Item **hanya tampil** bila `emailEnabled \|\| pending > 0 \|\| unparsed > 0`. Juga ada di Command Palette dengan syarat yang sama. | User tanpa email capture tidak melihat menu kosong. |
| K3 | **Satu panggilan daftar**: `pending_captures()` mengembalikan per capture: draft (K9 9a), nama rule, snippet, dan kandidat duplikat beserta data tampilannya (`DuplicateTxn`). Urutan dari storage: tanggal terbaru dulu. Draft dihitung lewat helper privat `draft_of(&CaptureRecord)` agar `get_capture` tidak diulang. `capture_draft`/`duplicate_candidates` tetap ada dan memakai helper yang sama. | UI tidak perlu N panggilan IPC; logika duplikat tetap di Rust. |
| K4 | **`DuplicateTxn { id, date, payee, amount, account_id }`** diturunkan dari `get_txn`. Income/expense: `amount = \|Σ posting\|`, `account_id` = akun posting pertama. Transfer: `amount` = posting tanpa kategori terbesar, `account_id` = posting tanpa kategori yang negatif. | Sama dengan definisi nominal di `find_duplicate_txns`. |
| K5 | **Catat cepat**: tombol `Catat` hanya tampil bila draft *siap*. Siap berarti: akun ada dan aktif; income/expense punya kategori aktif sejenis; transfer punya akun tujuan aktif yang berbeda dari asal; tanpa kandidat duplikat. Selain itu aksi utama adalah `Tinjau`. Payload dibentuk `toDto(fromCapture(c))`. Rust tetap memvalidasi. | Klik sekali hanya untuk draft yang tidak butuh keputusan. |
| K6 | **Tinjau** = `CaptureSheet` (sheet kanan 440px, pola `RuleSheet`) berisi `TxnForm` dengan prop `submit` → `inbox.confirm(id, state)`. Di atas form ada: (a) kotak lipat "Email asli" berisi snippet (`whitespace-pre-wrap`, `text-meta`), (b) daftar kandidat duplikat dengan tombol `Hubungkan`. `fromCapture` mengosongkan akun/kategori yang diarsip atau salah jenis. Kategori kosong membuat form meminta kategori. `note` form kosong; nama rule tidak masuk ke catatan transaksi. | Pakai ulang form & validasi yang ada; data arsip tidak membuat simpan gagal. |
| K7 | **Abaikan** satu = `ignore_capture`. Toast "Diabaikan" dengan aksi `Urungkan` → `unignore_captures([id])`. **Abaikan massal**: menu `⋯` di PageHeader → dialog tanggal (default hari ini) + pratinjau jumlah dari daftar yang dimuat → `ignore_captures_until(date)`. Isinya pending dengan `date <= date`, satu SQLite transaction, mengembalikan id. Toast dengan `Urungkan` → `unignore_captures(ids)`. | Sync pertama sebuah label bisa membawa ratusan email lama yang sudah dicatat manual (348 email di 9a). Konvensi repo: aksi menghapus bisa di-undo. |
| K8 | **`unignore_captures(ids)`**: semua id harus berstatus `ignored`. Bila ada yang tidak → `AppError::CaptureNotIgnored` (code `capture_not_ignored`) dan tidak ada yang berubah. Status kembali `pending`, `txn_id` NULL. | Undo atomik; pesan jelas bila sudah diproses di tempat lain. |
| K9 | **Confirm** memakai `confirm_capture_with_alerts` (pola `create_txn_with_alerts`) → `SavedTxnDto`. Alert budget ditampilkan lewat `transactions.notifyAlerts`. | Konsisten dengan simpan transaksi biasa. |
| K10 | **Ringkasan** `inbox_summary` → `{ pending, unparsed, emailEnabled }` untuk badge sidebar. Dimuat saat sidebar mount, setelah setiap aksi tulis inbox, dan saat event `EmailSyncDone` (listener `ipc.onEmailSync` dipasang sekali oleh store, dipanggil dari `AppSidebar`). Bila daftar Inbox sudah dimuat, event `done` juga memuat ulang daftar. | Sync otomatis tiap 15 menit memperbarui badge tanpa membuka Settings. |
| K11 | **Reload tidak menutup sheet**: `CaptureSheet` memegang snapshot `CaptureDto` miliknya sendiri. Bila capture sudah tidak pending saat disimpan → `capture_not_pending` tampil di form, daftar dimuat ulang, sheet tetap terbuka sampai ditutup user. | Sync berjalan di latar saat user sedang mengedit. |
| K12 | **Tab "Gagal dibaca"**: baris berisi subjek (atau "(tanpa subjek)"), pengirim, tanggal terima lokal, dan snippet yang bisa dibuka-tutup. Tidak ada aksi (9d). | EML-02: unparsed tidak dibuang diam-diam; user bisa melihatnya. |
| K13 | **Header Inbox**: tombol `Sync sekarang` (outline) bila `emailEnabled`. Memakai `ipc.syncEmail`; selesai → toast `email.syncDone` + reload. Error lewat `withErrorToast`. Belum dikonfigurasi → EmptyState dengan link ke Settings. Kartu Email di Settings mengganti teks "segera hadir" dengan link `Tinjau di Inbox`. | Satu tempat untuk memproses hasil sync. |
| K14 | **Mode browser**: `browser-ledger.ts` menyimpan `captures`/`unparsed` in-memory. Isinya diisi lewat `__seedEmailInbox({ captures, unparsed })` yang dipasang di `window` (pola `__enterRecovery`). Aturannya port Rust: siap/duplikat (±1 hari, nominal sama, jenis sama, akun draft bila ada), status, abaikan massal, undo, error code sama. Confirm membuat transaksi lewat jalur `createTxn` browser. `syncEmail` browser tetap mengembalikan ringkasan nol. | Test UI & e2e tanpa Tauri (K14 9b). |

## Global Constraints

- Arah dependensi `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage, nabungo-mail)`. Command Tauri tipis; logika di `nabungo-app`.
- Satu use case tulis = satu SQLite transaction. Repository tulis menerima `&Transaction`; baca boleh `&Connection`.
- Command baru didaftarkan di `collect_commands!` (`src-tauri/src/lib.rs`). Bindings di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- Error code baru (`capture_not_ignored`) masuk `ErrorCode` (`src-tauri/src/error.rs`) + `errors` di `ui/src/i18n/id.ts` & `en.ts`.
- UI mengikuti `docs/design/design.md`. Semua panggilan lewat `ui/src/ipc/`. Angka uang integer, format lewat `lib/money.ts`/`MoneyText`. Store memuat ulang dari backend setelah setiap aksi tulis; UI tidak menghitung saldo.
- UI tidak menyusun postings; confirm mengirim `TxnFormDto`.
- Privasi: snippet/subjek email tidak dicatat di log Rust maupun `console`.
- Tidak ada network call baru. Inbox hanya membaca DB; sync tetap lewat jalur 9b.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`, `pnpm --filter @nabungo/ui e2e`.

## Review Focus

1. **Sync pertama membawa ratusan email lama**: daftar tetap bisa dipakai, dan email lama bisa diabaikan massal lalu di-undo. Test: `ignore_until_only_pending_on_or_before`, `unignore_restores_pending` (Task 1); e2e `bulk ignore + undo` (Task 6).
2. **Draft menunjuk akun/kategori yang sudah diarsip** (dari rule atau saran payee): `Catat` tidak muncul, dan form Tinjau mengosongkan field itu sehingga simpan tidak gagal karena arsip. Test: `archived_draft_fields_are_dropped`, `not_ready_when_account_archived` (Task 3).
3. **Klik `Catat` dua kali / capture sudah diproses di tempat lain**: tidak ada transaksi dobel. Error `capture_not_pending` tampil dan daftar dimuat ulang. Test: `confirm_twice_second_is_not_pending` (Task 1), `confirm_not_pending_reloads` (Task 3).
4. **Salah klik Abaikan**: `Urungkan` mengembalikan item ke daftar. Undo setelah item diproses di tempat lain gagal atomik. Test: `unignore_non_ignored_rolls_back` (Task 1), e2e (Task 6).
5. **Sync otomatis selesai saat sheet Tinjau terbuka**: isian user tidak hilang, sheet tidak tertutup. Test: `reload_keeps_open_sheet` (Task 5).

---

### Task 1: Use case inbox di `nabungo-app`

**Files:**
- Modify: `crates/storage/src/email.rs` (fungsi hitung + id pending sampai tanggal + test)
- Modify: `crates/app/src/capture.rs`, `crates/app/src/budget_alerts.rs`, `crates/app/src/ledger.rs` (varian `CaptureNotIgnored`), `crates/app/src/lib.rs` (re-export)

**Interfaces:**
- Consumes: 9a `CaptureRecord`, `CaptureDraft`, `capture_draft`, `confirm_capture`, `ignore_capture`, `find_duplicate_txns`, `get_txn`.
- Produces (storage):
  ```rust
  pub fn count_captures(conn: &Connection, status: CaptureStatus) -> Result<u32>;
  pub fn count_unparsed(conn: &Connection) -> Result<u32>;
  /// Pending dengan `date <= until`, urut date, id.
  pub fn pending_ids_until(conn: &Connection, until: Date) -> Result<Vec<Uuid>>;
  ```
- Produces (app):
  ```rust
  pub struct DuplicateTxn { pub id: Uuid, pub date: Date, pub payee: Option<String>, pub amount: i64, pub account_id: Uuid }
  pub struct CaptureItem { pub draft: CaptureDraft, pub snippet: Option<String>, pub duplicates: Vec<DuplicateTxn> }
  pub struct InboxSummary { pub pending: u32, pub unparsed: u32 }
  impl Ledger {
      pub fn pending_captures(&self) -> Result<Vec<CaptureItem>, AppError>;          // K3, K4
      pub fn inbox_summary(&self) -> Result<InboxSummary, AppError>;
      pub fn ignore_captures_until(&mut self, until: Date) -> Result<Vec<Uuid>, AppError>; // K7
      pub fn unignore_captures(&mut self, ids: &[Uuid]) -> Result<(), AppError>;     // K8
      pub fn confirm_capture_with_alerts(&mut self, id: Uuid, form: &TxnForm)
          -> Result<(Uuid, Vec<BudgetAlert>), AppError>;                             // K9
  }
  // AppError::CaptureNotIgnored
  ```
  `CaptureDraft.note` tetap berisi nama rule; DTO mengekspos nilainya sebagai `ruleName`.

- [ ] **Step 1: Tulis test storage** di `email.rs`: `count_and_pending_until`. Isi 3 capture (2 pending bertanggal 05-09 dan 05-11, 1 ignored bertanggal 05-01) + 1 unparsed. Harapan: `count_captures(Pending) == 2`, `count_unparsed == 1`, `pending_ids_until(05-10)` = hanya capture 05-09.
- [ ] **Step 2: Tulis test app** di `capture.rs` (pakai helper `kit`, `ingest`, `qris_capture`, `expense` yang sudah ada):
  - `pending_captures_include_draft_and_duplicates`: QRIS + expense manual 41.700 tanggal `2026-05-10` di akun Kit. `items.len() == 1`, `items[0].draft.amount == 41_700`. `duplicates == [DuplicateTxn { id: <txn>, date: 2026-05-10, payee: Some("IDM INDOMARET D"), amount: 41_700, account_id: <akun> }]`. `snippet.is_some()`.
  - `pending_captures_exclude_confirmed_and_ignored`.
  - `transfer_duplicate_uses_source_account`: `Kit::new(2, …)`, TARIK (TransferOut) dengan rule `account_id` diisi akun pertama (UPDATE langsung di test) + transfer manual bernominal sama dari akun pertama ke akun kedua, tanggal sama dengan fixture. `duplicates[0].amount` = nominal, `account_id` = akun asal.
  - `inbox_summary_counts`: QRIS + UNKNOWN → `{ pending: 1, unparsed: 1 }`.
  - `ignore_until_only_pending_on_or_before`: `ignore_captures_until(QRIS_DATE - 1 hari)` → `[]`; `ignore_captures_until(QRIS_DATE)` → `[qris]`, status ignored.
  - `unignore_restores_pending`: ignore → unignore → muncul lagi di `pending_captures`, `txn_id` None.
  - `unignore_non_ignored_rolls_back`: `[ignored_id, pending_id]` → `Err(CaptureNotIgnored)`, `ignored_id` tetap ignored.
  - `confirm_twice_second_is_not_pending` (bila 9a belum punya): confirm kedua → `CaptureNotPending`, jumlah txn expense tetap 1.
- [ ] **Step 3: Tulis test** `confirm_capture_reports_alert` di `budget_alerts.rs`: pola test `create_txn_with_alerts` yang ada (budget Makan & Minum kecil, confirm capture dengan form expense kategori itu → satu alert `Exceeded`).
- [ ] **Step 4:** `cargo test -p nabungo-storage email::` dan `cargo test -p nabungo-app capture:: budget_alerts::` → FAIL.
- [ ] **Step 5: Implementasi.** Refactor `capture_draft` ke helper privat `fn draft_of(&self, c: &CaptureRecord) -> Result<CaptureDraft, AppError>`. `duplicate_candidates` memakai `fn duplicate_ids(&self, d: &CaptureDraft)`. `DuplicateTxn` dibentuk dari `get_txn` sesuai K4.
- [ ] **Step 6:** Jalankan test Step 4 lagi → PASS; `cargo test --workspace` → PASS.
- [ ] **Step 7: Commit** `feat(app): use case inbox email capture`.

### Task 2: Command & DTO Tauri

**Files:**
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` (`collect_commands!`), `src-tauri/src/error.rs` (`CaptureNotIgnored`)
- Modify (generate): `ui/src/ipc/bindings.ts`
- Modify: `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts` (`errors.capture_not_ignored` saja; teks lain di Task 4)

**Interfaces:**
- Consumes: Task 1.
- Produces (DTO, `camelCase`):
  ```rust
  pub struct DuplicateTxnDto { pub id: String, pub date: String, pub payee: Option<String>, pub amount: i64, pub account_id: String }
  pub struct CaptureDto {
      pub id: String, pub kind: TxnFormKindDto, pub date: String, pub amount: i64, pub payee: Option<String>,
      pub account_id: Option<String>, pub to_account_id: Option<String>, pub category_id: Option<String>,
      pub rule_name: Option<String>, pub snippet: Option<String>, pub duplicates: Vec<DuplicateTxnDto>,
  }
  pub struct UnparsedEmailDto { pub message_id: String, pub sender: String, pub subject: String, pub received_at: Option<String>, pub snippet: String }
  pub struct InboxSummaryDto { pub pending: u32, pub unparsed: u32, pub email_enabled: bool }
  ```
  `CaptureDto.kind` dari `TxnKind` draft (selalu Income/Expense/Transfer); `to_account_id` = `counter_account_id`.
- Produces (command, semuanya lewat `ledger(&state)?`):
  ```rust
  inbox_summary() -> InboxSummaryDto                          // email_enabled dari email_settings()
  list_pending_captures() -> Vec<CaptureDto>
  list_unparsed_emails() -> Vec<UnparsedEmailDto>
  confirm_capture(id: String, form: TxnFormDto) -> SavedTxnDto
  link_capture(id: String, txn_id: String) -> ()
  ignore_capture(id: String) -> ()
  ignore_captures_until(date: String) -> Vec<String>          // YYYY-MM-DD → parse_day
  unignore_captures(ids: Vec<String>) -> ()
  ```

- [ ] **Step 1: Tulis test** di `dto.rs`/`error.rs`:
  - `capture_dto_serializes_camel_case`: field `accountId`, `toAccountId`, `ruleName`, `duplicates[0].accountId`.
  - `capture_dto_maps_transfer_counter_account`.
  - `capture_not_ignored_maps_to_code`: serialisasi `"capture_not_ignored"`.
- [ ] **Step 2:** `cargo test -p nabungo-desktop` → FAIL; implementasi; → PASS.
- [ ] **Step 3:** `cargo test -p nabungo-desktop export_bindings` lalu `pnpm --filter @nabungo/ui typecheck`. Expected: bindings berubah (8 command + 4 DTO + code baru), typecheck PASS setelah `errors.capture_not_ignored` ditambahkan di `id.ts`/`en.ts`.
- [ ] **Step 4: Commit** `feat(desktop): command inbox email capture` (termasuk `bindings.ts`).

### Task 3: Facade IPC, mode browser, store `inbox`, helper draft

**Files:**
- Modify: `ui/src/ipc/index.ts`, `ui/src/ipc/browser.ts`, `ui/src/ipc/browser-ledger.ts`
- Create: `ui/src/lib/capture.ts`, `ui/src/lib/capture.test.ts`, `ui/src/stores/inbox.ts`, `ui/src/stores/inbox.test.ts`
- Test: `ui/src/ipc/browser-ledger.test.ts` (tambah describe `email inbox`)

**Interfaces:**
- Consumes: Task 2 bindings.
- Produces (`ipc`):
  ```ts
  inboxSummary(): Promise<InboxSummaryDto>
  listPendingCaptures(): Promise<CaptureDto[]>
  listUnparsedEmails(): Promise<UnparsedEmailDto[]>
  confirmCapture(id: string, form: TxnFormDto): Promise<SavedTxnDto>
  linkCapture(id: string, txnId: string): Promise<void>
  ignoreCapture(id: string): Promise<void>
  ignoreCapturesUntil(date: string): Promise<string[]>
  unignoreCaptures(ids: string[]): Promise<void>
  ```
  Re-export tipe `CaptureDto`, `DuplicateTxnDto`, `UnparsedEmailDto`, `InboxSummaryDto`.
- Produces (browser, K14):
  ```ts
  export type BrowserCaptureSeed = Omit<CaptureDto, 'id' | 'duplicates'> & { id?: string }
  export type BrowserUnparsedSeed = UnparsedEmailDto
  export function __seedEmailInbox(seed: { captures?: BrowserCaptureSeed[]; unparsed?: BrowserUnparsedSeed[] }): void
  ```
  `duplicates` dihitung saat `listPendingCaptures` dari transaksi browser aktif. Aturannya sama dengan `find_duplicate_txns`: jenis sama, nominal sama, ±1 hari, akun draft bila ada, maks 5, urut selisih hari. `inboxSummary.emailEnabled` diambil dari `browserEmail.emailStatus()` (dirakit di `browser.ts`). `__seedEmailInbox` dipasang di `window` bersama `__enterRecovery`.
- Produces (`lib/capture.ts`):
  ```ts
  /** Draft → state form; akun/kategori arsip atau salah jenis dikosongkan (K6). */
  export function fromCapture(c: CaptureDto, ctx: { accounts: AccountDto[]; categories: CategoryDto[]; feeCategoryId: string | null }): TxnFormState
  /** K5. */
  export function isReady(c: CaptureDto, ctx: { accounts: AccountDto[]; categories: CategoryDto[] }): boolean
  ```
- Produces (`stores/inbox.ts`, setup store `useInboxStore`):
  ```ts
  summary: Ref<InboxSummaryDto>; pending: Ref<CaptureDto[]>; unparsed: Ref<UnparsedEmailDto[]>; loaded: Ref<boolean>
  showNav: ComputedRef<boolean>                          // K2
  reloadSummary(): Promise<void>
  reload(): Promise<void>                                // summary + pending + unparsed; loaded = true
  confirm(id: string, state: TxnFormState): Promise<string>   // K9; selalu reload (juga saat error) lalu rethrow
  quickConfirm(c: CaptureDto): Promise<string>           // fromCapture → confirm
  link(id: string, txnId: string): Promise<void>
  ignore(id: string): Promise<void>
  ignoreUntil(date: string): Promise<string[]>
  unignore(ids: string[]): Promise<void>
  listen(): Promise<void>                                // K10; idempoten, subscribe ipc.onEmailSync sekali
  ```
  Setelah confirm/link, store juga memuat ulang data yang terdampak transaksi baru, sama dengan yang dimuat ulang `scheduled.ts` setelah `record`.

- [ ] **Step 1: Tulis test** `lib/capture.test.ts`:
  - `expense_draft_maps_to_form`: kind, amount, accountId, categoryId, payee, `note === ''`.
  - `transfer_draft_maps_to_and_fee_category`.
  - `archived_draft_fields_are_dropped`: akun arsip → `accountId null`; kategori income pada draft expense → `categoryId null`.
  - `ready_rules`: lengkap → true. Tanpa akun / tanpa kategori / duplikat ≥ 1 / transfer tanpa tujuan / tujuan = asal → false.
  - `not_ready_when_account_archived`.
- [ ] **Step 2: Tulis test** `browser-ledger.test.ts` describe `email inbox`:
  - seed 1 capture expense → `listPendingCaptures` berisi 1.
  - `confirmCapture` → transaksi muncul di `listTxns`, capture hilang dari pending, confirm kedua → `capture_not_pending`.
  - transaksi manual bernominal sama sehari sesudahnya → `duplicates.length === 1`.
  - `ignoreCapturesUntil` + `unignoreCaptures` (termasuk id pending → `capture_not_ignored`, tidak ada yang berubah).
  - `inboxSummary` menghitung pending/unparsed.
- [ ] **Step 3: Tulis test** `stores/inbox.test.ts` (memakai mode browser):
  - `reload_fills_lists_and_summary`.
  - `showNav_rules`: semua nol & email mati → false; pending > 0 → true; `emailEnabled` → true.
  - `confirm_not_pending_reloads`: hapus capture lewat `ignoreCapture` langsung di ipc, lalu `store.confirm` → reject `capture_not_pending` dan `store.pending` sudah tanpa item itu.
  - `listen_reloads_on_sync_done`: `ipc.syncEmail()` (browser memancarkan `done`) → `reloadSummary` terpanggil.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui exec vitest run src/lib/capture.test.ts src/ipc/browser-ledger.test.ts src/stores/inbox.test.ts` → FAIL; implementasi; → PASS.
- [ ] **Step 5:** `pnpm --filter @nabungo/ui typecheck` → PASS.
- [ ] **Step 6: Commit** `feat(ui): ipc & store inbox email capture`.

### Task 4: Layar Inbox, navigasi, daftar pending & unparsed

**Files:**
- Create: `ui/src/views/InboxView.vue`, `ui/src/views/InboxView.test.ts`, `ui/src/components/inbox/CaptureRow.vue`, `ui/src/components/inbox/UnparsedList.vue`, `ui/src/components/inbox/IgnoreUntilDialog.vue`
- Modify: `ui/src/router.ts` (+ `router.test.ts` bila mendaftar semua route), `ui/src/components/layout/AppSidebar.vue` (+ test), `ui/src/components/CommandPalette.vue`, `ui/src/components/settings/EmailSection.vue` (+ test), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `docs/design/design.md`

**Interfaces:**
- Consumes: Task 3 (`useInboxStore`, `isReady`).
- Produces: `CaptureRow` props `{ capture: CaptureDto; ready: boolean; busy: boolean }`, emits `confirm`, `review`, `ignore`, `duplicates`. `InboxView` menampung `CaptureSheet`/`DuplicateDialog` dari Task 5 (Task 4 memasang emit `review`/`duplicates` ke ref lokal `reviewing: CaptureDto | null`, `dupOf: CaptureDto | null`).

Tata letak (tambahkan sebagai `design.md` §8.10 Inbox, dan item sidebar di §6.2):

```
 Inbox                                   [Sync sekarang] [⋯]   ← ⋯: Abaikan sampai tanggal…
 [ Perlu ditinjau  12 | Gagal dibaca  2 ]                      ← Tabs
 9 Mei 2026
 [✉] IDM INDOMARET D                  −Rp 41.700  [Catat] [Tinjau] ⋯
     BCA Pembayaran (QRIS/VA) · BCA · Makan & Minum
 [✉] MUHAMMAD NUR  (Mungkin duplikat)   −Rp 10.000          [Tinjau] ⋯
     BCA Transfer · Pilih kategori                              ← field kosong: teks warning
```

- Baris dikelompokkan per tanggal (header tanggal 32px, pola §7.5). Baris 1: payee, atau nama rule bila payee kosong. Baris 2 `text-meta`: nama rule · akun · kategori/tujuan. Field kosong ditulis "Pilih akun"/"Pilih kategori" dengan warna warning.
- Nominal bertanda dan berwarna semantik (`MoneyText`); transfer netral.
- Badge `Mungkin duplikat` (outline, warning) adalah tombol yang memancarkan `duplicates`.
- `Catat` (default, kecil) hanya bila `ready`. `Tinjau` (outline, kecil) selalu ada. `⋯` berisi `Abaikan`, plus `Hubungkan ke transaksi…` bila ada duplikat.
- Copy kunci i18n `inbox.*`:
  - Judul & tab: `title` "Inbox", `tabPending` "Perlu ditinjau", `tabUnparsed` "Gagal dibaca".
  - Aksi baris: `record` "Catat", `review` "Tinjau", `ignore` "Abaikan", `linkMenu` "Hubungkan ke transaksi…", `duplicate` "Mungkin duplikat".
  - Field kosong: `pickAccount` "Pilih akun", `pickCategory` "Pilih kategori".
  - Toast: `recorded` "Transaksi dicatat", `ignored` "Diabaikan", `ignoredMany` "{n} email diabaikan", `undo` "Urungkan".
  - Abaikan massal: `ignoreUntil` "Abaikan sampai tanggal…", `ignoreUntilHint` "{n} item bertanggal sampai {date} akan diabaikan".
  - Empty state: `emptyTitle` "Semua sudah ditinjau", `notConfiguredTitle` "Email capture belum aktif", `notConfiguredAction` "Buka Pengaturan", `unparsedEmpty` "Tidak ada email yang gagal dibaca".
  - Lain-lain: `noSubject` "(tanpa subjek)", `nav.inbox` "Inbox".
  - Ganti `email.reviewSoon` menjadi `email.reviewInInbox` "Tinjau di Inbox" (link).

- [ ] **Step 1: Tulis test** `InboxView.test.ts` (browser mode + `__seedEmailInbox`):
  - `lists_pending_grouped_by_date`.
  - `quick_confirm_only_when_ready`: draft lengkap punya tombol `Catat`; draft tanpa kategori tidak, dan menampilkan "Pilih kategori".
  - `quick_confirm_records_and_removes_row`: klik `Catat` → baris hilang, toast `inbox.recorded`.
  - `ignore_with_undo`: `Abaikan` → baris hilang; aksi toast `Urungkan` → baris kembali.
  - `ignore_until_dialog_previews_count_and_ignores`.
  - `unparsed_tab_shows_subject_and_toggles_snippet`.
  - `not_configured_empty_state_links_settings`.
  - Tes sidebar (`AppSidebar.test.ts`): item Inbox tersembunyi saat `showNav` false, tampil dengan badge `nav-inbox-badge` = jumlah pending.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui exec vitest run src/views/InboxView.test.ts src/components/layout/AppSidebar.test.ts src/components/settings/EmailSection.test.ts` → FAIL; implementasi; → PASS.
- [ ] **Step 3:** `pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test` → PASS.
- [ ] **Step 4: Commit** `feat(ui): layar Inbox email capture`.

### Task 5: Sheet Tinjau & dialog duplikat

**Files:**
- Create: `ui/src/components/inbox/CaptureSheet.vue`, `ui/src/components/inbox/CaptureSheet.test.ts`, `ui/src/components/inbox/DuplicateDialog.vue`
- Modify: `ui/src/views/InboxView.vue` (+ test), `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`

**Interfaces:**
- Consumes: Task 3 (`fromCapture`, `useInboxStore.confirm/link`), Task 4 (`reviewing`, `dupOf`).
- Produces:
  - `CaptureSheet`: props `{ capture: CaptureDto | null }`, emits `close`. Saat `capture` berubah dari null → snapshot lokal + `session++` (remount `TxnForm`, pola `NewTxnSheet`). Perubahan isi `capture` saat sheet terbuka diabaikan (K11).
  - `DuplicateDialog`: props `{ capture: CaptureDto | null }`, emits `close`. Tiap kandidat menampilkan tanggal, payee, akun, nominal, dan tombol `Hubungkan` → `inbox.link` → toast `inbox.linked` → close.
- Copy: `inbox.reviewTitle` "Tinjau transaksi", `inbox.originalEmail` "Email asli", `inbox.duplicatesHint` "Mirip dengan transaksi yang sudah ada", `inbox.link` "Hubungkan", `inbox.linked` "Dihubungkan ke transaksi yang ada", `inbox.duplicatesTitle` "Kemungkinan duplikat".

- [ ] **Step 1: Tulis test** `CaptureSheet.test.ts`:
  - `prefills_form_from_draft`: nominal, payee, akun, kategori terisi.
  - `save_confirms_capture`: pilih kategori → Simpan → `listTxns` bertambah 1, emit `close`, toast `inbox.recorded`.
  - `snippet_is_collapsed_by_default`.
  - `shows_duplicates_with_link`: `Hubungkan` → capture tidak pending, jumlah transaksi tetap.
  - `not_pending_error_stays_open`: capture di-ignore lewat ipc sebelum simpan → pesan `errors.capture_not_pending` di form, sheet tetap terbuka.
- [ ] **Step 2: Tulis test** di `InboxView.test.ts`:
  - `reload_keeps_open_sheet`: buka Tinjau, ubah payee, jalankan `inbox.reload()` → sheet masih terbuka dan payee hasil edit tetap.
  - `duplicate_badge_opens_dialog`.
- [ ] **Step 3:** `pnpm --filter @nabungo/ui exec vitest run src/components/inbox src/views/InboxView.test.ts` → FAIL; implementasi; → PASS.
- [ ] **Step 4: Commit** `feat(ui): tinjau & hubungkan capture email`.

### Task 6: E2E, aksesibilitas, dokumentasi

**Files:**
- Create: `ui/e2e/inbox-9c.spec.ts`
- Modify: `ui/e2e/a11y.spec.ts` (route `/inbox` dengan data seed, light + dark, kedua tab)
- Modify: `CLAUDE.md` (bagian email capture: Inbox, `__seedEmailInbox`, `capture_not_ignored`), `docs/plans/2026-10-12-v1-roadmap.md` (baris 9c → Inbox + link plan + status; baris baru 9d Rule editor), `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` (status milestone 5 + bagian review flow milestone 6), `docs/design/design.md` (bila belum di Task 4)

- [ ] **Step 1: Tulis e2e** `inbox-9c.spec.ts` (onboarding seperti spec lain, lalu `page.evaluate(() => window.__seedEmailInbox(...))`):
  - `quick record shows in transactions`: `Catat` → buka Transaksi → baris payee terlihat.
  - `review with category then save`.
  - `bulk ignore + undo`: 5 capture tanggal berbeda, abaikan sampai tanggal ke-3 → tersisa 2; `Urungkan` → 5.
  - `link duplicate`: transaksi manual dibuat lewat UI, capture bernominal sama → badge → `Hubungkan` → jumlah transaksi tidak bertambah.
  - `sidebar badge follows pending count`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui e2e` → semua PASS (termasuk `a11y.spec.ts` 0 pelanggaran).
- [ ] **Step 3: Verifikasi akhir** (Global Constraints) → semua PASS. Cek manual `pnpm dev` dengan akun Gmail sungguhan: sync → badge muncul → Catat/Tinjau/Abaikan/undo di WebKitGTK.
- [ ] **Step 4: Commit** `docs: fase 9c inbox email capture`.
