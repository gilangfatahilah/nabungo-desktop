# Fase 8a — Performa dengan 50.000 transaksi Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Membuktikan (dan bila perlu memperbaiki) target performa PRD §8 dengan data 50.000 transaksi: startup sampai dasbor < 1,5 s, simpan transaksi < 100 ms, buka daftar transaksi < 200 ms, laporan 12 bulan & net worth over time < 500 ms. Semua diukur untuk DB plaintext **dan** terenkripsi SQLCipher.

**Architecture:**
- `nabungo-app::fixture`: generator data deterministik (PRNG SplitMix64 tanpa dependensi). Generator menulis lewat use case yang sudah ada, dan transaksi ditulis per batch lewat `write_txn` dalam satu SQLite transaction.
- `crates/app/examples/seed_fixture.rs`: menulis file DB fixture (opsional terenkripsi) untuk diukur manual di jendela Tauri.
- `crates/app/tests/perf.rs`: harness `#[ignore]` yang mengukur skenario S1–S6 (K4) pada DB file nyata (WAL, PRAGMA produksi), mencetak tabel, dan gagal bila ada skenario di atas budget.
- Optimasi O1–O3 dikerjakan **hanya bila** baseline melewati pemicunya (K6). Tiap optimasi punya test kesetaraan hasil sebelum dan sesudah.
- Pengukuran startup end-to-end: command `app_ready` mencatat `startup_ms` ke log lokal setelah dasbor pertama selesai dimuat.

**Tech Stack:** Rust (tanpa dependensi baru; `std::time::Instant`, `tempfile` sudah ada di dev-deps), rusqlite/SQLCipher, Tauri 2 + tauri-specta, Vue 3.

**Spec:**
- `PRD.md` §7 "Pola akses database" (pool baca, `account_balance_daily` opsional), §8 baris Performa.
- Roadmap: `docs/plans/2026-10-12-v1-roadmap.md` Fase 8, baris 8a.
- Keputusan tertunda: K11 plan Fase 5 (pool koneksi baca diukur di Fase 8).

**Di luar plan ini:**
- Performa render UI (virtual list sudah ada sejak Fase 4). Hanya startup end-to-end yang diukur di jendela Tauri.
- Benchmark sebagai gerbang CI: waktu di runner GitHub tidak stabil. CI hanya menjalankan smoke test fixture kecil (Task 1).
- Pengukuran di Windows (WebView2) ikut 8d setelah CI Windows aktif.
- `account_balance_daily` dan pool koneksi baca, kecuali pemicu di K7 terpenuhi.

## Keputusan

