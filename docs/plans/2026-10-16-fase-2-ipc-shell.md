# Fase 2 — IPC & App Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aplikasi bisa dibuka, menyimpan DB di folder data user, berkomunikasi dengan UI lewat binding TypeScript ter-generate, dan punya kerangka navigasi keyboard-first untuk semua layar PRD §9 (masih kosong).

**Architecture:** `nabungo-core` mendapat parser nominal (`25rb`, `1,5jt`, `25000+12500`). `nabungo-app` mendapat pengaturan UI (tema, bahasa) di tabel `setting`. `src-tauri` memegang `Mutex<Ledger>` sebagai state, me-resolve folder data (normal/portable), menulis log berputar, dan mengekspor command lewat `tauri-specta` ke `ui/src/ipc/bindings.ts`. Error domain dipetakan ke `IpcError { code, entity }` dan diterjemahkan di UI lewat i18n `errors.<code>`. UI mendapat layout sidebar, router lengkap, tema terang/gelap, command palette, dan shortcut.

**Tech Stack:** Rust 2024, `tauri-specta` 2.0.0-rc.25 + `specta` 2.0.0-rc.25 + `specta-typescript` 0.0.12, `tracing-appender` 0.2, `directories` 6; Vue 3, shadcn-vue (`sidebar`, `command`, `sheet`, `select`, `dialog`), `@vueuse/core`, vue-i18n, Pinia, Vitest.

**Spec:** `PRD.md` §7 (crate & pola akses DB), §8 (offline, privasi, portabilitas, i18n, aksesibilitas), §9 (layar, shortcut, input nominal, token warna). Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 2.

## Global Constraints

- Offline total: tidak ada aset/request remote; CSP di `src-tauri/tauri.conf.json` tidak dilonggarkan untuk origin eksternal.
- Tidak ada telemetry; log hanya ke `<data_dir>/logs/`.
- `src-tauri` tetap tipis: tidak ada logika bisnis, hanya DTO + pemetaan error + wiring.
- Semua panggilan backend dari UI lewat `ui/src/ipc/`; komponen tidak memanggil `invoke()` atau `commands.*` mentah.
- Nominal uang `i64` satuan minor; di TS diekspor sebagai `number` (aman karena `MAX_AMOUNT` = 10^12 < 2^53).
- Lokasi data: Linux `~/.local/share/nabungo/`, Windows `%APPDATA%\nabungo\`; portable = folder `data/` di samping executable bila file penanda `portable` ada di sana.
- Pesan UI wajib ada di `id` dan `en`; skema pesan diturunkan dari `ui/src/i18n/id.ts`.
- Shortcut (PRD §9): `N` transaksi baru, `G D` dasbor, `G T` transaksi, `G B` budget, `G A` alokasi, `G R` laporan, `Ctrl+,` pengaturan, `Ctrl+K` command palette.
- Migrasi yang sudah ada tidak diubah (fase ini tidak butuh migrasi; tabel `setting` sudah ada di `0001_init.sql`).
- Setiap task diakhiri `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test` hijau.

## Keputusan desain

- **Read pool ditunda ke Fase 5.** PRD §7 menyarankan pool koneksi baca, tapi sampai Fase 4 tidak ada query baca yang berat. Satu `Mutex<Ledger>` cukup; WAL sudah aktif. Pool ditambahkan bersama laporan (Fase 5) kalau benchmark menunjukkan lock contention. Roadmap diperbarui di Task 8.
- **Pengaturan UI disimpan di tabel `setting` di DB**, bukan `localStorage`, supaya "pindah laptop = copy satu folder" (PRD §8) juga membawa preferensi.
- **Parser nominal di Rust** (satu sumber kebenaran, roadmap Fase 2). UI memanggil `parse_amount` lewat IPC; latensi IPC lokal cukup kecil untuk dipanggil saat blur/enter, bukan setiap ketukan.
- **Gagal membuka DB saat startup**: `setup` Tauri mengembalikan error, alasan ditulis ke log dan stderr, proses keluar. Layar error yang ramah ditunda ke Fase 6 (integrity check).

## Review Focus

1. Shortcut satu huruf (`N`, `G …`) ditekan saat fokus di `<input>`, `<textarea>`, `<select>`, atau elemen `contenteditable` → tidak memicu aksi. Test di Task 7.
2. Nominal ambigu (`1.5`, `25.000,50`, `1.2345jt`, `10,5` tanpa sufiks) → error yang jelas, bukan angka yang diam-diam salah. Test di Task 1.
3. Nilai pengaturan rusak/tidak dikenal di DB (diedit manual, atau dari versi lain) → jatuh ke default, app tetap terbuka. Test di Task 2.
4. Run pertama: folder data belum ada → dibuat; mode portable aktif hanya bila penanda ada. Test di Task 3.
5. UI dijalankan di browser (`pnpm ui:dev`, tanpa Tauri) → semua panggilan IPC memakai fallback, layar tetap bisa dinavigasi tanpa error console. Test di Task 4.

---

### Task 1: Parser nominal `parse_amount` di core

**Files:**
- Create: `crates/core/src/amount.rs`
- Modify: `crates/core/src/lib.rs` (`pub mod amount;` + re-export)

**Interfaces:**
- Produces: `pub fn parse_amount(input: &str) -> Result<i64, AmountParseError>`; `pub enum AmountParseError { Empty, Invalid, Fractional, TooLarge }` (derive `Debug, Clone, Copy, PartialEq, Eq, thiserror::Error`). Re-export keduanya dari `nabungo_core`.

Grammar (case-insensitive, spasi diabaikan): `ekspresi = ["-"] suku (("+" | "-") suku)*`; `suku = angka [sufiks]`; `sufiks = "rb" (×1.000) | "jt" (×1.000.000)`; `angka = digit+ ("." digit{3})* ["," digit+]`. `.` hanya pemisah ribuan (grup tepat 3 digit), `,` hanya pemisah desimal. Desimal hanya boleh bila hasil suku setelah dikali sufiks bulat. Hitung pakai `i128`, tolak bila |hasil| > `MAX_AMOUNT`. Tidak ada float (`#![deny(clippy::float_arithmetic)]`). Hasil 0 atau negatif dikembalikan apa adanya; menolak 0 adalah tugas `validate`.

