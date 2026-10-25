# Fase 8d: Offline & Rilis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Membuktikan "0 request keluar" secara otomatis, menyalakan lagi CI Windows, membuat installer Linux (AppImage/deb/rpm) dan Windows (NSIS/MSI) dari CI di bawah 15 MB, serta workflow yang menyiapkan draft GitHub Release. Rilis v1 (Gate 2) tinggal: bump versi, checklist manual, lalu push tag.

**Architecture:** Bukti offline punya tiga lapis. (1) Test konfigurasi Rust memastikan CSP tidak membuka origin eksternal dan tidak ada updater. (2) E2E Playwright merekam setiap request UI di semua layar. (3) Firewall test menjalankan binary release sungguhan di network namespace Linux dengan interface dummy + `tcpdump`, lalu menghitung paket IP (harus 0). Packaging memakai `pnpm tauri build --bundles …` biasa di `.github/workflows/release.yml` (tanpa `tauri-action`). Pengecekan ukuran dan versi dilakukan skrip Node kecil di `scripts/` beserta `node --test`. Lampiran Windows dibuka lewat salinan berekstensi di `cache/open/` dalam folder data.

**Tech Stack:** Rust (`nabungo-app`, `nabungo-desktop`), Tauri 2 bundler, GitHub Actions (`ubuntu-22.04`, `windows-latest`), Node 24 (`node:test`), Playwright, bash + iproute2 + tcpdump + Xvfb.

**Spec:** `PRD.md` §8 (Offline "0 request keluar (diverifikasi dengan firewall test)", Ukuran "Installer < 15 MB (Tauri)", Update "Updater Tauri **mati secara default**", Packaging "Linux: AppImage, `.rpm`, `.deb`; Windows: `.msi`/NSIS — Build reproducible di CI"), D9 (Linux + Windows di v1, macOS ditunda), §7 Strategi testing ("CI: … build artefak Linux & Windows"); roadmap `docs/plans/2026-10-12-v1-roadmap.md` baris 8d dan catatan Windows lampiran.

## Kondisi saat ini (hasil riset)

1. **CSP sudah self-only**: `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data: blob:; …`. Tidak ada `tauri-plugin-updater`. Capability hanya `core:default`. Belum ada test yang menjaga keadaan ini.
2. **`reqwest`/`hyper` ada di `Cargo.lock`** sebagai dependensi `tauri` itu sendiri, bukan dipakai kode kita. Firewall test (lapis 3) yang membuktikan tidak ada pemakaian saat runtime.
3. **CI Windows dimatikan** di commit `9fc3d1a` (matrix `rust` hanya `ubuntu-24.04`). Manifest Windows untuk test binary sudah ditangani `src-tauri/build.rs`.
4. **Bundle**: `tauri.conf.json` `targets: ["appimage","deb","rpm","nsis","msi"]`, tanpa metadata, tanpa `depends`. Belum ada `[profile.release]` di workspace. Belum ada workflow rilis. Versi `0.1.0` tercatat di `tauri.conf.json`, `Cargo.toml` (`workspace.package`), dan `package.json`.
5. **Lampiran**: `Ledger::attachment_path` mengembalikan `attachments/<sha256>` tanpa ekstensi, lalu `open_attachment` memanggil `opener().open_path`. Di Windows ShellExecute jadi tidak tahu aplikasi pembukanya. `ext_for(mime)` sudah ada di `crates/app/src/attachments.rs`.
6. **Penanda startup**: `tracing::info!(startup_ms, "dasbor pertama tampil")` ditulis ke `<data>/logs/nabungo*`. Mode portable (`portable` di samping exe → `<exe_dir>/data`) memungkinkan app berjalan di folder sementara tanpa mengubah `HOME`.

## Keputusan

