# Fase 8a2: Performa lanjutan (K7 terpicu) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menutup sisa pelanggaran budget K4 Fase 8a setelah O1–O3: S2 `startup_backend`, S4 `save`, serta S5/S6 di varian SQLCipher.

**Architecture:** Lanjutan `docs/plans/2026-10-24-fase-8a-performa.md` (K1–K8 tetap berlaku, harness `crates/app/tests/perf.rs` tidak diubah budgetnya). Langkahnya dari yang termurah: cache halaman (satu PRAGMA), lalu memangkas kerja ganda di jalur simpan, terakhir memangkas startup berdasarkan profil per panggilan.

**Tech Stack:** Rust, rusqlite (bundled-sqlcipher), SQLite 3.51.

**Spec:** `PRD.md` §8 (startup < 1,5 s, simpan < 100 ms, daftar < 200 ms, laporan < 500 ms).

## Data awal (dari Hasil Fase 8a)

| Skenario | Budget | Setelah O1–O3 plain / sqlcipher | + eksperimen `cache_size` 64 MB |
|---|---|---|---|
| `startup_backend` (S2) | 700 | 873 / 3244 | 813 / 885 |
| `save` (S4) | 100 | 194 / 490 | 188 / 183 |
| `list_*` (S5) maks | 200 | 148 / 255 | 148 / 148 |
| S6 maks | 500 | 328 / 1260 | 280 / 280 |

`save` = `create_txn_with_alerts`: `budget_levels` memanggil `overview_on` penuh (±90 ms) **dua kali** (sebelum dan sesudah tulis), padahal hanya kategori di form yang bisa berubah level.

## Keputusan

| ID | Keputusan | Alasan |
|---|---|---|
| L1 | `PRAGMA cache_size = -65536` (64 MiB, batas atas; dialokasikan sesuai pemakaian) di `configure()` untuk semua koneksi. | Eksperimen: SQLCipher setara plaintext di semua skenario. Biaya memori ≤ 64 MiB per koneksi, dan app hanya punya satu koneksi tulis. |
| L2 | `budget_levels` hanya menghitung kategori yang disentuh form (beserta induknya), lewat varian `overview_on` yang menerima filter kategori. Hasil alert harus identik. | Menghapus ±2×90 ms dari S4 tanpa cache atau kolom turunan. |
| L3 | S2 diprofil per panggilan dulu (tabel waktu per command dasbor), baru dipilih satu dari: (a) memo saldo per-request yang dipakai bersama `accounts_overview`/`allocation_overview`/titik terakhir `net_worth_history`; (b) `account_balance_daily` (K7). (a) dipilih bila menutup budget. | Sebagian besar S2 adalah saldo semua akun yang dihitung berulang kali; (a) tidak menambah skema. |
| L4 | `startup_ms` end-to-end diukur manual oleh user (release build portable + fixture, 3×). Bila < 1,5 s meskipun S2 > 700 ms, pelanggaran S2 dicatat sebagai diterima, bukan dioptimasi lebih jauh. | PRD menargetkan end-to-end; 700 ms hanya pembagian internal K4. |

## Global Constraints

- Sama dengan Fase 8a: saldo/net worth/budget/alokasi tetap diturunkan dari `posting`; tidak ada network; migrasi rilis tidak diubah.

## Review Focus

1. `cache_size` berlaku juga untuk koneksi `open_file_read_only` dan koneksi `convert_file`/backup, dan tidak mengubah hasil apa pun.
2. Alert budget setelah simpan split multi-kategori, kategori anak dengan budget di induk, dan rollover: identik dengan sebelum L2.
3. Transfer dengan biaya (posting biaya berkategori) tetap memicu alert kategori biaya.

---

### Task 1: Cache halaman 64 MiB (L1)

**Files:** Modify `crates/storage/src/lib.rs` (`configure`).

- [ ] **Step 1: Test** `cache_size_configured` di `lib.rs`: `open_in_memory()` → `PRAGMA cache_size` == `-65536`.
- [ ] **Step 2: Jalankan; gagal** (default `-2000`).
- [ ] **Step 3: Tambah PRAGMA di `configure`.**
- [ ] **Step 4: `cargo test --workspace` + harness.** Diharapkan: semua S5/S6 lolos di kedua varian.
- [ ] **Step 5: Commit** `perf(storage): cache halaman 64 MiB untuk SQLCipher`.

### Task 2: Alert budget hanya untuk kategori yang disentuh (L2)

**Files:** Modify `crates/app/src/budget.rs` (`overview_on` + filter), `crates/app/src/budget_alerts.rs` (`budget_levels`).