- [ ] **Step 1: Tulis test yang gagal** — test tabel di `amount.rs`:

```rust
#[test]
fn parses_table() {
    let ok = [
        ("25000", 25_000), ("25.000", 25_000), ("1.250.000", 1_250_000),
        ("25rb", 25_000), ("25 RB", 25_000), ("1,5jt", 1_500_000), ("1,25rb", 1_250),
        ("2jt", 2_000_000), ("25000+12500", 37_500), ("50rb - 12.500", 37_500),
        ("-5000", -5_000), ("1jt+250rb+3.000", 1_253_000), ("0", 0),
    ];
    for (input, want) in ok {
        assert_eq!(parse_amount(input), Ok(want), "{input}");
    }
}

#[test]
fn rejects_table() {
    use AmountParseError::*;
    let bad = [
        ("", Empty), ("   ", Empty),
        ("1.5", Invalid), ("25.00", Invalid), ("1.2345", Invalid), ("abc", Invalid),
        ("25++1", Invalid), ("25+", Invalid), ("rb", Invalid), ("1,5,0jt", Invalid),
        ("25.000,50", Fractional), ("10,5", Fractional), ("1,2345rb", Fractional),
        ("1000001jt", TooLarge), ("999999999999999999999999", TooLarge),
    ];
    for (input, want) in bad {
        assert_eq!(parse_amount(input), Err(want), "{input}");
    }
}

#[test]
fn max_amount_boundary() {
    assert_eq!(parse_amount("1.000.000jt"), Ok(MAX_AMOUNT));
    assert_eq!(parse_amount("1.000.000jt+1"), Err(AmountParseError::TooLarge));
}
```

- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-core amount::` → gagal compile (`parse_amount` belum ada).
- [ ] **Step 3: Implementasi** `parse_amount` sesuai grammar di atas (tokenizer tangan, tanpa regex/dependensi baru).
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test -p nabungo-core amount::` → PASS.
- [ ] **Step 5: Commit** — `feat(core): parse amount expressions with rb/jt suffixes`

---

### Task 2: Pengaturan UI (tema, bahasa) di storage + app

**Files:**
- Create: `crates/storage/src/setting.rs`, `crates/app/src/settings.rs`
- Modify: `crates/storage/src/lib.rs` (`pub mod setting;`), `crates/app/src/lib.rs` (`mod settings;` + re-export)