| ID | Keputusan | Alasan |
|---|---|---|
| R1 | Lampiran dibuka dari **salinan** `<data>/cache/open/<sha256>.<ext>` (ext dari `ext_for(mime)`) di semua OS, bukan hardlink. Bila salinan sudah ada dengan ukuran sama, salinan itu dipakai ulang (Windows mengunci file yang sedang dibuka viewer). `cache/open/` dikosongkan best-effort saat startup. | Hardlink membuat viewer yang menyimpan perubahan ikut mengubah blob, sehingga sha256 tidak cocok lagi. Struk berukuran kecil. Folder di dalam data dir agar mode portable tetap satu folder. |
| R2 | Updater **tidak dipasang sama sekali** di v1 (tidak ada opsi opt-in). Update manual dari GitHub Releases. Dijaga test (R3). | PRD: mati secara default. Opt-in tidak wajib → YAGNI. |
| R3 | Test konfigurasi `src-tauri/tests/offline_config.rs`: setiap token CSP yang berisi `://` harus salah satu dari `http://ipc.localhost` atau `http://asset.localhost`. Token `*`, `http:`, `https:`, `ws:`, `wss:` dilarang. Tidak ada `plugins.updater` maupun `bundle.createUpdaterArtifacts: true`. `Cargo.toml` desktop tidak menyebut `tauri-plugin-updater`/`tauri-plugin-http`. Tidak ada permission capability berawalan `http:`/`updater:`. | Penjaga murah yang jalan di setiap PR. |
| R4 | E2E `ui/e2e/offline.spec.ts` merekam `page.on('request')` dan menolak URL `http(s)`/`ws(s)` yang origin-nya bukan `http://localhost:5173`, di semua route + sheet transaksi + command palette + dialog backup. | Menangkap font/CDN/gambar remote di UI pada setiap PR. |
| R5 | Firewall test `scripts/firewall-test.sh <app-bin> <seed-bin>` (Linux, butuh sudo): netns `nbg-fw` berisi `lo` + dummy `fw0` (10.255.0.2/24, default route via 10.255.0.1), `/etc/netns/nbg-fw/resolv.conf` = `nameserver 10.255.0.1` (DNS ikut tertangkap). Langkahnya: `tcpdump -i any -n 'ip or ip6'`; app dijalankan sebagai user biasa di folder portable berisi fixture 2.000 transaksi lewat `xvfb-run -a dbus-run-session`; tunggu log `dasbor pertama tampil` (maks 60 s), tunggu lagi 15 s (backup otomatis dan request malas), lalu matikan. **Lulus = 0 paket.** Exit code: 0 lulus, 1 ada paket (dicetak), 2 app tidak sampai dasbor / setup gagal. `FW_SELFTEST=1` mengganti app dengan `getent hosts example.com`; hasil yang diharapkan exit 1, membuktikan tangkapan tidak kosong secara palsu. | Satu-satunya lapis yang menguji binary + WebKitGTK sungguhan. Paket di `lo` juga dihitung karena app produksi memakai protokol `tauri://`, bukan localhost. |
| R6 | Firewall test berjalan di `ubuntu-22.04`. Bila sandbox WebKit (bubblewrap) gagal di runner, set `WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1` **hanya di skrip test**, dengan komentar alasan. | Sandbox mengatur proses, bukan jaringan; yang diukur tetap trafik jaringan. |
| R7 | Windows tidak punya firewall test otomatis. Penggantinya cek manual di checklist rilis (Resource Monitor → Network: tidak ada koneksi TCP `nabungo.exe`), ditambah lapis R3/R4 yang tidak bergantung OS. | Netns tidak tersedia di Windows. Kode Rust/UI sama di kedua OS. |
| R8 | Target `< 15 MB` = **15 000 000 byte** per file `.deb`, `.rpm`, `.msi`, `*-setup.exe`. **AppImage dikecualikan** (hanya dilaporkan) karena membundel WebKitGTK sendiri (biasanya > 70 MB). | Angka PRD "(Tauri)" mengacu ke installer yang memakai webview sistem. |
| R9 | `[profile.release]` workspace: `lto = true`, `codegen-units = 1`, `strip = true`. `opt-level` dan `panic` tetap default. Benchmark perf 8a dijalankan ulang setelahnya. Bila `.deb`/`.rpm`/`.msi`/`setup.exe` masih ≥ 15 MB, **berhenti dan laporkan ukuran ke user** sebelum mengubah `opt-level`. | Menurunkan `opt-level` bisa membatalkan hasil perf 8a. Itu keputusan produk. |
| R10 | Metadata bundle: `publisher: "Nabungo"`, `category: "Finance"`, `shortDescription: "Keuangan pribadi offline"`, `longDescription: "Catat transaksi, saldo, budget, dan alokasi modal. Semua data tersimpan di satu file milik Anda, tanpa internet."`. `linux.deb.depends: ["libssl3"]`, `linux.rpm.depends: ["openssl-libs"]` (SQLCipher menautkan libcrypto sistem). NSIS: `installMode: "currentUser"`, `languages: ["Indonesian","English"]`. MSI memakai bahasa default. WebView2: mode default (`downloadBootstrapper`, butuh internet saat **instal** saja). | Data di `%APPDATA%` per user, jadi tidak perlu admin. Win10/11 umumnya sudah punya WebView2. |
| R11 | Installer Windows **tidak ditandatangani** di v1. Catatan rilis menjelaskan peringatan SmartScreen. | Belum ada sertifikat. Bisa ditambah tanpa mengubah alur build. |
| R12 | Workflow rilis: `.github/workflows/release.yml` dengan trigger `push: tags: ['v*']`, `workflow_dispatch`, dan `pull_request` dengan `paths` (`.github/workflows/release.yml`, `scripts/**`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `Cargo.toml`, `Cargo.lock`). Job: `version` → `bundle` (matrix `ubuntu-22.04` + `windows-latest`) → `firewall` → `release` (hanya pada tag; `gh release create --draft` + `SHA256SUMS`). Publish draft dilakukan manual oleh user. | Trigger `pull_request` membuat workflow ini teruji di PR fase ini (`workflow_dispatch` baru aktif setelah merge) dan setiap ada perubahan dependensi. `ubuntu-22.04` memberi glibc lebih tua untuk deb/AppImage. |
| R13 | `scripts/check-version.mjs [tag]`: versi `tauri.conf.json`, `Cargo.toml` `[workspace.package]`, dan `package.json` harus sama. Bila tag diberikan, tag harus `v<versi>`. Bump ke `1.0.0` **tidak** dilakukan di fase ini, tapi di langkah rilis (checklist). | Mencegah tag dan versi binary berbeda. |
| R14 | CI Windows: `windows-latest` ditambahkan kembali ke matrix job `rust`. Test yang gagal diperbaiki di kode. `#[cfg_attr(windows, ignore)]` hanya boleh untuk perilaku yang memang khas OS, dengan komentar alasan. | Exit roadmap: CI Windows aktif. |

