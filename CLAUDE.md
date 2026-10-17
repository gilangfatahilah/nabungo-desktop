# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Nabungo: aplikasi keuangan pribadi desktop, offline-first, data di satu file SQLite milik user. Spesifikasi lengkap ada di `PRD.md` (bahasa Indonesia) — itu sumber kebenaran untuk requirement (ID seperti `TRX-02`, `CAP-03`), keputusan D1–D12, skema, dan acceptance criteria. Roadmap & plan eksekusi ada di `docs/plans/`. Panduan desain UI (token warna/tipografi, layout, komponen, pola layar) ada di `docs/design/ui-guidelines.md` — setiap perubahan UI wajib mengikutinya.

## Commands

Prasyarat Linux (Fedora): `sudo dnf install gcc webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel librsvg2-devel`. Rust via rustup (`source ~/.cargo/env`). pnpm di-pin ke 10.x lewat `packageManager`.

```bash
pnpm install                         # install dependensi JS (workspace root + ui/)
pnpm dev                             # tauri dev (otomatis jalankan Vite di :5173)
pnpm build                           # bundle installer (AppImage/deb/rpm, NSIS/MSI di Windows)

cargo test --workspace               # semua test Rust
cargo test -p nabungo-core money::   # satu modul / filter nama test
cargo test -p nabungo-storage -- --exact tests::foreign_keys_enabled
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings

pnpm --filter @nabungo/ui test                         # vitest run
pnpm --filter @nabungo/ui exec vitest run src/lib/money.test.ts -t "negative"
pnpm --filter @nabungo/ui typecheck                    # vue-tsc -b
pnpm --filter @nabungo/ui e2e                          # Playwright (UI di browser + browser-ledger); sekali: pnpm --filter @nabungo/ui exec playwright install chromium
pnpm --filter @nabungo/ui dev                          # UI saja di browser (tanpa backend Tauri)
```

Menambah komponen shadcn-vue: jalankan dari `ui/` → `CI=true pnpm dlx shadcn-vue@latest add <komponen> --yes`. Komponen di-copy ke `ui/src/components/ui/` dan boleh diedit. Setelahnya periksa diff: CLI bisa menambah ulang import Google Fonts di `style.css` (revert) dan menulis `import { cn } from 'cn'` plus dependensi `cn` (ganti ke `@/lib/utils`, hapus dependensinya).

Binding TS `ui/src/ipc/bindings.ts` di-generate oleh tauri-specta — otomatis saat `pnpm dev` (debug build) atau lewat `cargo test -p nabungo-desktop export_bindings`. Jangan diedit manual; hasil generate **wajib di-commit** (CI memeriksa tidak ada diff).