**Interfaces:**
- Produces (storage): `pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>>`; `pub fn set_setting(tx: &Transaction, key: &str, value: &str) -> Result<()>` (upsert `INSERT … ON CONFLICT(key) DO UPDATE`).
- Produces (app): `pub enum Theme { Light, Dark, System }` (default `System`, disimpan `"light" | "dark" | "system"` di key `ui.theme`); `pub enum Locale { Id, En }` (default `Id`, disimpan `"id" | "en"` di key `ui.locale`); `pub struct UiSettings { pub theme: Theme, pub locale: Locale }`; method di `impl Ledger` (blok `impl` di `settings.rs`): `pub fn ui_settings(&self) -> Result<UiSettings, AppError>`, `pub fn set_theme(&mut self, theme: Theme) -> Result<(), AppError>`, `pub fn set_locale(&mut self, locale: Locale) -> Result<(), AppError>`. Ketiga enum/struct derive `Debug, Clone, Copy, PartialEq, Eq, Default`. Re-export dari `nabungo_app`.
- `Ledger.conn` perlu `pub(crate)` agar `settings.rs` bisa memakainya.

- [ ] **Step 1: Tulis test yang gagal**
  - storage `setting_upsert_roundtrip`: `get_setting` key belum ada → `None`; `set_setting("ui.theme","dark")` lalu `"light"` → `get_setting` = `Some("light")`.
  - app `ui_settings_default_when_empty`: `Ledger::open_in_memory()` → `UiSettings { theme: System, locale: Id }`.
  - app `ui_settings_roundtrip`: `set_theme(Dark)`, `set_locale(En)` → `ui_settings()` sama.
  - app `unknown_setting_value_falls_back_to_default`: `INSERT INTO setting VALUES ('ui.theme','neon'), ('ui.locale','fr')` → `ui_settings()` = default, bukan error.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-storage setting && cargo test -p nabungo-app settings` → gagal compile.
- [ ] **Step 3: Implementasi** sesuai Interfaces.
- [ ] **Step 4: Jalankan, pastikan lulus** — perintah yang sama → PASS.
- [ ] **Step 5: Commit** — `feat(app): persist UI theme and locale in setting table`

---

### Task 3: Folder data, log lokal, dan state `Ledger` di Tauri

**Files:**
- Create: `src-tauri/src/paths.rs`, `src-tauri/src/logging.rs`, `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (tambah `tracing-appender = "0.2.5"`, fitur `env-filter` untuk `tracing-subscriber`; dev-dep `tempfile`)

**Interfaces:**
- Produces: `pub struct DataDir { pub path: PathBuf, pub portable: bool }`; `pub fn resolve_data_dir(exe_dir: &Path, base_data_dir: Option<&Path>) -> anyhow::Result<DataDir>` — portable bila `exe_dir/portable` ada → `exe_dir/data`; selain itu `base_data_dir/nabungo` (dipanggil dengan `directories::BaseDirs::new().map(|b| b.data_dir())`); `None` → error. Membuat folder secara rekursif. Path DB = `path.join("nabungo.db")`.
- Produces: `pub fn init_logging(dir: &Path) -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard>` — `RollingFileAppender` harian, prefix `nabungo`, sufiks `log`, `max_log_files(14)`, ke `<data_dir>/logs`; di debug build juga ke stdout. Filter default `info`, bisa diubah lewat `RUST_LOG`.
- Produces: `pub struct AppState { pub ledger: Mutex<Ledger>, pub data_dir: DataDir }` — di-`manage` di `setup`. Guard log disimpan sebagai state terpisah supaya hidup selama app jalan.