## Global Constraints

- Offline total: tidak ada network call runtime, tidak ada aset remote. Skrip/CI boleh memakai jaringan saat **build**.
- Tidak ada telemetry. Skrip tidak mengirim data ke mana pun selain artefak GitHub Actions.
- CLAUDE.md: `bindings.ts` tidak berubah di fase ini (tidak ada command baru). Bila berubah, regenerate dan commit.
- Commit memakai pesan Indonesia bergaya `tipe(scope): …` seperti riwayat, diakhiri baris `Co-Authored-By` dari system reminder.
- Push tag `v*` dan publish GitHub Release **tidak** dilakukan agen. Itu langkah user di checklist.

## Review Focus

1. **Lampiran yang sedang terbuka di viewer Windows lalu dibuka lagi**: tidak boleh error; salinan dipakai ulang. Dikunci test `open_copy_reused_when_present` (salinan dibuat read-only di antara dua panggilan) di Task 1.
2. **Firewall test lulus palsu** (tcpdump tidak menangkap apa pun, misalnya interface salah atau DNS lewat `lo`): self-test `FW_SELFTEST=1` wajib exit 1 sebelum test sungguhan dijalankan di CI (Task 4).
3. **App crash/hang di netns sehingga "0 paket" karena app tidak pernah jalan**: wajib ada log `dasbor pertama tampil`, kalau tidak exit 2 (Task 4).
4. **Paket `.deb` terpasang tapi gagal start karena `libcrypto.so.3` hilang**: `depends` libssl3/openssl-libs dicek di Task 3 (`dpkg-deb -f … Depends` mengandung `libssl3`). Instal manual di checklist.
5. **Tag `v1.0.0` di-push saat versi file masih `0.1.0`**: job `version` gagal sebelum build. Dikunci test `check-version` (Task 5).