| # | Keputusan | Alasan |
|---|---|---|
| K1 | **Isi fixture** (`FixtureSpec { txns: 50_000, seed: 42, end }`): onboarding locale `id` (start day 25), 10 akun (2 bank, tunai, 2 e-wallet, kartu kredit, investasi, tabungan, piutang, satu akun arsip), 40 payee, 15 tag. Tanggal tersebar merata selama 36 periode yang berakhir di `end`. Campuran transaksi: 70% expense (10% di antaranya split 2–3 kategori), 8% income, 15% transfer (20% di antaranya dengan biaya), 5% refund (expense negatif), 2% dihapus (soft delete). Ditambah valuation bulanan untuk akun investasi, budget 12 kategori expense di setiap periode (3 kategori dengan rollover), 6 bucket alokasi (2 tingkat, 1 bertarget nominal) dengan mapping, dan 8 jadwal berulang tanpa auto-post. Nominal kelipatan 500, expense 5.000–500.000, income gaji 15.000.000 per periode. | Mendekati pemakaian nyata beberapa tahun dan menyentuh setiap jalur query: split, biaya, rollover, alokasi bertingkat, posting terhapus. |
| K2 | **Generator deterministik:** seed yang sama menghasilkan isi DB yang sama (ID UUIDv7 boleh berbeda). PRNG SplitMix64 ditulis di modul (±10 baris). Transaksi ditulis per batch 1.000 dalam satu SQLite transaction lewat `crate::transactions::write_txn`, bukan 50.000 kali `create_txn`. | Generator selesai dalam hitungan detik. Hasil ukur bisa dibandingkan antar-run tanpa menambah dependensi. |
| K3 | **Fixture adalah `pub mod fixture` biasa** di `nabungo-app`, tanpa feature flag. Modul ini tidak pernah dipanggil `src-tauri`. | Ikut clippy `--all-targets` dan test biasa tanpa matriks feature. Linker membuang kode yang tidak dipakai binary. |
| K4 | **Skenario & budget** (median 7 run setelah 1 warm-up, `--release`, DB file di tempdir, mesin dev Fedora user). Budget sama untuk plaintext dan SQLCipher: **S1** `open_with_backups` tanpa migrasi ≤ 150 ms. **S2** startup backend = S1 + `post_auto_due` + set dasbor (`accounts_overview`, `list_categories`, `period_of`, `list_txns` limit 8, `budget_overview`, `category_breakdown` expense, `allocation_overview`, `cashflow_report` 12, `net_worth_history` 12, `list_rules`), dijalankan berurutan, ≤ 700 ms. **S3** `auto_backup_if_due` saat jatuh tempo: dilaporkan saja, menjadi pemicu O2. **S4** `create_txn_with_alerts` (expense periode berjalan, termasuk commit) ≤ 100 ms. **S5** `list_txns` limit 100 ≤ 200 ms untuk: default; filter akun tersibuk (saldo berjalan); search `"kopi"`; sort nominal; sort pihak dengan offset 20.000. **S6** ≤ 500 ms untuk: `cashflow_report` 12, `net_worth_history` 12, `category_breakdown`, `budget_overview`, `allocation_overview`, `allocation_history` 12. | S2 berurutan karena satu `Mutex<Ledger>` memang menserialkan command. Sisa 800 ms dari 1,5 s untuk jendela, WebView, dan Vue. S4 mengukur yang dijalankan command `create_txn`, bukan hanya `create_txn`. |
| K5 | **Startup end-to-end:** `AppState.started: Instant` diisi saat state dibuat di `run()`. Command baru `app_ready()` mencatat `tracing::info!(startup_ms, "dasbor pertama tampil")` sekali per proses (`AtomicBool`). `DashboardView` memanggil `ipc.appReady()` sekali setelah `load()` pertama selesai dan `nextTick`. Diukur manual 3× dengan release build mode portable + fixture. | Satu angka nyata di log lokal, tanpa telemetry. Mode browser: `appReady` no-op. |
| K6 | **Optimasi bersyarat.** O1–O3 dikerjakan hanya bila pemicunya terpenuhi di baseline Task 3. Optimasi yang tidak terpicu ditandai "tidak perlu" beserta angkanya di bagian Hasil. | Roadmap: "tambahkan `account_balance_daily` hanya bila profiling menunjukkan perlu". Tanpa data, optimasi hanya tebakan. |
| K7 | **Pool koneksi baca dan `account_balance_daily` tidak dikerjakan di 8a**, kecuali S2/S6 masih di atas budget setelah O1–O3. Bila itu terjadi, berhenti dan tulis plan tambahan. | Menutup K11 Fase 5 berdasarkan data. Pool menambah kompleksitas kunci/enkripsi (key per koneksi) dan baru berguna bila command berjalan paralel. |
| K8 | Harness hanya membaca API publik `nabungo_app` + `nabungo_storage::convert_file` (untuk varian terenkripsi, key `DbKey::generate()`). Hasil dicetak sebagai tabel Markdown (`skenario | plaintext ms | sqlcipher ms | budget`) agar bisa langsung disalin ke bagian Hasil. | Angka di dokumen sama persis dengan output harness. |

## Global Constraints