- [ ] **Step 1: Tulis test yang gagal** (di `paths.rs`, pakai `tempfile::tempdir()`):
  - `uses_base_dir_and_creates_it`: base = `tmp/share` (belum ada) → `path == tmp/share/nabungo`, `portable == false`, folder ada.
  - `portable_marker_wins`: buat file `exe/portable` → `path == exe/data`, `portable == true`, folder ada.
  - `no_base_dir_is_error`: `resolve_data_dir(exe, None)` tanpa penanda → `Err`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-desktop paths::` → gagal compile.
- [ ] **Step 3: Implementasi** `paths.rs`, `logging.rs`, `state.rs`; ganti `tracing_subscriber::fmt::init()` di `run()` dengan: resolve data dir (dari `std::env::current_exe()?.parent()`), `init_logging`, `Ledger::open(db_path)`, `app.manage(...)` di dalam `.setup(...)`. Log satu baris `info!` berisi path data dan flag portable saat startup.
- [ ] **Step 4: Jalankan, pastikan lulus** — `cargo test -p nabungo-desktop` → PASS; `pnpm dev` → jendela terbuka, `~/.local/share/nabungo/nabungo.db` dan `~/.local/share/nabungo/logs/nabungo.<tanggal>.log` ada.
- [ ] **Step 5: Commit** — `feat(desktop): data dir resolution, rolling local logs, Ledger state`

---

### Task 4: `tauri-specta`, `IpcError`, command, dan facade `ui/src/ipc`

**Files:**
- Create: `src-tauri/src/error.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/dto.rs`, `ui/src/ipc/bindings.ts` (generated, di-commit), `ui/src/ipc/browser.ts`, `ui/src/ipc/index.test.ts`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (`tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }`, `specta = "=2.0.0-rc.25"`, `specta-typescript = "0.0.12"`), `ui/src/ipc/index.ts`, `ui/src/views/DashboardView.vue`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces (Rust): `#[derive(Serialize, specta::Type)] pub struct IpcError { pub code: ErrorCode, pub entity: Option<String> }`; `#[serde(rename_all = "snake_case")] pub enum ErrorCode { NotFound, Archived, OpeningNotAllowed, TxnNoPostings, TxnZeroAmount, TxnAmountTooLarge, TxnMissingCategory, TxnWrongCategoryKind, TxnUnexpectedCategory, TxnTransferLegCount, TxnTransferSameAccount, TxnCurrencyMismatch, TxnUnbalanced, TxnOverflow, TxnValuationNonInvestment, TxnOpeningPostingCount, AmountEmpty, AmountInvalid, AmountFractional, AmountTooLarge, Internal }`. `impl From<AppError> for IpcError` dan `impl From<AmountParseError> for IpcError` memakai `match` **tanpa wildcard** supaya varian baru memaksa update. `Storage`/`Sqlite` → `Internal` + `tracing::error!` dengan detail (detail tidak dikirim ke UI). `entity` diisi untuk `NotFound`/`Archived`.
- Produces (DTO): `AppInfo { version: String, data_dir: String, portable: bool }`; `ThemeDto { Light, Dark, System }` dan `LocaleDto { Id, En }` (`rename_all = "lowercase"`) dengan `From` dua arah ke `Theme`/`Locale`; `UiSettingsDto { theme: ThemeDto, locale: LocaleDto }`.
- Produces (commands, semua `#[tauri::command] #[specta::specta]`): `app_info(state) -> AppInfo`; `get_ui_settings(state) -> Result<UiSettingsDto, IpcError>`; `set_theme(state, theme: ThemeDto) -> Result<(), IpcError>`; `set_locale(state, locale: LocaleDto) -> Result<(), IpcError>`; `parse_amount(input: String) -> Result<i64, IpcError>`. Command `app_version` dihapus.
- Produces (TS): `ui/src/ipc/index.ts` mengekspor `appInfo()`, `getUiSettings()`, `setTheme(t)`, `setLocale(l)`, `parseAmount(s)` yang mengembalikan data langsung atau melempar `class IpcFailure extends Error { readonly error: IpcError }`; tipe `IpcError`, `ErrorCode`, `ThemeDto`, `LocaleDto` di-re-export dari `bindings.ts`. Bila `!isTauri()`, semua fungsi memakai `browser.ts` (settings di memori, `appInfo` → `{ version: 'browser-dev', dataDir: '', portable: false }`, `parseAmount` → `Number(input)` bila integer, selain itu `IpcFailure` kode `amount_invalid`).
- Builder: `tauri_specta::Builder::<tauri::Wry>::new().commands(collect_commands![...])`; di `cfg(debug_assertions)` ekspor ke `../ui/src/ipc/bindings.ts` saat startup. i64 diekspor sebagai `number` (atur perilaku BigInt di `specta_typescript::Typescript`; cek nama opsinya di docs.rs untuk versi yang di-pin). Fungsi `pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry>` dipakai bersama oleh `run()` dan test ekspor.