---

### Task 1: Lampiran dibuka lewat salinan berekstensi (R1)

**Files:**
- Modify: `crates/app/src/attachments.rs` (ganti `attachment_path` → `attachment_open_path`, tambah `clear_open_cache`, test)
- Modify: `crates/app/src/lib.rs` (re-export `clear_open_cache`)
- Modify: `src-tauri/src/paths.rs` (`open_cache_path` + test), `src-tauri/src/commands.rs:787-800`, `src-tauri/src/lib.rs` (panggil `clear_open_cache` setelah `resolve_data_dir`, ±baris 133)

**Interfaces:**
- Produces: `impl Ledger { pub fn attachment_open_path(&self, id: Uuid, dir: &Path, open_dir: &Path) -> Result<PathBuf, AppError> }`. Hasilnya `open_dir/<sha>.<ext_for(mime)>`. Membuat `open_dir` bila belum ada. Mengembalikan `AttachmentMissing` bila blob hilang atau sha tidak valid, `NotFound("attachment")` bila id tidak ada, dan `Io` bila salin gagal tanpa salinan yang bisa dipakai ulang.
- Produces: `pub fn clear_open_cache(open_dir: &Path)` (best-effort; error di-`tracing::warn!`, folder tidak ada = no-op).
- Produces: `DataDir::open_cache_path(&self) -> PathBuf` = `self.path.join("cache").join("open")`.

- [ ] **Step 1: Tulis test yang gagal** di modul test `attachments.rs`, memakai fixture `fx()` yang sudah ada. Test `missing_file_is_flagged_and_open_fails` dan `tampered_sha_never_escapes_attachments_dir` diubah memanggil `attachment_open_path(row.id, f.dir.path(), &open)` (dengan `open = tempdir`) dan tetap mengharapkan `AttachmentMissing`. Test tampered juga memastikan `open` tetap kosong. Test baru:

```rust
#[test]
fn open_copy_has_extension_and_same_bytes() {
    // PDF → open/<sha>.pdf, isi identik, blob asli tetap ada
    let p = f.file("s.pdf", PDF);
    let row = f.add(f.txn, std::slice::from_ref(&p)).unwrap().remove(0);
    let sha = sha256_file(&p).unwrap();
    let got = f.kit.ledger.attachment_open_path(row.id, f.dir.path(), open.path()).unwrap();
    assert_eq!(got, open.path().join(format!("{sha}.pdf")));
    assert_eq!(fs::read(&got).unwrap(), PDF);
    assert!(f.dir.path().join(&sha).is_file());
}

#[test]
fn open_copy_reused_when_present() {
    // panggilan pertama membuat salinan PNG; salinan dibuat read-only; panggilan kedua Ok, path sama
}

#[test]
fn clear_open_cache_removes_files_and_ignores_missing_dir() {
    // file di open/ hilang setelah clear; clear pada path yang tidak ada tidak panic
}
```

Di `paths.rs`: test `open_cache_under_data` memastikan `d.open_cache_path() == d.path.join("cache").join("open")`.

- [ ] **Step 2: Jalankan dan pastikan gagal**

Run: `cargo test -p nabungo-app attachments:: && cargo test -p nabungo-desktop paths::`
Expected: FAIL kompilasi (`attachment_open_path`, `clear_open_cache`, `open_cache_path` belum ada)

