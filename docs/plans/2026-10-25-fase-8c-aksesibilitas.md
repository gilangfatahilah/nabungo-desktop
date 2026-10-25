# Fase 8c: Aksesibilitas Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Semua layar lolos kontras WCAG AA (teks ≥ 4.5:1, elemen UI dan indikator fokus ≥ 3:1) di tema terang dan gelap, ukuran font bisa diubah dari Pengaturan dan shortcut, dan navigasi keyboard diaudit otomatis di semua layar.

**Architecture:** Ukuran font = `font-size` di `<html>` dalam persen. Seluruh UI memakai `rem` (Tailwind), jadi satu nilai menskalakan semuanya; sisa ukuran `px` yang relevan (virtualizer, teks sumbu chart, beberapa kelas `[..px]`) ikut diskalakan. Setting disimpan di DB (`ui.font_scale`) dan di-cache di `localStorage` agar layar kunci dan onboarding memakai ukuran yang sama sebelum DB terbuka. Audit memakai `@axe-core/playwright` (devDependency, tidak ikut bundle) di e2e terhadap browser-ledger, untuk semua route dan kedua tema.

**Tech Stack:** Rust (`nabungo-app` settings, tauri-specta), Vue 3 + Pinia, Tailwind v4, vitest, Playwright + axe-core.

**Spec:** `PRD.md` §8 NFR Aksesibilitas ("Navigasi penuh dengan keyboard, kontras WCAG AA, ukuran font bisa diubah"); `docs/design/design.md` §12; roadmap `docs/plans/2026-10-12-v1-roadmap.md` baris 8c.

## Kondisi saat ini (hasil riset)

1. **Teks sudah lolos.** Semua pasangan teks/latar di `style.css` ≥ 4.5:1 (tercatat di komentar `style.css` dan design.md §3.2).
2. **Border input gagal 3:1.** `--input` `#CFD5DC` vs Lembar = 1.48:1 (dark `#3A434F` vs `#1A1F27` = 1.65:1). Border ini satu-satunya batas visual input, select, checkbox, dan switch dalam keadaan mati (WCAG 1.4.11).
3. **Ring fokus 50% alfa.** 14 tempat memakai `ring-ring/50` / `outline-ring/50` (komponen shadcn di `components/ui/` + `DateInput`, `BudgetAmountCell`, `DonutChart`, `LineChart`, `AccountsCard`, base layer `style.css`). Zamrud 50% di atas putih ≈ 2.3:1.
4. **Tidak ada setting ukuran font.** Virtualizer `TxnList` (40/56) dan `TxnTable` (`ROW = 40`) memakai px tetap; teks sumbu `LineChart`/`BarChart` `font-size="11"` dalam unit viewBox = px.
5. **Tidak ada audit otomatis** kontras/ARIA/keyboard.

## Keputusan