- [ ] **Step 1: Tulis test yang gagal**
  - Rust `error.rs`: `not_found_maps_with_entity` (`AppError::NotFound("account")` → `{ code: NotFound, entity: Some("account") }`), `txn_error_maps_to_specific_code` (`TxnError::TransferSameAccount` → `TxnTransferSameAccount`), `amount_error_maps` (`AmountParseError::Fractional` → `AmountFractional`), `error_serializes_snake_case` (`serde_json::to_value` → `{"code":"txn_zero_amount","entity":null}`).
  - Rust `lib.rs`: `export_bindings` — `specta_builder().export(<Typescript dengan i64→number>, "../ui/src/ipc/bindings.ts")` sukses.
  - TS `index.test.ts` (jsdom, tanpa Tauri): `appInfo()` → `version === 'browser-dev'`; `setTheme('dark')` lalu `getUiSettings()` → `theme === 'dark'`; `parseAmount('1,5jt')` → melempar `IpcFailure` dengan `error.code === 'amount_invalid'`.
- [ ] **Step 2: Jalankan, pastikan gagal** — `cargo test -p nabungo-desktop` dan `pnpm --filter @nabungo/ui exec vitest run src/ipc` → gagal.
- [ ] **Step 3: Implementasi** Rust (`error.rs`, `dto.rs`, `commands.rs`, builder di `lib.rs`), jalankan `cargo test -p nabungo-desktop export_bindings` untuk menghasilkan `bindings.ts`, lalu facade TS + `browser.ts`. `DashboardView.vue` memakai `appInfo()` (menampilkan `core v{version}`).
- [ ] **Step 4: Jalankan, pastikan lulus** — kedua perintah di Step 2 → PASS; `pnpm --filter @nabungo/ui typecheck` → bersih.
- [ ] **Step 5: Tambah cek binding di CI** — job `rust`, setelah `cargo test`, hanya `if: runner.os == 'Linux'`: `git diff --exit-code ui/src/ipc/bindings.ts` (di Windows dilewati karena line ending checkout).
- [ ] **Step 6: Commit** — `feat(desktop): typed IPC via tauri-specta with domain error codes`

---

### Task 5: i18n error, tema terang/gelap, token warna semantik, store pengaturan

**Files:**
- Create: `ui/src/lib/theme.ts`, `ui/src/lib/theme.test.ts`, `ui/src/stores/settings.ts`, `ui/src/lib/errors.ts`, `ui/src/lib/errors.test.ts`
- Modify: `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`, `ui/src/style.css`, `ui/src/App.vue`, `ui/src/lib/money.ts` (tipe locale pakai `LocaleDto`)

**Interfaces:**
- Consumes: `getUiSettings`, `setTheme`, `setLocale`, `IpcFailure`, `ErrorCode`, `ThemeDto`, `LocaleDto` dari `@/ipc`.
- Produces: `resolveTheme(pref: ThemeDto, systemDark: boolean): 'light' | 'dark'`; `applyTheme(resolved: 'light' | 'dark'): void` (toggle class `dark` di `document.documentElement`); `useSettingsStore()` (Pinia) dengan state `theme`, `locale`, aksi `load()`, `setTheme(t)`, `setLocale(l)` yang memanggil IPC, memperbarui `i18n.global.locale`, dan menerapkan tema (mengikuti `matchMedia('(prefers-color-scheme: dark)')` bila `system`); `errorMessage(e: unknown, t: ComposerTranslation): string` → `IpcFailure` → `t('errors.' + code, { entity })`, selain itu `t('errors.internal')`.
- i18n: `errors` berisi satu kunci per `ErrorCode` di `id.ts` dan `en.ts`. Di `id.ts` gunakan `satisfies Record<ErrorCode, string>` pada objek `errors` agar kode baru dari binding gagal di typecheck bila belum diterjemahkan. Contoh `id`: `txn_transfer_same_account: 'Akun asal dan tujuan transfer harus berbeda'`, `not_found: '{entity} tidak ditemukan'`, `internal: 'Terjadi kesalahan. Detail ada di log aplikasi.'`.
- Token CSS (di `:root` dan `.dark`, dipetakan di `@theme inline` sebagai `--color-income`, `--color-expense`, `--color-transfer`): income hijau, expense merah, transfer netral (pakai nilai `--muted-foreground`). Kontras teks terhadap `--background` minimal 4.5:1 (WCAG AA) di kedua tema.