- Arah dependensi tetap: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`. `src-tauri` tanpa logika bisnis; `nabungo-core` tanpa IO.
- Optimasi tidak boleh mengubah hasil: saldo, net worth, realisasi budget, dan porsi alokasi tetap diturunkan dari `posting` (tidak ada kolom saldo).
- Migrasi `0001`–`0008` tidak diubah. Indeks baru (O3) masuk `0009_perf_indexes.sql` dan didaftarkan di `migrations()` `crates/storage/src/lib.rs`.
- Tidak ada dependensi baru (termasuk `criterion`/`rand`).
- Test perf `#[ignore]`; `cargo test --workspace` tetap cepat. Jalankan: `cargo test -p nabungo-app --release --test perf -- --ignored --nocapture`.
- `ui/src/ipc/bindings.ts` hanya lewat generate dan wajib di-commit. Command baru didaftarkan di `collect_commands!`. `browser-ledger`/fallback `ipc` mendapat no-op yang setara.
- Verifikasi akhir: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `pnpm --filter @nabungo/ui typecheck`, `pnpm --filter @nabungo/ui test`, `pnpm --filter @nabungo/ui e2e`.
- Commit diakhiri `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Transaksi terhapus di fixture.** 2% transaksi soft delete harus tidak ikut saldo, laporan, dan daftar. Test: Task 1 `fixture_deleted_txns_excluded` (net worth dari `accounts_overview` = Σ posting transaksi aktif yang dihitung langsung dengan SQL di test).
2. **DB terenkripsi lebih lambat** (dekripsi per halaman SQLCipher). Setiap skenario diukur di kedua varian; budget tidak dilonggarkan untuk SQLCipher. Test: Task 2 menjalankan S1–S6 untuk `Variant::Plain` dan `Variant::Encrypted`.
3. **Hasil optimasi O1 berbeda dari aturan lama** (mis. off-by-one pada batas tanggal `<=` hari terakhir periode). Test: Task 4 `period_balances_match_balances_as_of` membandingkan dengan `balances_as_of` per tanggal pada fixture kecil, termasuk periode tanpa transaksi dan tanggal sebelum transaksi pertama.
4. **Backup startup dipindah (O2) tetapi tidak berjalan lagi saat integrity check gagal.** Test: Task 5 `first_tick_respects_integrity` (fungsi murni `startup_backup_plan(&StartupReport) -> BackupPlan`).
5. **Paginasi dalam dengan sort pihak** (offset 20.000) memaksa SQLite mengurutkan seluruh tabel. Skenario S5 terakhir harus tetap ≤ 200 ms. Kalau tidak, O3 menambah indeks yang mendukung `ORDER BY` itu. Test: Task 2 skenario `list_sort_payee_deep`.

---

### Task 1: Generator fixture deterministik

**Files:**
- Create: `crates/app/src/fixture.rs`
- Modify: `crates/app/src/lib.rs` (`pub mod fixture;`)

**Interfaces:**
- Consumes: `Ledger::complete_onboarding`, `create_account`, `set_budget`, `set_budget_rollover`, `create_bucket`, `set_account_allocation`, `record_valuation`, `create_rule`, `delete_txn`, `crate::transactions::{form_input, write_txn}`.
- Produces:
  - `pub struct FixtureSpec { pub txns: u32, pub seed: u64, pub end: Date }` + `impl FixtureSpec { pub fn standard(end: Date) -> Self }` (50.000, seed 42)
  - `pub struct FixtureSummary { pub accounts: Vec<Uuid>, pub busiest_account: Uuid, pub expense_category: Uuid, pub txns: u32, pub deleted: u32 }`
  - `pub fn seed(ledger: &mut Ledger, spec: &FixtureSpec) -> Result<FixtureSummary, AppError>`: ledger harus belum onboarding (bila sudah, `AppError::AlreadyOnboarded` dari `complete_onboarding` diteruskan).

- [ ] **Step 1: Tulis test di `fixture.rs` (`#[cfg(test)]`, spec kecil `txns: 2_000`)**

