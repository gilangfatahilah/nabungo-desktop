# Revamp layar Laporan

**Goal:** Layar Laporan sudah dipakai dengan data nyata dan terasa kaku: tiga section datar, rentang tetap 12 periode, tanpa pembanding atau drill-down, dan isinya tumpang tindih dengan kartu Tren di Dasbor. Revamp menjadikannya layar analisis berbasis kartu (pola §7.11/§7.12 `docs/design/design.md`) yang menjawab "ke mana uang saya, dibanding biasanya?".

**Keputusan user:**
- Revamp tampilan **dan** tambah insight. Backend boleh diubah.
- Insight yang ditambah:
  - banding per kategori dengan periode lalu;
  - tren per kategori;
  - drill-down (sub-kategori → Transaksi);
  - rasio tabungan dan rata-rata.
- Dua kontrol waktu:
  - rentang chart 3/6/12/24 periode;
  - periode fokus `‹ Okt 2026 ›` untuk KPI dan kategori.

## Layout

```
PageHeader: Periode 25 Sep – 24 Okt            [‹ Okt 2026 ›]  [1 tahun terakhir ▾]
[Pemasukan ↗] [Pengeluaran ↘] [Sisa ↗] [Rasio tabungan]          ← section card §7.11
┌ Pemasukan vs pengeluaran ─ col 8 ┐ ┌ Perubahan terbesar ─ col 4 ┐
┌ Kategori Okt 2026 ──────── col 7 ┐ ┌ Tren kategori ────── col 5 ┐
┌ Kekayaan bersih ───────────────────────────────────────── full ┐
```

## Backend

Use case di `crates/app/src/reports.rs`, keduanya tanpa migrasi:
- `category_comparison(period, kind)`: root dengan `amount` + `previous` (periode sebelumnya) + child yang ≠ 0.
  - Posting langsung di root tampil sebagai child ber-id root bila root punya child lain.
  - Root masuk bila `amount > 0 || previous > 0`.
- `category_trend(end, count, category_id)`: total root + child per periode. Id yang tidak dikenal → `not_found`.

Command `category_comparison` / `category_trend` ditambahkan di `src-tauri`. Logikanya di-port ke `ipc/browser-ledger.ts` dengan test paritas.

## Frontend

- **Store `stores/reports.ts`.** `load()` tetap 12 periode untuk Dasbor. `loadReport()` memuat `range + 1` periode arus kas (titik ekstra = pembanding periode pertama) dan `range` titik kekayaan bersih, lalu banding dan tren untuk periode fokus.
  - Fokus yang keluar dari jendela kembali ke periode berjalan.
  - Rentang disimpan di localStorage (`nabungo.reports.range`).
- **`lib/reportStats.ts`.** Hanya membagi angka backend:
  - `savingsRate`;
  - `perDay` (periode berjalan dibagi hari yang sudah lewat);
  - `average`;
  - `biggestChanges`.
- **Kartu di `components/reports/`.** Tabel angka tetap ada di bawah setiap chart, terlipat (`TableToggle`).
  - `ReportsKpis`
  - `CashflowReportCard`
  - `BiggestChangesCard`
  - `CategoryReportCard`: donut top 5 + Lainnya, baris dengan porsi + badge tren, child bisa dibuka. Panah membuka Transaksi terfilter kategori + rentang periode.
  - `CategoryTrendCard`
  - `NetWorthReportCard`

## Verifikasi

- `cargo test -p nabungo-app reports::`
- `pnpm --filter @nabungo/ui typecheck` / `test`
- e2e `ui/e2e/reports.spec.ts`
- `a11y.spec.ts` (tabel terbuka + sub-kategori, light/dark, font 150%)