Folder data (DB `nabungo.db` + `logs/`): `~/.local/share/nabungo/` (Linux), `%APPDATA%\nabungo\` (Windows). Mode portable: buat file kosong `portable` di samping executable → data di `<exe_dir>/data/` (tidak untuk AppImage).

Error IPC: command mengembalikan `IpcError { code, ... }`; UI menerjemahkan `code` ke kunci i18n `errors.<code>` (lewat `ui/src/lib/errors.ts`, code tak dikenal → `errors.internal`). Code baru wajib ditambahkan ke `errors` di `ui/src/i18n/id.ts` (typecheck gagal bila belum).

CI (`.github/workflows/ci.yml`) menjalankan fmt check, clippy `-D warnings`, test di Linux + Windows, serta typecheck/test/build UI.

## Arsitektur

Cargo workspace + pnpm workspace. Arah dependensi wajib: `ui → src-tauri → nabungo-app → (nabungo-core, nabungo-storage)`, dan `nabungo-storage → nabungo-core`.

- `crates/core` (`nabungo-core`) — domain murni: Money, Account, Txn/Posting, validasi invariant, `period_of(date)`, kalkulasi budget & alokasi. **Tidak boleh** bergantung pada IO (rusqlite, tauri, fs). `#![deny(clippy::float_arithmetic)]`. Nama paket diberi prefix `nabungo-` karena `core` bentrok dengan crate std `core`.
- `crates/storage` (`nabungo-storage`) — rusqlite (bundled), PRAGMA saat open, migrasi `rusqlite_migration` di `crates/storage/migrations/` (versi via `user_version`). Repository yang menulis menerima `&rusqlite::Transaction`, bukan `&Connection`; query baca murni (lookup, saldo) boleh `&Connection` agar bisa dipakai koneksi baca. `insert_txn` hanya menerima `ValidTxn` (dibuat lewat `ValidTxn::new`, yang menjalankan validasi core). Test memakai `open_in_memory()` yang menjalankan migrasi penuh.
- `crates/app` (`nabungo-app`) — use case di atas satu `Ledger` (pemilik koneksi tulis), dipecah per area sebagai blok `impl Ledger` di file masing-masing (`accounts.rs`, `categories.rs`, `payee.rs`, `onboarding.rs`, `settings.rs`); satu use case tulis = satu SQLite transaction. Pelanggaran UNIQUE di storage menjadi `StorageError::Duplicate`, dipetakan ke `AppError::Duplicate(entity)` lewat `AppError::duplicate_as`. `nabungo-app` me-re-export `Uuid`, `Date`, `parse_date`, `format_date` agar `src-tauri` tidak butuh dependensi langsung.
- `src-tauri` (`nabungo-desktop`) — command Tauri yang tipis + DTO serde; tidak ada logika bisnis. Command baru didaftarkan di `collect_commands!` dalam `specta_builder()` di `src-tauri/src/lib.rs` (tauri-specta).
- `ui/` (`@nabungo/ui`) — Vue 3 `<script setup>` + TS + Vite, shadcn-vue (Reka UI + Tailwind v4), Pinia, vue-router (hash history), vue-i18n (default `id`, fallback `en`; skema pesan diturunkan dari `src/i18n/id.ts`). Semua panggilan backend lewat `ui/src/ipc/` — jangan `invoke()` langsung dari komponen. Di browser/jsdom (tanpa Tauri) `ipc` memakai fallback in-memory `ipc/browser-ledger.ts`; aturan turunannya (mis. net worth) harus tetap sama dengan Rust. Store Pinia memuat ulang dari backend setelah setiap aksi tulis — jangan hitung saldo di UI. Test komponen memakai jsdom + `src/test-setup.ts` (stub API layout untuk Reka). jsdom tidak mengukur layout: `TxnList` punya prop `virtual=false` (khusus test) untuk merender semua baris. Parser nominal mode browser (`ipc/browser-amount.ts`) adalah port `crates/core/src/amount.rs` — ubah keduanya bersamaan.

### Model data (wajib dipahami sebelum menyentuh transaksi)

- Sumber kebenaran tunggal adalah tabel `posting`. Saldo, net worth, realisasi budget, dan porsi alokasi **selalu diturunkan** dari postings — tidak pernah disimpan sebagai kolom.
- Transaksi = header `txn` + ≥1 `posting` (double-entry ringan, D3). Transfer = 2 posting tanpa kategori yang berjumlah 0 (+ opsional posting biaya dengan kategori expense). Transfer tidak pernah dihitung sebagai income/expense atau masuk budget.
- Invariant (PRD §6) ditegakkan di layer domain Rust, bukan trigger SQL dan bukan hanya di UI.
- Form (income/expense/transfer, split, biaya) diubah menjadi postings oleh `form_postings` di `nabungo-core`; UI hanya mengirim `TxnFormDto`, tidak pernah menyusun postings sendiri.
- Uang: `i64` satuan minor (IDR decimals = 0). Di UI pun angka uang diperlakukan sebagai integer; format pakai `ui/src/lib/money.ts`. Persentase alokasi dalam basis point (`target_bp`, `share_bp`; 10000 = 100%).
- Tanggal transaksi `TEXT` `YYYY-MM-DD` tanpa jam; timestamp audit ISO-8601 UTC. ID UUIDv7 sebagai TEXT. Soft delete via `deleted_at`.
- v1 hanya IDR (D4), tapi kolom `currency` tetap diisi agar multi-currency v2 tidak butuh migrasi besar.

### Migrasi

Migrasi yang sudah dirilis tidak boleh diubah — tambahkan file baru dan daftarkan di `migrations()` di `crates/storage/src/lib.rs`.

## Batasan produk yang memengaruhi kode

- Offline total: tidak ada network call runtime. CSP di `src-tauri/tauri.conf.json` memblokir origin eksternal — jangan tambah aset remote (font, CDN, analytics). Font Inter dibundel via `@fontsource-variable/inter` (shadcn init default menambahkan import Google Fonts; sudah dihapus, jangan dikembalikan).
- Tidak ada telemetry; log hanya lokal.
- Hapus transaksi = soft delete + undo; akun/kategori yang terpakai di-arsip/merge, tidak dihapus.

## Catatan lingkungan

- pnpm memakai policy `minimumReleaseAge`: paket yang baru dirilis (< ~1 hari) ditolak. Jika install gagal karena itu, pin ke versi sebelumnya (contoh: vitest di-pin 5.0.3).