```rust
#[test] fn fixture_is_deterministic() {
    // dua ledger in-memory, seed sama → saldo per nama akun & total per kategori 12 periode identik
    assert_eq!(snapshot(&a), snapshot(&b));
}
#[test] fn fixture_counts_and_mix() {
    // txns aktif + deleted == 2_000 (+ opening/valuation tidak dihitung di `txns`)
    // ada ≥1 split, ≥1 transfer berbiaya, ≥1 refund, budget rollover, bucket nominal, 8 rule
}
#[test] fn fixture_deleted_txns_excluded() {
    // net worth accounts_overview == SELECT SUM(p.amount) ... deleted_at IS NULL AND posted, akun include_in_net_worth (liability sesuai aturan networth)
}
```

- [ ] **Step 2: Jalankan `cargo test -p nabungo-app fixture::`.** Diharapkan: gagal kompilasi (`fixture` belum ada).
- [ ] **Step 3: Implementasikan `seed` sesuai K1/K2.** SplitMix64 lokal (`fn next(&mut self) -> u64`, `fn below(&mut self, n: u64) -> u64`). Tanggal transaksi ke-i = hari ke `i × total_hari / txns` sejak awal periode pertama, supaya tersebar merata. Gaji per periode dihitung di luar kuota acak.
- [ ] **Step 4: Jalankan test di atas + `cargo clippy -p nabungo-app --all-targets -- -D warnings`.** Diharapkan: PASS, tanpa warning.
- [ ] **Step 5: Commit** `feat(app): generator fixture deterministik untuk benchmark`.

### Task 2: Harness perf dan CLI fixture

**Files:**
- Create: `crates/app/tests/perf.rs`, `crates/app/examples/seed_fixture.rs`
- Modify: `CLAUDE.md` (bagian Commands: perintah perf & seed)

**Interfaces:**
- Consumes: `fixture::{seed, FixtureSpec, FixtureSummary}` (Task 1), `nabungo_storage::convert_file`, `DbKey::generate`, `Ledger::open_with_backups`.
- Produces: `enum Variant { Plain, Encrypted }`; output tabel Markdown (K8); `const BUDGETS: &[(&str, u128)]` dengan nama skenario: `open`, `startup_backend`, `auto_backup`(tanpa budget), `save`, `list_default`, `list_account`, `list_search`, `list_sort_amount`, `list_sort_payee_deep`, `cashflow_12`, `net_worth_12`, `breakdown`, `budget_overview`, `allocation_overview`, `allocation_history_12`.

- [ ] **Step 1: Tulis `perf.rs`.** Satu test `#[ignore] fn perf_50k()`: seed fixture ke `tmp/plain.db` (via `Ledger::open`), `convert_file` ke `tmp/enc.db`, lalu untuk tiap varian buka dengan `open_with_backups(path, tmp/backups, key, now)` dan ukur skenario K4 (`fn time<F: FnMut()>(f: F) -> u128` = median 7 run setelah 1 warm-up, dalam ms). `save` memakai tanggal unik per run agar tidak ada efek cache payee. `auto_backup` memakai folder backup kosong per run. Cetak tabel lalu `assert!` daftar skenario yang melebihi budget kosong (pesan berisi nama + angka).
- [ ] **Step 2: Tulis `seed_fixture.rs`**: `cargo run -p nabungo-app --release --example seed_fixture -- <dir> [--txns N] [--password P]`. Menulis `<dir>/nabungo.db`. Dengan `--password`, memanggil `enable_encryption(P, <dir>/backups, KdfParams::DEFAULT)` supaya `nabungo.key` valid untuk layar kunci Tauri, lalu mencetak recovery key. Menolak bila `<dir>/nabungo.db` sudah ada.
- [ ] **Step 3: Jalankan `cargo test -p nabungo-app --release --test perf -- --ignored --nocapture`.** Diharapkan: tabel tercetak. PASS/FAIL dicatat apa adanya, karena kegagalan di sini adalah input Task 3, bukan bug harness.
- [ ] **Step 4: Jalankan example ke tempdir dengan `--txns 1000` dan `--password rahasia-123`.** Diharapkan: `nabungo.db` + `nabungo.key` ada, recovery key tercetak.
- [ ] **Step 5: Commit** `test(app): harness perf 50.000 transaksi dan example seed_fixture`.