| ID | Keputusan | Alasan |
|---|---|---|
| A1 | `--input` light `#8A94A0`, dark `#6B7583` (target ≥ 3:1 vs Lembar, Kanvas, popover). Nilai final dicek dengan skrip kontras; bila ada yang < 3:1 di Kanvas aktif, gelapkan/terangkan sampai lolos. `--border` (pemisah dekoratif) tidak diubah. | Pemisah dekoratif dikecualikan 1.4.11; batas kontrol tidak. |
| A2 | Semua `ring-ring/50` → `ring-ring`, `outline-ring/50` → `outline-ring`. Ketebalan ring tetap. | Zamrud penuh ≥ 4.5:1 di semua latar (dark 7.5+). |
| A3 | `FontScale` = persen, pilihan `[90, 100, 110, 125, 150]`, default 100, key `ui.font_scale`. Nilai hilang/tak valid → 100 (pola `auto_lock_minutes`). Set di luar pilihan → `AppError::InvalidFontScale` (code `invalid_font_scale`). `UiSettingsDto` mendapat `fontScale: number`; command baru `set_font_scale(scale: u16)`. | Konsisten dengan setting lain; validasi di Rust. |
| A4 | UI menerapkan `document.documentElement.style.fontSize = `${scale}%``. Store juga menulis `localStorage['nabungo.fontScale']` (try/catch) dan `applyFontScale(readCachedFontScale())` dijalankan saat store dibuat. DB tetap sumber kebenaran: `load()` menimpa cache. | Layar kunci/onboarding/pemulihan terbaca sebelum DB terbuka; preferensi tampilan bukan data sensitif. |
| A5 | Shortcut `Ctrl/⌘ + =` (dan `+`), `Ctrl/⌘ + -`, `Ctrl/⌘ + 0` → naik/turun satu langkah dalam pilihan A3 / reset 100. Aksi baru `{ kind: 'fontScale'; step: 1 \| -1 \| 0 }` di `createShortcutHandler`; aktif juga saat fokus di input (modifier). Di ujung skala tidak berbuat apa-apa. | Konvensi desktop; zoom hotkey bawaan Tauri mati. |
| A6 | Ukuran px yang harus ikut skala: estimasi baris virtualizer dikali `rootFontScale()` (= `parseFloat(getComputedStyle(html).fontSize) / 16`, 1 di jsdom), dan virtualizer di-`measure()` ulang saat skala berubah; `font-size` sumbu chart = `11 * scale`; kelas `text-[15px]` → `text-[0.9375rem]`, `h-[18px]` → `h-[1.125rem]`, `max-w-[440px]` → `max-w-[27.5rem]`. `ChartTooltip` offset 12px dibiarkan. | Tanpa ini baris virtual tumpang tindih di skala > 100%. |
| A7 | Audit = e2e `ui/e2e/a11y.spec.ts`: untuk setiap route utama (dasbor, transaksi Riwayat + Tabel, budget, alokasi, laporan, akun, kategori, tag/payee, terjadwal, pengaturan, onboarding, kunci, pemulihan), di light dan dark, jalankan axe dengan tag `wcag2a, wcag2aa, wcag21aa` dan harapkan 0 pelanggaran. Ditambah tes keyboard: dari dasbor, `Tab` berulang mencapai link sidebar, tombol "Transaksi baru", dan konten; setiap elemen yang terfokus punya `outline`/`box-shadow` yang terlihat (tidak `none`). Sheet transaksi dibuka dengan `N`, diisi, disimpan hanya dengan keyboard. | Audit yang bisa diulang di CI, bukan checklist sekali jalan. |
| A8 | Pelanggaran yang ditemukan axe diperbaiki di komponen terkait dalam Task 4. Pengecualian hanya lewat `.exclude()` per selektor dengan komentar alasan (mis. elemen dekoratif `aria-hidden`); tidak ada `disableRules` global. | Pengecualian harus kelihatan dan beralasan. |
| A9 | design.md §3.1 (nilai `--input`), §12 (ring penuh, ukuran font, audit axe) dan komentar kontras `style.css` diperbarui. | CLAUDE.md: perubahan UI mengikuti design.md. |

## Global Constraints

- Offline total: `@axe-core/playwright` hanya devDependency e2e; tidak ada aset remote.
- Komponen memakai token, bukan warna Tailwind mentah (design.md §3).
- `ui/src/ipc/bindings.ts` di-generate (`cargo test -p nabungo-desktop export_bindings`) dan di-commit.
- Code error baru `invalid_font_scale` wajib ada di `errors` `ui/src/i18n/id.ts` dan `en.ts`.
- `browser-ledger.ts` meniru aturan Rust untuk `setFontScale` (pilihan sama, error sama).
- pnpm `minimumReleaseAge`: bila install axe gagal, pin ke versi sebelumnya.

## Review Focus

1. **Skala 150% di jendela minimum 960×600:** sidebar, SiteHeader, dan sheet transaksi tidak boleh membuat scroll horizontal di halaman. Dikunci oleh tes e2e `font scale 150 has no horizontal overflow` (Task 3).
2. **Baris virtual saat skala berubah ketika daftar sudah terbuka:** baris tidak tumpang tindih. Dikunci oleh tes unit `TxnList` yang memeriksa `estimateSize` mengikuti `rootFontScale` (Task 3).
3. **`localStorage` tidak tersedia/melempar atau berisi sampah:** app tetap start di 100%. Dikunci oleh tes `readCachedFontScale` (Task 2).
4. **Shortcut font saat dialog/sheet terbuka atau fokus di input nominal:** tetap bekerja dan tidak mengetik `=`/`-` ke input. Dikunci oleh tes `createShortcutHandler` (Task 2).
5. **Dark mode komponen yang jarang dilihat** (toast, dialog konfirmasi destruktif, tooltip chart, badge): ikut dicek axe karena tes membuka masing-masing (Task 4).

---

### Task 1: Rust — setting `font_scale` (A3)