- [ ] **Step 3: Implementasi.** Lookup blob memakai `blob_path` yang sudah ada (tetap `is_file()`). Bila tujuan sudah ada dan `metadata().len()` sama dengan blob, pakai ulang. Selain itu `fs::copy`. `open_attachment` di `commands.rs` memakai `attachment_open_path(id, &attachments_path(), &state.data_dir.open_cache_path())`. `run()` memanggil `nabungo_app::clear_open_cache(&data_dir.open_cache_path())` tepat setelah `resolve_data_dir`.

- [ ] **Step 4: Jalankan dan pastikan lulus**

Run: `cargo test -p nabungo-app attachments:: && cargo test -p nabungo-desktop && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, clippy bersih

- [ ] **Step 5: Commit** — `fix(app): lampiran dibuka lewat salinan berekstensi`

---

### Task 2: Penjaga offline — konfigurasi Rust dan e2e request (R2, R3, R4)

**Files:**
- Create: `src-tauri/tests/offline_config.rs`
- Create: `ui/e2e/offline.spec.ts`

**Interfaces:**
- Consumes: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/capabilities/default.json` (dibaca lewat `include_str!` relatif `CARGO_MANIFEST_DIR`), `serde_json` (sudah dependensi).

- [ ] **Step 1: Tulis test Rust**

```rust
#[test] fn csp_has_no_external_origins()   // token berisi "://" ∈ {"http://ipc.localhost","http://asset.localhost"};
                                           // tidak ada token "*", "http:", "https:", "ws:", "wss:"
#[test] fn no_updater_configured()         // conf["plugins"]["updater"] null; conf["bundle"]["createUpdaterArtifacts"] bukan true
#[test] fn no_network_plugins()            // Cargo.toml desktop tidak berisi "tauri-plugin-updater" / "tauri-plugin-http"
#[test] fn capabilities_have_no_network_permissions() // tidak ada permission berawalan "http:" / "updater:"
```

Uji tiap fungsi pemeriksa dengan input buruk (mis. CSP berisi `https://fonts.googleapis.com`). Caranya: fungsi `fn csp_violations(csp: &str) -> Vec<String>` diuji langsung dengan string sintetis selain konfigurasi asli, supaya test tidak lulus secara vakum.

- [ ] **Step 2: Jalankan** — `cargo test -p nabungo-desktop --test offline_config`. Expected: PASS untuk konfigurasi sekarang, dan test input sintetis membuktikan pemeriksa menolak origin eksternal.

- [ ] **Step 3: Tulis `ui/e2e/offline.spec.ts`**: satu test `tidak ada request ke origin lain di semua layar`. Helper `onboard` mengikuti pola lokal di `a11y.spec.ts` (setiap spec punya helper sendiri). Sebelum `page.goto`, pasang listener:

```ts
const external: string[] = []
page.on('request', (r) => {
  const u = new URL(r.url())
  if (['http:', 'https:', 'ws:', 'wss:'].includes(u.protocol) && u.origin !== 'http://localhost:5173') external.push(r.url())
})
```

Langkah: onboarding → catat satu transaksi lewat `N` → kunjungi `ROUTES` yang sama dengan `a11y.spec.ts` → buka command palette `Control+K` → buka `#/settings` bagian backup. Di akhir `expect(external).toEqual([])`.

- [ ] **Step 4: Jalankan** — `pnpm --filter @nabungo/ui exec playwright test e2e/offline.spec.ts`. Expected: PASS. Untuk memastikan test bisa gagal, tambahkan sementara `<img src="https://example.com/x.png">` di `App.vue`, jalankan (FAIL), lalu kembalikan.

- [ ] **Step 5: Commit** — `test: penjaga offline untuk CSP, updater, dan request UI`

---

### Task 3: CI Windows dan konfigurasi bundle (R8, R9, R10, R14)

**Files:**
- Modify: `.github/workflows/ci.yml` (matrix `os: [ubuntu-24.04, windows-latest]`, hapus komentar penundaan; tambah step `node --test scripts/` di job `ui`)
- Modify: `Cargo.toml` (`[profile.release]` R9), `src-tauri/tauri.conf.json` (`bundle` R10)
- Create: `scripts/check-bundle-size.mjs`, `scripts/check-bundle-size.test.mjs`

