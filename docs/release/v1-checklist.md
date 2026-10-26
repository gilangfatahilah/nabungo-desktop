# Checklist rilis v1 (Gate 2)

Bagian otomatis (CI `ci.yml` Linux + Windows, `release.yml`: ukuran installer, firewall test) harus hijau dulu. Daftar ini berisi cek yang hanya bisa dilakukan manusia. Centang dan catat temuan di bawah setiap butir.

Artefak: unduh dari run `release.yml` terbaru (Actions → Release → artefak `bundle-Linux`, `bundle-Windows`), atau dari draft release setelah tag di-push.

## 1. Instal di Linux (Fedora, WebKitGTK)

- [x] `.rpm`: `sudo dnf install ./Nabungo-*.rpm` berhasil, Nabungo muncul di menu aplikasi.
- [x] AppImage: `chmod +x` lalu jalankan.
  - 2026-10-26, `Nabungo_0.1.0_amd64.AppImage` (build lokal) dengan `XDG_DATA_HOME` terpisah: jendela tampil, proses WebKit hidup, DB dibuat dan dimigrasi ke versi 10, log tanpa WARN/ERROR.
- [x] Untuk keduanya: onboarding → catat expense, transfer dengan biaya, undo hapus → buka lampiran PDF dan JPG (terbuka di viewer bawaan).

## 2. Instal di Windows 10/11 (WebView2)

- [ ] `*-setup.exe`: instal tanpa admin, Nabungo ada di Start menu, data di `%APPDATA%\nabungo\`.
- [ ] `.msi`: instal dan uninstall bersih.
- [ ] Alur yang sama dengan butir 1, termasuk **buka lampiran PDF/JPG** (perbaikan salinan berekstensi Fase 8d).
- [ ] Offline (R7): buka Resource Monitor → tab Network, filter `nabungo.exe` dan proses `msedgewebview2.exe` milik Nabungo, pakai app ±2 menit → tidak ada koneksi TCP.
- [ ] Teks dan layout di skala font 150% dan DPI Windows 150%.

## 3. Waktu startup dengan 50.000 transaksi (target < 1500 ms)

```bash
cargo run -p nabungo-app --release --example seed_fixture -- <exe_dir>/data
touch <exe_dir>/portable   # di samping executable yang terpasang/hasil build
```

Jalankan app 3×, baca `startup_ms` dari baris `dasbor pertama tampil` di `<exe_dir>/data/logs/`.

- [ ] Fedora: 1881 / 1652 / 1811 ms — **gagal target** (2026-10-26, Fedora 44 Wayland, 8 core, build lokal 0.1.0, portable)
  - Pembanding fixture 100 transaksi: 1512 / 1382 / 1366 ms. Jadi start shell + WebKitGTK sendiri sudah ~1,4 s; 50.000 transaksi menambah ~350 ms.
  - Benchmark `perf`: `startup_backend` 304 ms (plaintext) / 376 ms (sqlcipher), cocok dengan selisih di atas. Query terberat: `net_worth_history` 81 ms, `allocation_overview` 70 ms, `accounts_overview` 70 ms.
- [ ] Windows: ____ / ____ / ____ ms

## 4. Pembaca layar (ditunda dari 8c)

Orca (Fedora) dan NVDA (Windows): onboarding, sheet transaksi (label field, error validasi dibacakan), daftar transaksi (baris dan grup tanggal), budget (progress dan sisa), layar kunci.

- [ ] Orca
- [ ] NVDA

## 5. Cek manual Tauri dengan DB nyata (tertunda dari Fase 3–5)

- [ ] Hapus akun/kategori yang terpakai diblokir dan menawarkan arsip/merge.
- [ ] Transaksi: input, edit, split, transfer dengan biaya, hapus + undo.
- [ ] Budget dan alokasi: salin dari bulan lalu, rollover, donut, dan tabel target vs aktual.

## 6. Gate 1

- [ ] MVP sudah dipakai harian ≥ 1 bulan berdampingan dengan tracker lama (tulis rentang tanggalnya): ____

## 7. Langkah rilis

1. [ ] Bump versi ke `1.0.0` di `src-tauri/tauri.conf.json`, `Cargo.toml` (`[workspace.package]`), dan `package.json`, lalu `cargo check` agar `Cargo.lock` ikut. `node scripts/check-version.mjs v1.0.0` harus mencetak `versi cocok`.
2. [ ] Commit `chore: rilis v1.0.0`, merge ke `main`.
3. [ ] `git tag v1.0.0 && git push origin v1.0.0` → `release.yml` membuat **draft** release.
4. [ ] Tinjau draft: semua artefak ada, `SHA256SUMS` benar, catatan rilis (`docs/release/notes.md` + catatan otomatis) masuk akal.
5. [ ] Publish.