**Files:**
- Modify: `crates/app/src/settings.rs`, `crates/app/src/error.rs` (atau tempat `AppError` didefinisikan)
- Modify: `src-tauri/src/dto.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/error.rs` (mapping code)
- Regenerate: `ui/src/ipc/bindings.ts`

**Interfaces:**
- Produces: `pub const FONT_SCALE_CHOICES: [u16; 5] = [90, 100, 110, 125, 150]`; `UiSettings.font_scale: u16`; `Ledger::set_font_scale(&mut self, scale: u16) -> Result<(), AppError>`; `AppError::InvalidFontScale` → code `invalid_font_scale`.
- Produces (TS): `UiSettingsDto.fontScale: number`, `commands.setFontScale(scale: number)`.

- [ ] **Step 1: Tests di `settings.rs`:** `font_scale_default_100` (DB kosong → 100), `font_scale_roundtrip` (set 125 → `ui_settings().font_scale == 125`), `font_scale_rejects_unknown` (set 120 → `Err(AppError::InvalidFontScale)`), `font_scale_garbage_falls_back` (setting `"abc"` atau `"120"` langsung di tabel → 100).
- [ ] **Step 2:** `cargo test -p nabungo-app font_scale` → FAIL (belum ada).
- [ ] **Step 3:** Implementasi mengikuti pola `auto_lock_minutes`; DTO `font_scale` (`#[serde(rename_all = "camelCase")]` sudah ada), command `set_font_scale`, daftarkan di `collect_commands!`.
- [ ] **Step 4:** `cargo test -p nabungo-app font_scale` → PASS; `cargo test -p nabungo-desktop export_bindings`; `cargo clippy --workspace --all-targets -- -D warnings` bersih.
- [ ] **Step 5:** Commit `feat(app): setting ukuran font`.

### Task 2: UI — store, cache, shortcut, Pengaturan (A4, A5)

**Files:**
- Create: `ui/src/lib/fontScale.ts` (+ `fontScale.test.ts`)
- Modify: `ui/src/ipc/index.ts` (`setFontScale`), `ui/src/ipc/browser-ledger.ts`, `ui/src/stores/settings.ts` (+ test), `ui/src/lib/shortcuts.ts` (+ test), `ui/src/App.vue` (handler aksi), `ui/src/views/SettingsView.vue` (+ test), `ui/src/i18n/id.ts`, `en.ts`

**Interfaces:**
- Consumes: Task 1 bindings.
- Produces (`lib/fontScale.ts`): `FONT_SCALES = [90, 100, 110, 125, 150] as const`; `type FontScale = (typeof FONT_SCALES)[number]`; `isFontScale(v: unknown): v is FontScale`; `stepFontScale(cur: FontScale, step: 1 | -1 | 0): FontScale`; `applyFontScale(s: FontScale): void`; `readCachedFontScale(): FontScale`; `writeCachedFontScale(s: FontScale): void`; `rootFontScale(): number`.
- Produces (store): `settings.fontScale: Ref<FontScale>`, `settings.setFontScale(s: FontScale): Promise<void>`, `settings.stepFontScale(step): Promise<void>`.
- Produces (shortcut): `ShortcutAction` `{ kind: 'fontScale'; step: 1 | -1 | 0 }`.