### Task 3: Baseline

**Files:**
- Modify: `docs/plans/2026-10-24-fase-8a-performa.md` (bagian **Hasil**)

- [ ] **Step 1: Jalankan harness 2× (release) di mesin dev.** Ambil run kedua (cache OS hangat sama di kedua run).
- [ ] **Step 2: Isi tabel "Baseline" di bagian Hasil.** Tandai pemicu O1–O3 yang terpenuhi (K6). Untuk skenario yang lambat, sertakan `EXPLAIN QUERY PLAN` query utamanya.
- [ ] **Step 3: Commit** `docs: baseline performa Fase 8a`.

### Task 4 (O1, bersyarat): Saldo per periode dalam satu pass

**Pemicu:** `net_worth_12` atau `allocation_history_12` > 250 ms (setengah budget, karena keduanya ikut S2), atau S2 di atas budget dengan `net_worth_12` sebagai skenario terbesar.

**Files:**
- Modify: `crates/storage/src/report.rs` (fungsi baru), `crates/app/src/allocation.rs` (`period_points`)

**Interfaces:**
- Produces: `pub fn balances_at_dates(conn: &Connection, dates: &[Date]) -> Result<Vec<Vec<(Uuid, i64)>>>`. `dates` urut naik; hasil[i] sama persis dengan `balances_as_of(conn, dates[i])`.

- [ ] **Step 1: Test di `report.rs`** `period_balances_match_balances_as_of`: fixture storage yang sudah ada + posting di batas tanggal; `dates` mencakup sebelum transaksi pertama, hari yang persis sama dengan transaksi, dan periode kosong; `assert_eq!(balances_at_dates(&c, &d)[i], balances_as_of(&c, d[i]))` untuk semua i.
- [ ] **Step 2: Jalankan; gagal kompilasi.**
- [ ] **Step 3: Implementasi.** Satu query `GROUP BY p.account_id, bucket` dengan `bucket` = indeks tanggal pertama `>= t.date` (via `CASE` yang dibangun dari `dates`, atau CTE `VALUES`), posting setelah tanggal terakhir dibuang. Prefix sum per akun dilakukan di Rust. `period_points` memakai fungsi ini.
- [ ] **Step 4: `cargo test --workspace` + harness.** Diharapkan: test lama net worth/alokasi lolos tanpa diubah; angka `net_worth_12` turun.
- [ ] **Step 5: Commit** `perf(storage): saldo semua periode dalam satu query`.

### Task 5 (O2, bersyarat): Backup otomatis keluar dari jalur startup

**Pemicu:** `auto_backup` > 200 ms di salah satu varian.

**Files:**
- Modify: `src-tauri/src/lifecycle.rs`

**Interfaces:**
- Produces: `pub enum BackupPlan { Disabled, Immediate }`, `pub fn startup_backup_plan(r: &StartupReport) -> BackupPlan` (Disabled ⇔ `!auto_backup_allowed(r)`). `open_unlocked` tetap menjalankan `run_auto_post` secara sinkron (hasilnya harus tampil di dasbor), tetapi tidak lagi menjalankan `run_auto_backup`. `spawn_backup_scheduler` menjalankan putaran pertama segera (backup lalu sleep, bukan sleep dulu).

- [ ] **Step 1: Test** `first_tick_respects_integrity`: `Failed` → `Disabled`; `Ok`/`Skipped` → `Immediate`.
- [ ] **Step 2: Jalankan; gagal kompilasi.**
- [ ] **Step 3: Implementasi sesuai Interfaces.** Urutan loop scheduler: kunci ledger → backup + auto-post → lepas kunci → sleep.
- [ ] **Step 4: `cargo test -p nabungo-desktop`, lalu cek manual `pnpm dev`.** Diharapkan: log "backup otomatis dibuat" muncul sesaat setelah start ketika backup jatuh tempo (folder `backups/` kosong).
- [ ] **Step 5: Commit** `perf(desktop): backup otomatis startup berjalan di thread latar`.