**Interfaces:**
- Produces: `export function checkSizes(files: {name: string, bytes: number}[]): {rows: {name, bytes, limit: number|null, ok: boolean}[], ok: boolean}`. Batasnya `LIMIT = 15_000_000` untuk nama berakhiran `.deb`, `.rpm`, `.msi`, `-setup.exe`. `.AppImage` dicatat dengan `limit: null`. Nama lain diabaikan. `ok` false bila tidak ada satu pun installer. CLI: `node scripts/check-bundle-size.mjs <dir>` menelusuri rekursif, mencetak tabel (MB, 2 desimal), dan exit 1 bila tidak ok.

- [ ] **Step 1: Tulis `check-bundle-size.test.mjs`** (`node:test` + `assert`): `14_999_999` byte `.deb` → ok, `15_000_000` byte `.msi` → gagal, `200_000_000` byte `.AppImage` saja → `ok: false` (tidak ada installer berbatas), `.AppImage` besar + `.deb` kecil → ok, `Nabungo_0.1.0_x64-setup.exe` dikenali.
- [ ] **Step 2: Jalankan** `node --test scripts/` → FAIL (modul belum ada). **Step 3:** implementasi. **Step 4:** `node --test scripts/` → PASS.
- [ ] **Step 5: Profil release + metadata bundle**, lalu build lokal: `pnpm tauri build --bundles deb,rpm,appimage` (dengan `APPIMAGE_EXTRACT_AND_RUN=1` bila FUSE tidak ada) dan `node scripts/check-bundle-size.mjs target/release/bundle`. Expected: deb/rpm < 15 MB; ukuran semua artefak dicatat untuk roadmap. Bila ≥ 15 MB: berhenti dan laporkan (R9). Periksa `dpkg-deb -f target/release/bundle/deb/*.deb Depends` mengandung `libssl3`, dan `rpm -qpR target/release/bundle/rpm/*.rpm` mengandung `openssl-libs`.
- [ ] **Step 6: Perf ulang** — `cargo test -p nabungo-app --release --test perf -- --ignored --nocapture`. Expected: S1–S6 masih dalam budget (bandingkan dengan angka 8a di plan `2026-10-24-fase-8a2-performa-lanjutan.md`).
- [ ] **Step 7: Commit** — `build: profil release dan metadata bundle` lalu `ci: aktifkan kembali test Windows`.
- [ ] **Step 8: Push dan pantau CI Windows** — `git push -u origin fase-8d-rilis`, lalu `gh run watch`. Perbaiki setiap test yang gagal di Windows sesuai R14, satu commit `fix(…): …` per akar masalah. Sebelum memperbaiki, pakai superpowers:systematic-debugging. Expected akhir: job `rust (windows-latest)` hijau.

---

### Task 4: Firewall test (R5, R6)

**Files:**
- Create: `scripts/firewall-test.sh` (executable, `set -euo pipefail`, `trap` membersihkan netns, `/etc/netns/nbg-fw`, dan proses)

**Interfaces:**
- Consumes: binary app release (`target/release/nabungo-desktop`), binary `target/release/examples/seed_fixture` (opsi `--txns`, lihat `crates/app/examples/seed_fixture.rs`), log `dasbor pertama tampil` di `<data>/logs/`.
- Produces: `scripts/firewall-test.sh <app-bin> <seed-bin>`, exit 0/1/2 sesuai R5. Env `FW_SELFTEST=1`. Prasyarat: `sudo`, `ip`, `tcpdump`, `xvfb-run`, `dbus-run-session`.