- [ ] **Step 1: Tests.**
  - `fontScale.test.ts`: `stepFontScale(100, 1) === 110`, `stepFontScale(150, 1) === 150`, `stepFontScale(90, -1) === 90`, `stepFontScale(125, 0) === 100`; `readCachedFontScale()` → 100 bila kosong, bila `'abc'`, bila `'120'`, dan bila `localStorage.getItem` melempar; `applyFontScale(125)` → `document.documentElement.style.fontSize === '125%'`.
  - `shortcuts.test.ts`: `Ctrl+=`, `Ctrl++` → step 1; `Ctrl+-` → -1; `Ctrl+0` → 0; target `<input>` tetap memicu aksi dan `preventDefault` dipanggil; `=` tanpa modifier tidak memicu.
  - `settings.test.ts`: `load()` mengambil `fontScale` dari backend, menerapkannya ke `<html>`, dan menulis cache; `setFontScale(110)` memanggil IPC.
  - `browser-ledger`: `setFontScale(120)` → error code `invalid_font_scale`.
  - `SettingsView.test.ts`: ada Select berlabel "Ukuran teks" dengan opsi `90%`…`150%`.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui test` → FAIL pada tes baru.
- [ ] **Step 3:** Implementasi. Select di Pengaturan di bawah Tema, label `settings.fontScale` ("Ukuran teks" / "Text size"), hint `settings.fontScaleHint` ("Juga: Ctrl + = / Ctrl + − / Ctrl + 0" / "Also: …"). `errors.invalid_font_scale`. App.vue menangani aksi `fontScale` lewat `withErrorToast(() => settings.stepFontScale(step))`. Shortcut font dicek sebelum cabang `isTyping`.
- [ ] **Step 4:** `pnpm --filter @nabungo/ui test` dan `typecheck` → PASS.
- [ ] **Step 5:** Commit `feat(ui): ukuran teks bisa diubah`.

### Task 3: UI — ukuran px ikut skala (A6)

**Files:**
- Modify: `ui/src/components/txn/TxnList.vue`, `TxnTable.vue`, `components/charts/LineChart.vue`, `BarChart.vue` (bila punya teks sumbu), `RecoveryKeyDialog.vue`, `AppLogo.vue`, `NewTxnSheet.vue`, `views/OnboardingView.vue`, `views/UnlockView.vue`, `views/RecoveryView.vue` (bila ada `440px`), `App.vue` (komentar 440px)
- Test: `ui/src/components/txn/TxnList.test.ts` (atau test yang ada), `ui/e2e/a11y.spec.ts` (tes overflow)

**Interfaces:**
- Consumes: `rootFontScale()`, `settings.fontScale` (untuk memicu `virtualizer.measure()` lewat `watch`).

- [ ] **Step 1: Tests.** Unit: dengan `rootFontScale` di-mock 1.25, estimasi baris tanggal `TxnList` = 50 dan baris transaksi = 70; `TxnTable` = 50. E2e `font scale 150 has no horizontal overflow`: viewport 960×600, setel 150% lewat Pengaturan, kunjungi dasbor, transaksi, budget, buka sheet (`n`) → `document.documentElement.scrollWidth <= clientWidth`.
- [ ] **Step 2:** Jalankan → FAIL.
- [ ] **Step 3:** Implementasi sesuai A6. Bila tes overflow gagal karena layout, perbaiki dengan `min-w-0`/wrap di komponen yang meluap (bukan dengan mengecilkan skala).
- [ ] **Step 4:** Unit + e2e → PASS.
- [ ] **Step 5:** Commit `fix(ui): ukuran baris virtual dan chart mengikuti ukuran teks`.

### Task 4: Kontras + audit axe semua layar (A1, A2, A7, A8, A9)

**Files:**
- Modify: `ui/src/style.css`, 14 file dengan `ring-ring/50`/`outline-ring/50`, komponen yang dilaporkan axe
- Create: `ui/e2e/a11y.spec.ts` (+ helper bersama dengan Task 3)
- Modify: `ui/package.json` (`@axe-core/playwright`), `docs/design/design.md`

- [ ] **Step 1: Tests.** `a11y.spec.ts`: helper `seed(page)` membuat onboarding + beberapa akun/transaksi/budget lewat UI (pola spec e2e lain); untuk setiap route × `['light','dark']` → `new AxeBuilder({ page }).withTags(['wcag2a','wcag2aa','wcag21aa']).analyze()` → `violations` = `[]` (pesan gagal mencetak `id` + `target`). Tambahan: dialog konfirmasi hapus, toast, tooltip chart (hover) di-scan dalam keadaan terbuka; layar kunci dan pemulihan lewat hook browser-ledger yang ada. Tes keyboard sesuai A7.
- [ ] **Step 2:** `pnpm --filter @nabungo/ui e2e a11y` → catat pelanggaran (minimal `color-contrast` border tidak ditangkap axe; tetap ubah A1).
- [ ] **Step 3:** Terapkan A1, A2; perbaiki setiap pelanggaran axe di komponennya. Cek nilai `--input` baru dengan skrip kontras terhadap Lembar, Kanvas, Kanvas aktif, popover.
- [ ] **Step 4:** e2e a11y + seluruh e2e + vitest → PASS.
- [ ] **Step 5:** Update design.md (A9) dan komentar `style.css`; commit `fix(ui): kontras WCAG AA dan audit aksesibilitas otomatis`.

### Task 5: Roadmap

- [ ] Tandai 8c selesai di `docs/plans/2026-10-12-v1-roadmap.md` beserta ringkasan keputusan dan yang ditunda (cek manual pembaca layar di WebKitGTK/WebView2 → 8d). Commit `docs: Fase 8c selesai — aksesibilitas`.