### Task 6 (O3, bersyarat): Indeks pendukung

**Pemicu:** skenario S5/S6 di atas budget yang `EXPLAIN QUERY PLAN`-nya menunjukkan `SCAN` tabel `posting`/`txn` atau `USE TEMP B-TREE FOR ORDER BY` yang bisa dihindari indeks.

**Files:**
- Create: `crates/storage/migrations/0009_perf_indexes.sql`
- Modify: `crates/storage/src/lib.rs` (`migrations()`, test versi skema terbaru)

Kandidat (pakai hanya yang terbukti menurunkan angka harness; satu baris komentar SQL per indeks menyebut skenarionya):
- `CREATE INDEX idx_posting_account_txn ON posting (account_id, txn_id, amount);`: saldo per akun, saldo berjalan.
- `CREATE INDEX idx_posting_txn_cat ON posting (txn_id, category_id, amount);`: `category_totals`.
- `CREATE INDEX idx_txn_posted_date ON txn (date, id) WHERE deleted_at IS NULL AND status = 'posted';`: filter rentang tanggal laporan.

- [ ] **Step 1: Test di `lib.rs`** `perf_indexes_exist`: setelah `open_in_memory()`, `sqlite_master` berisi setiap indeks yang dipilih.
- [ ] **Step 2: Jalankan; gagal.**
- [ ] **Step 3: Tulis migrasi `0009`**, daftarkan, lalu perbarui assert versi skema terbaru yang sudah ada.
- [ ] **Step 4: `cargo test --workspace` + harness + `EXPLAIN QUERY PLAN` ulang.** Diharapkan: semua test lolos; skenario terkait turun. Indeks yang tidak berpengaruh dibuang sebelum commit.
- [ ] **Step 5: Commit** `perf(storage): migrasi 0009 indeks laporan dan saldo`.

### Task 7: Startup end-to-end (`app_ready`)

**Files:**
- Modify: `src-tauri/src/state.rs` (`started: Instant`, `ready_logged: AtomicBool`), `src-tauri/src/lib.rs` (isi state, `collect_commands!`), `src-tauri/src/commands.rs`, `ui/src/ipc/index.ts`, `ui/src/ipc/bindings.ts` (generate), `ui/src/views/DashboardView.vue`, `ui/src/views/DashboardView.test.ts`

**Interfaces:**
- Produces: command `app_ready() -> ()` (tidak pernah error, tidak butuh ledger); `ipc.appReady(): Promise<void>` (mode browser: resolve tanpa efek).

- [ ] **Step 1: Test** `DashboardView.test.ts` → `calls appReady once after first load`: spy `ipc.appReady`; mount; flush; `expect(spy).toHaveBeenCalledTimes(1)`; picu `ledgerVersion` berubah; tetap 1.
- [ ] **Step 2: Jalankan `pnpm --filter @nabungo/ui exec vitest run src/views/DashboardView.test.ts`.** Diharapkan: FAIL.
- [ ] **Step 3: Implementasi K5.** Generate binding dengan `cargo test -p nabungo-desktop export_bindings`.
- [ ] **Step 4: Jalankan test UI + typecheck.** Diharapkan: PASS.
- [ ] **Step 5: Ukur manual.** `pnpm build`, buat file `portable` di samping `target/release/nabungo-desktop`, jalankan `seed_fixture` ke `target/release/data`, start 3× (plaintext). Ulangi dengan `--password` (waktu dihitung setelah unlock: catat juga `unlock` dari log terpisah). Catat `startup_ms` di Hasil.
- [ ] **Step 6: Commit** `feat(desktop): catat waktu startup sampai dasbor tampil`.

### Task 8: Hasil akhir dan roadmap