- [ ] **Step 1: Test** `alerts_scoped_to_touched_categories_match_full` di `budget_alerts.rs`: ledger dengan budget di 3 kategori (salah satunya induk dengan anak); simpan split ke 2 kategori yang melewati 80%/100% → `assert_eq!` alert dengan hasil versi penuh (helper test yang memanggil `overview_on` tanpa filter); kategori ketiga tidak muncul.
- [ ] **Step 2: Jalankan; gagal kompilasi.**
- [ ] **Step 3: Implementasi.** Filter = kategori posting form ∪ induknya; `create/update/confirm_*_with_alerts` memakai filter (update: gabungan kategori lama dan baru).
- [ ] **Step 4: Test lama budget alert + harness.** Diharapkan: `save` ≤ 100 ms.
- [ ] **Step 5: Commit** `perf(app): peringatan budget hanya menghitung kategori yang disentuh`.

### Task 3: Profil dan pangkas startup backend (L3, L4)

**Files:** Modify `crates/app/tests/perf.rs` (cetak rincian S2 per panggilan, tanpa budget baru); file sesuai opsi yang dipilih.

- [ ] **Step 1: Tambah rincian S2** ke output harness (median per panggilan). Catat di Hasil.
- [ ] **Step 2: Pilih opsi L3** berdasarkan rincian; tulis `Ruling:` di ledger dengan angka.
- [ ] **Step 3: Test kesetaraan** untuk opsi terpilih: hasil `accounts_overview`, `allocation_overview`, `net_worth_history` identik sebelum/sesudah pada fixture 2.000 transaksi.
- [ ] **Step 4: Implementasi + harness.** Diharapkan: `startup_backend` ≤ 700 ms di kedua varian, atau L4 diterapkan dengan angka `startup_ms` dari user.
- [ ] **Step 5: Commit** `perf(app): startup dasbor tanpa hitung saldo berulang`.

### Task 4: Hasil akhir

- [ ] Isi Hasil di plan ini dan di plan 8a (kolom Akhir), tandai 8a selesai di `docs/plans/2026-10-12-v1-roadmap.md`. Commit `docs: Fase 8a selesai — hasil performa`.

## Hasil

Harness `perf_50k` (fixture standar 50.000 transaksi, median 7 run, mesin dev Fedora), plaintext / sqlcipher ms.

| Skenario | Budget | Sebelum 8a2 | T1 cache | T2 alert | T3 startup (akhir) |
|---|---|---|---|---|---|
| `open` | 150 | 1.7 / 2.9 | 1.7 / 2.6 | 1.7 / 2.6 | 1.8 / 2.6 |
| `startup_backend` | 700 | 873 / 3244 | 728 / 889 | 734 / 896 | **516 / 657** |
| `auto_backup` (latar) | — | 305 / 1098 | 319 / 1034 | 309 / 1024 | 310 / 1035 |
| `save` | 100 | 194 / 490 | 216 / 188 | **93 / 95** | 90 / 90 |
| `list_*` maks | 200 | 148 / 255 | **151 / 151** | 148 / 149 | 149 / 150 |
| `net_worth_12` | 500 | 326 / 1168 | 283 / 281 | 275 / 278 | 132 / 132 |
| `allocation_overview` | 500 | 195 / 1224 | 147 / 147 | 150 / 148 | 109 / 118 |
| `allocation_history_12` | 500 | 328 / 1260 | 279 / 283 | 298 / 283 | 126 / 146 |
| `budget_overview` | 500 | 91 / 216 | 91 / 90 | 87 / 93 | 88 / 93 |

Semua skenario dalam budget di kedua varian; L4 tidak diperlukan.

Rincian S2 per panggilan (Task 3 Step 1), sebelum → sesudah T3: `accounts_overview` 145 → 112, `allocation_overview` 146 → 114, `net_worth_history` 273 → 144, `budget_overview` 88, `cashflow_report` 29, `list_txns_8` 18, sisanya < 3 ms.

- **L3**: tidak memakai (a) memo maupun (b) `account_balance_daily`. Profil menunjukkan `list_accounts` (±140 ms) dipanggil tiga kali, dan dua bagiannya bisa dibuang tanpa state turunan:
  - subquery `reconciled_through` membaca setiap posting akun (55 ms). Migrasi `0010` menambah indeks parsial `idx_posting_reconciled`, sehingga turun ke ~0 ms.
  - `period_points` hanya butuh jenis akun dan flag net worth. Kini memakai `list_account_meta` tanpa menghitung saldo.
- **Belum diukur**: `startup_ms` end-to-end (release build + GUI) oleh user. Diukur ulang di 8d (Windows).