- [ ] **Step 1: Tulis test yang gagal**
  - `theme.test.ts`: `resolveTheme('system', true) === 'dark'`, `resolveTheme('system', false) === 'light'`, `resolveTheme('light', true) === 'light'`, `resolveTheme('dark', false) === 'dark'`.
  - `errors.test.ts`: dengan `i18n.global.t` locale `id`, `errorMessage(new IpcFailure({ code: 'txn_transfer_same_account', entity: null }), t)` → string di atas; `errorMessage(new Error('x'), t)` → pesan `internal`; setelah ganti locale ke `en` pesan berubah ke bahasa Inggris.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui exec vitest run src/lib` → gagal.
- [ ] **Step 3: Implementasi**; `App.vue` memanggil `settings.load()` saat mount.
- [ ] **Step 4: Jalankan, pastikan lulus** — perintah yang sama + `typecheck` → PASS.
- [ ] **Step 5: Commit** — `feat(ui): theme, semantic money colors, translated IPC errors`

---

### Task 6: Layout sidebar, router semua layar, halaman Pengaturan

**Files:**
- Create: `ui/src/components/AppSidebar.vue`, `ui/src/views/{Transactions,Budget,Allocation,Reports,Accounts,Categories,Scheduled,Settings}View.vue`, `ui/src/views/PlaceholderView.vue`, `ui/src/router.test.ts`
- Modify: `ui/src/router.ts`, `ui/src/App.vue`, `ui/src/i18n/id.ts`, `ui/src/i18n/en.ts`
- Add shadcn (dari `ui/`): `CI=true pnpm dlx shadcn-vue@latest add sidebar select --yes`. Setelah add, cek tidak ada `@import url(` remote di `style.css`.

**Interfaces:**
- Produces: route bernama `dashboard` (`/`), `transactions`, `budget`, `allocation`, `reports`, `accounts`, `categories`, `scheduled`, `settings` (path = `/<nama>`), diekspor juga sebagai `export const routes` dan `export type RouteName` (union sembilan nama itu). Setiap view placeholder memakai `PlaceholderView` dengan judul dari i18n `nav.<nama>` dan teks `common.comingSoon`.
- `AppSidebar`: grup navigasi utama (dasbor, transaksi, budget, alokasi, laporan), grup kelola (akun, kategori, terjadwal), pengaturan di footer; setiap item menampilkan shortcut-nya (`G D`, dst.) sebagai `<kbd>`; tombol "+ Transaksi" di header sidebar (aksinya disambung di Task 7). Item aktif menandai route sekarang.
- `SettingsView`: dua `Select` — Tema (Terang/Gelap/Ikuti sistem) dan Bahasa (Indonesia/English) — terikat ke `useSettingsStore()`.

- [ ] **Step 1: Tulis test yang gagal** — `router.test.ts`: `routes.map(r => r.name)` sama dengan sembilan nama di atas; untuk setiap nama, `id.nav[name]` dan `en.nav[name]` terdefinisi dan tidak kosong.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui exec vitest run src/router.test.ts` → gagal.
- [ ] **Step 3: Implementasi** router, view, sidebar, settings.
- [ ] **Step 4: Jalankan, pastikan lulus** — test + typecheck → PASS; `pnpm dev`: klik semua item sidebar, ganti tema & bahasa di Pengaturan, tutup lalu buka lagi app → pilihan tetap.
- [ ] **Step 5: Commit** — `feat(ui): sidebar shell, routes for all screens, settings page`

---

### Task 7: Shortcut keyboard, command palette, sheet "Transaksi baru"

**Files:**
- Create: `ui/src/lib/shortcuts.ts`, `ui/src/lib/shortcuts.test.ts`, `ui/src/components/CommandPalette.vue`, `ui/src/components/NewTxnSheet.vue`, `ui/src/stores/ui.ts`
- Modify: `ui/src/App.vue`, `ui/src/components/AppSidebar.vue`, i18n
- Add shadcn: `CI=true pnpm dlx shadcn-vue@latest add command sheet dialog --yes`

**Interfaces:**
- Produces: `type ShortcutAction = { kind: 'navigate'; route: RouteName } | { kind: 'newTxn' } | { kind: 'palette' }`; `createShortcutHandler(opts: { onAction: (a: ShortcutAction) => void; now?: () => number; sequenceTimeoutMs?: number }): (e: KeyboardEvent) => void` (default timeout 1000 ms). Pemetaan: `n` → `newTxn`; `g` lalu `d|t|b|a|r` → `navigate` ke `dashboard|transactions|budget|allocation|reports`; `Ctrl+K`/`Meta+K` → `palette`; `Ctrl+,`/`Meta+,` → `navigate settings`. Shortcut satu huruf diabaikan bila ada modifier (`Ctrl/Meta/Alt`), atau target event adalah `input`, `textarea`, `select`, atau `isContentEditable`; `Ctrl+K` tetap jalan dari input. Aksi yang terpicu memanggil `preventDefault()`.
- Produces: `useUiStore()` dengan `paletteOpen: boolean`, `newTxnOpen: boolean`. `App.vue` memasang handler di `window` `keydown` (lepas saat unmount) dan menerjemahkan aksi ke router/store.
- `CommandPalette`: `CommandDialog` berisi grup "Navigasi" (sembilan layar, dengan shortcut) dan "Aksi" (Transaksi baru). Enter menjalankan item; Esc menutup.
- `NewTxnSheet`: `Sheet` kanan dengan judul `txn.new` dan teks bahwa form datang di Fase 4. Dibuka oleh `N`, palette, dan tombol "+ Transaksi" di sidebar.

- [ ] **Step 1: Tulis test yang gagal** — `shortcuts.test.ts` dengan `now` palsu dan `new KeyboardEvent('keydown', { key, ctrlKey })` yang di-dispatch ke elemen target:
  - `n` di `document.body` → `{ kind: 'newTxn' }`.
  - `g` lalu `t` dalam 500 ms → navigate `transactions`; `g` lalu `t` setelah 1500 ms → tidak ada aksi.
  - `g` lalu `x` lalu `d` → tidak ada aksi untuk `x`; `d` sendiri tidak memicu apa pun.
  - `n` dan `g`,`d` dengan target `<input>`, `<textarea>`, dan `<div contenteditable>` → tidak ada aksi.
  - `Ctrl+K` dari `<input>` → `palette`; `Ctrl+,` → navigate `settings`; `Ctrl+N` → tidak ada aksi.
- [ ] **Step 2: Jalankan, pastikan gagal** — `pnpm --filter @nabungo/ui exec vitest run src/lib/shortcuts.test.ts` → gagal.
- [ ] **Step 3: Implementasi** handler, store, palette, sheet.
- [ ] **Step 4: Jalankan, pastikan lulus** — test + typecheck → PASS; `pnpm dev`: semua shortcut di Global Constraints jalan, Tab/Shift+Tab menjangkau semua item sidebar dengan focus ring terlihat.
- [ ] **Step 5: Commit** — `feat(ui): keyboard shortcuts, command palette, new transaction sheet`

---

### Task 8: Verifikasi exit criteria & update dokumen

**Files:**
- Modify: `docs/plans/2026-10-12-v1-roadmap.md`, `CLAUDE.md`

- [ ] **Step 1: Cek CSP & offline** — `pnpm dev`, buka devtools webview (klik kanan → Inspect), tab Console & Network saat menavigasi semua layar: 0 pelanggaran CSP, 0 request ke origin selain `ipc:`/`localhost`. `grep -rn "https://" ui/src --include=*.{ts,vue,css} | grep -v components/ui` → kosong.
- [ ] **Step 2: Cek build release** — `pnpm build` sukses; jalankan AppImage hasil build sekali, pastikan DB & log dibuat di folder data dan tema tersimpan.
- [ ] **Step 3: Update roadmap** — tandai Fase 2 selesai, catat keputusan "read pool ditunda ke Fase 5", tambahkan link plan ini di tabel.
- [ ] **Step 4: Update CLAUDE.md** — bagian Commands: binding TS di-generate otomatis oleh `pnpm dev` / `cargo test -p nabungo-desktop export_bindings` dan wajib di-commit; lokasi folder data & mode portable; error IPC = `IpcError.code` → `errors.<code>` di i18n.
- [ ] **Step 5: Commit** — `docs: mark Fase 2 done, document IPC bindings and data dir`

## Exit criteria Fase 2

- Semua test hijau, clippy bersih, `bindings.ts` sesuai hasil generate (dicek CI).
- Navigasi keyboard penuh antar sembilan layar kosong (shortcut + palette + Tab).
- CSP tidak memblokir apa pun yang dipakai app; 0 request keluar.
- Pengaturan tema & bahasa bertahan setelah restart; DB dan log ada di folder data yang benar (normal & portable).
- Setelah ini: tulis plan Fase 3 (`docs/plans/YYYY-MM-DD-fase-3-akun-kategori.md`).