- [ ] **Step 1: Tulis skrip** sesuai R5. Urutan: work dir `mktemp -d` → salin app ke `$WORK/nabungo`, `touch $WORK/portable`, seed `$WORK/data --txns 2000` → setup netns + resolv.conf → `tcpdump -i any -n -U -w $WORK/fw.pcap 'ip or ip6'` (tunggu 1 s agar tangkapan aktif) → `sudo ip netns exec nbg-fw sudo -u "$USER" env HOME="$HOME" GDK_BACKEND=x11 xvfb-run -a dbus-run-session -- "$WORK/nabungo"` di latar → poll `grep -rq 'dasbor pertama tampil' "$WORK/data/logs"` setiap 1 s sampai 60 s (exit 2 + cetak log bila gagal) → `sleep 15` → kill app, hentikan tcpdump → `tcpdump -r … -n` dan hitung baris; bila > 0, cetak paket dan exit 1, selain itu cetak `firewall test: 0 paket` dan exit 0.
- [ ] **Step 2: Self-test lokal** (Fedora: `sudo dnf install tcpdump xorg-x11-server-Xvfb`, lalu `pnpm tauri build --no-bundle` dan `cargo build -p nabungo-app --release --example seed_fixture`):
  Run: `FW_SELFTEST=1 scripts/firewall-test.sh target/release/nabungo-desktop target/release/examples/seed_fixture; echo $?`
  Expected: paket DNS ke 10.255.0.1 tercetak, exit `1`
- [ ] **Step 3: Test sungguhan lokal**
  Run: `scripts/firewall-test.sh target/release/nabungo-desktop target/release/examples/seed_fixture; echo $?`
  Expected: `firewall test: 0 paket`, exit `0`. Bila ada paket: identifikasi sumbernya (WebKit, Tauri, atau kode kita) dan perbaiki di sumber. Jangan menambah allowlist tanpa keputusan user.
- [ ] **Step 4: Commit** — `test: firewall test binary release di network namespace`

---

### Task 5: Workflow rilis dan cek versi (R11, R12, R13)

**Files:**
- Create: `scripts/check-version.mjs`, `scripts/check-version.test.mjs`
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `scripts/check-bundle-size.mjs` (Task 3), `scripts/firewall-test.sh` (Task 4).
- Produces: `export function checkVersion(tag: string | null, files: {tauriConf: string, cargoToml: string, packageJson: string}): string[]` (daftar pesan error, kosong = ok). Versi Cargo dibaca dari blok `[workspace.package]`. CLI: `node scripts/check-version.mjs [tag]` dengan exit 1 bila ada error.

- [ ] **Step 1: Tulis test** — semua `0.1.0` + tag `v0.1.0` → `[]`. Tag `null` + semua sama → `[]`. Tag `v1.0.0` sementara file `0.1.0` → satu error yang menyebut `v1.0.0` dan `0.1.0`. Tag `0.1.0` tanpa `v` → error. `package.json` `0.2.0` lainnya `0.1.0` → error yang menyebut `package.json`.
- [ ] **Step 2:** `node --test scripts/` → FAIL. **Step 3:** implementasi. **Step 4:** `node --test scripts/` → PASS dan `node scripts/check-version.mjs v0.1.0; echo $?` → `0`.
- [ ] **Step 5: Tulis `release.yml`** (R12):
  - `version` (ubuntu-22.04): `node scripts/check-version.mjs "${{ github.ref_type == 'tag' && github.ref_name || '' }}"`.
  - `bundle` (matrix ubuntu-22.04 / windows-latest, `fail-fast: false`): setup sama dengan `ci.yml` (deps apt + `patchelf`, Rust stable, rust-cache, pnpm, Node 24), `pnpm install --frozen-lockfile`. Build: `pnpm tauri build --bundles appimage,deb,rpm` (Linux, env `APPIMAGE_EXTRACT_AND_RUN=1`) / `pnpm tauri build --bundles nsis,msi` (Windows). Lalu `node scripts/check-bundle-size.mjs target/release/bundle`. Upload artefak `bundle-${{ runner.os }}` (isi `target/release/bundle/**` dengan pola `*.deb`, `*.rpm`, `*.AppImage`, `*.msi`, `*-setup.exe`). Khusus Linux: `cargo build -p nabungo-app --release --example seed_fixture` dan upload `fw-bins` (`target/release/nabungo-desktop`, `target/release/examples/seed_fixture`).
  - `firewall` (needs bundle, ubuntu-22.04): apt `tcpdump xvfb dbus-x11 libwebkit2gtk-4.1-0 libssl3`, download `fw-bins`, `chmod +x`. Jalankan `FW_SELFTEST=1 scripts/firewall-test.sh …` dengan ekspektasi exit 1 (`if [ $? -ne 1 ]; then exit 1; fi`), lalu `scripts/firewall-test.sh …` dengan ekspektasi exit 0.
  - `release` (needs version+bundle+firewall, `if: github.ref_type == 'tag'`, `permissions: contents: write`): download `bundle-*`, `sha256sum * > SHA256SUMS`, `gh release create "$GITHUB_REF_NAME" --draft --title "Nabungo ${GITHUB_REF_NAME#v}" --notes-file docs/release/notes.md --generate-notes <files> SHA256SUMS`.