**Files:**
- Modify: `docs/plans/2026-10-24-fase-8a-performa.md` (Hasil), `docs/plans/2026-10-12-v1-roadmap.md` (baris 8a selesai + ringkasan keputusan 8a)

- [ ] **Step 1: Jalankan harness final**, isi tabel "Akhir" di samping Baseline, tandai O1–O3 "dikerjakan" atau "tidak perlu", dan catat keputusan K7 (pool/`account_balance_daily`) beserta angkanya.
- [ ] **Step 2: Verifikasi akhir** (perintah di Global Constraints). Diharapkan: semua hijau.
- [ ] **Step 3: Commit** `docs: Fase 8a selesai — hasil performa`.

## Hasil

_Diisi di Task 3 dan Task 8._ Fixture standar: 50.000 transaksi (988 terhapus), 10 akun, file DB ±45 MB. Baseline = run kedua `perf_50k` (median 7 run), Fedora, mesin dev.

| Skenario | Budget (ms) | Baseline plain | Baseline sqlcipher | Akhir plain | Akhir sqlcipher |
|---|---|---|---|---|---|
| `open` | 150 | 0.6 | 0.8 | | |
| `startup_backend` | 700 | 3242.2 | 15847.0 | | |
| `auto_backup` | — | 267.5 | 887.0 | | |
| `save` | 100 | 186.6 | 370.1 | | |
| `list_default` | 200 | 101.1 | 703.6 | | |
| `list_account` | 200 | 390.0 | 2143.9 | | |
| `list_search` | 200 | 298.0 | 1833.6 | | |
| `list_sort_amount` | 200 | 416.4 | 2655.0 | | |
| `list_sort_payee_deep` | 200 | 316.7 | 1528.0 | | |
| `cashflow_12` | 500 | 29.2 | 60.4 | | |
| `net_worth_12` | 500 | 2320.3 | 11498.4 | | |
| `breakdown` | 500 | 2.2 | 2.5 | | |
| `budget_overview` | 500 | 92.1 | 190.4 | | |
| `allocation_overview` | 500 | 314.6 | 1915.9 | | |
| `allocation_history_12` | 500 | 2361.9 | 11442.4 | | |

### Analisis baseline (Task 3)

18 pelanggaran budget. Pemicu K6:

- **O1 terpicu**: `net_worth_12` 2320 ms dan `allocation_history_12` 2362 ms (> 250 ms). Masing-masing 12× `balances_as_of` (±190 ms per panggilan).
- **O2 terpicu**: `auto_backup` 267,5 ms plain / 887 ms sqlcipher (> 200 ms).
- **O3 terpicu**: EXPLAIN menunjukkan scan penuh plus lookup baris `txn` per posting:
  - `balances_as_of`: `SCAN p USING INDEX idx_posting_account` lalu `SEARCH t USING INDEX sqlite_autoindex_txn_1 (id=?)`, yaitu 64.463 lookup acak per panggilan. Pola yang sama dipakai saldo di `list_accounts` (`accounts_overview`, `allocation_overview` 315 ms).
  - `COUNT(*)` di `list_txns`: `SCAN t USING INDEX idx_txn_payee` lalu membaca baris untuk `status`. Biaya tetap ±100 ms di setiap daftar (`list_default` hampir seluruhnya biaya ini).
  - Sort pihak: `SCAN t USING INDEX idx_txn_payee`, `SEARCH py`, `USE TEMP B-TREE FOR ORDER BY`.
- **Temuan di luar O1–O3**: SQLCipher 5–7× lebih lambat di skenario yang banyak membaca. Penyebabnya `cache_size` default (±2 MB) jauh di bawah ukuran DB, sehingga setiap scan mendekripsi ulang halaman. Kalau setelah O1–O3 masih di atas budget, K7 berlaku dan perlu plan tambahan (kandidat: `PRAGMA cache_size` lebih besar).

Startup end-to-end (`startup_ms`, 3 run): —