- [ ] **Step 6: Commit dan push**, lalu pantau run `release.yml` dari trigger `pull_request`: `gh run list --workflow release.yml --branch fase-8d-rilis` → `gh run watch <id>`. Expected: `version`, `bundle (2 OS)`, `firewall` hijau, `release` dilewati. Catat ukuran setiap artefak dari log `check-bundle-size`. Bila sandbox WebKit gagal di runner, terapkan R6. Commit: `ci: workflow rilis dengan bundle, ukuran, dan firewall test`.

---

### Task 6: Dokumentasi dan checklist rilis

**Files:**
- Create: `docs/release/notes.md` (template catatan rilis: tautan PRD fitur singkat, "installer Windows belum ditandatangani — SmartScreen: More info → Run anyway", "AppImage membundel WebKitGTK sehingga lebih besar", cara verifikasi `SHA256SUMS`, lokasi folder data dan backup)
- Create: `docs/release/v1-checklist.md`
- Modify: `CLAUDE.md` (Commands: `pnpm tauri build --bundles …`, `node --test scripts/`, firewall test + prasyaratnya. Bagian CI: Windows aktif kembali, `release.yml`), `docs/plans/2026-10-12-v1-roadmap.md` (baris 8d: **Selesai** + ukuran artefak + keputusan R1/R8/R11)

- [ ] **Step 1: Tulis `v1-checklist.md`** sebagai checklist centang untuk user:
  1. Instal `.rpm` (Fedora) dan AppImage: start, onboarding, catat transaksi, buka lampiran PDF/JPG.
  2. Instal NSIS dan MSI di Windows 10/11: hal yang sama + buka lampiran (R1). Resource Monitor → Network: tidak ada koneksi `nabungo.exe` (R7).
  3. `startup_ms` dengan fixture 50.000 (`seed_fixture` + portable) di Fedora dan Windows: baca log, target < 1500.
  4. Pembaca layar: Orca (Fedora) dan NVDA (Windows) di onboarding, sheet transaksi, daftar transaksi, budget, dan layar kunci (dari 8c).
  5. Cek manual Tauri dengan DB nyata yang tertunda dari Fase 3–5 (hapus aman, transaksi/undo, budget/alokasi).
  6. Status Gate 1 (pemakaian harian ≥ 1 bulan).
  7. Langkah rilis: bump `1.0.0` di tiga file (R13) → commit → `git tag v1.0.0` → `git push origin v1.0.0` → tinjau draft release dan artefaknya → Publish.
- [ ] **Step 2: Perbarui CLAUDE.md dan roadmap.** Verifikasi semua command yang ditulis memang ada dan jalan.
- [ ] **Step 3: Verifikasi akhir** — `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && node --test scripts/ && pnpm --filter @nabungo/ui typecheck && pnpm --filter @nabungo/ui test && pnpm --filter @nabungo/ui e2e`. Expected: semua hijau. Juga cek CI (`ci.yml` Linux + Windows, `release.yml`) hijau di PR.
- [ ] **Step 4: Commit** — `docs: Fase 8d selesai — offline dan rilis`
