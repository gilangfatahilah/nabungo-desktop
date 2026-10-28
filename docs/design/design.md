# Nabungo — Panduan desain

Acuan visual & interaksi untuk semua layar Nabungo. Setiap perubahan UI dicek terhadap dokumen ini. Requirement fungsional tetap di `PRD.md`; dokumen ini hanya mengatur *bagaimana tampilannya*.

## 1. Prinsip

- **Subjek:** buku kas pribadi orang Indonesia: rekening bank, e-wallet, dompet tunai, siklus gajian.
- **Pengguna:** satu orang, di laptop/desktop, membuka aplikasi beberapa kali seminggu untuk mencatat dan mengecek "masih aman nggak sampai gajian berikutnya".
- **Tugas utama layar:** mencatat transaksi secepat mungkin (keyboard), lalu membaca angka dengan yakin.

Angka adalah konten utama, bukan dekorasi. Kerangka aplikasi mengikuti pola dasbor shadcn yang familier, tetapi isinya tetap padat dan tenang: satu aksen (Tunas), warna hanya bermakna, dan angka selalu rapi sejajar.

## 2. Referensi: shadcn-vue `dashboard-01`

Kerangka mengikuti blok [`dashboard-01`](https://www.shadcn-vue.com/view/dashboard-01).

| Diadopsi | Tidak diadopsi |
|---|---|
| `Sidebar variant="inset"` + `SidebarInset` berlatar Lembar | `NavUser` (avatar/akun): Nabungo satu pengguna, offline |
| `SiteHeader` setinggi `--header-height` dengan `SidebarTrigger`, pemisah, judul | `NavDocuments` dan menu `⋯` per item sidebar |
| Tombol "Quick Create" di atas nav utama | `@tabler/icons-vue` → tetap Lucide (§10) |
| Section cards KPI dengan badge tren, grid berbasis container query | `@unovis` → tetap chart SVG sendiri (`components/charts/`) |
| Chart card dengan pemilih rentang | `@tanstack/vue-table`, drag-reorder baris, tab pada tabel |
| Tabel data di dalam kartu | Tema neutral default (primary hitam) |

## 3. Warna

Token didefinisikan sebagai CSS variable di `ui/src/style.css` (`:root` dan `.dark`), lalu dipetakan ke nama shadcn. Komponen **tidak boleh** memakai warna Tailwind mentah (`text-red-600`, `bg-gray-100`); selalu pakai token.

### 3.1 Palet dasar "Tunas"

Tunas adalah hijau logo Nabungo (§3.4): hijau tua di light, lime (warna koin logo) di dark. Dipakai hemat: aksi utama, fokus, seleksi.

| Nama | Peran | Light | Dark | Token shadcn |
|---|---|---|---|---|
| **Tinta** | teks utama | `#17202A` | `#E8E8EC` | `--foreground` |
| **Tinta pudar** | teks sekunder, meta | `#5B6472` | `#A1A1AA` | `--muted-foreground` |
| **Kanvas** | sidebar & latar di luar inset, hover, latar muted | `#F4F6F8` | `#111115` | `--sidebar`, `--muted`, `--secondary`, `--accent` (light) |
| **Kanvas aktif** | item sidebar aktif/hover | `#E6EAEE` | `#27272F` | `--sidebar-accent` |
| **Lembar** | inset konten, kartu, sheet, dialog | `#FFFFFF` | `#1C1C22` | `--background`, `--card`, `--popover` |
| **Garis** | border & pemisah | `#E1E5EA` | `#2E2E36` | `--border` (pemisah dekoratif) |
| **Batas kontrol** | border input, select, checkbox, track switch | `#7C8692` | `#71717C` | `--input` |
| **Isi kontrol** | latar input/select/tombol outline (dark), input nonaktif | `#CFD5DC` | `#3A3A44` | `--input-fill` (`bg-input-fill/*`) |
| **Tunas** | aksi utama, fokus, seleksi | `#167A3A` | `#A3E635` (lime-400) | `--primary`, `--ring` |

Batas kontrol ≥ 3:1 terhadap Lembar, Kanvas, Kanvas aktif, dan popover (light 3.7/3.4/3.1, dark 3.5/3.9/3.1/3.3; WCAG 1.4.11). Garis (`--border`) hanya untuk pemisah dekoratif, tidak untuk batas kontrol. Isi kontrol tetap memakai nilai lama agar latar input tidak menggelap; jangan pakai `bg-input` untuk isi selain track switch.

Teks di atas Tunas memakai `#FFFFFF` (light, 5.4:1) / `#1A2E05` (dark, lime-950, 9.7:1). Lime tidak dipakai di light: teks putih di atasnya hanya ~1.6:1 dan lime sebagai teks di Lembar putih gagal kontras.

Netral dark mengikuti tile ikon (Graphite `#1C1C22`, Black `#09090B`): abu netral tanpa rona biru.

### 3.2 Warna semantik

| Nama | Arti | Light | Dark | Token |
|---|---|---|---|---|
| **Masuk** | income, sisa positif | `#1E7A4F` | `#5CC38E` | `--income` |
| **Keluar** | expense | `#B4382F` | `#F08A80` | `--expense` |
| **Pindah** | transfer | = Tinta pudar | = Tinta pudar | `--transfer` |
| **Waspada** | budget ≥ 80%, peringatan | `#9A5B00` | `#E5A84B` | `--warning` |
| **Bahaya** | destruktif, error, budget lewat | = Keluar | = Keluar | `--destructive` |

Semua pasangan teks/latar lolos kontras ≥ 4.5:1 di Lembar, Kanvas, dan Kanvas aktif. Cek ulang bila mengubah nilai.

**Aturan warna semantik**

- Warna income/expense **tidak pernah jadi satu-satunya pembeda**. Selalu ada tanda (`+` / `−` U+2212) atau ikon.
- Warna hanya pada **angka** (dan ikon tren di badge), bukan latar baris atau seluruh teks baris.
- Total, saldo, dan kekayaan bersih memakai **Tinta**. Saldo negatif memakai Keluar.
- Tunas tidak dipakai untuk angka (akan terbaca sebagai income).

### 3.3 Warna kategori

Kategori boleh punya warna sendiri (dari `IconColorPicker`). Warna itu hanya muncul sebagai **tile ikon** 28px, dengan latar warna kategori 14% alfa dan ikon warna penuh. Tidak dipakai untuk teks atau chart utama, kecuali di laporan per kategori.

### 3.4 Logo & ikon aplikasi

Huruf N berfaset yang naik ke kanan dengan koin lime yang jatuh masuk ("menabung, lalu tumbuh"). Acuan visual: `docs/design/brand/Nabungo — logo & app icon@2x.png`; sumber vektor di folder yang sama:

| File | Isi | Dipakai di |
|---|---|---|
| `nabungo-mark.svg` | tanda N + koin, tanpa latar | `AppLogo.vue` (sidebar, Unlock, Pemulihan) |
| `nabungo-icon.svg` | tanda di tile graphite membulat | ikon aplikasi ≥ 48px |
| `nabungo-icon-small.svg` | tile + N diperbesar, **tanpa koin** | ikon ≤ 44px (16–44, `.ico` 16/24/32), favicon |

- Warna tanda tetap (gradien faset `#417A35`→`#145541`, `#56A046`→`#38864F`, `#8AD23E`→`#4AA356`, pucuk putih 28%, koin `#A3E635`), tidak ikut tema; ini satu-satunya pengecualian aturan gradient §5. Tanda bersifat dekoratif (`aria-hidden`), nama dibaca dari wordmark teks.
- Wordmark = teks `app.name` lowercase bold dengan font aplikasi, bukan gambar.
- Regenerate ikon setelah mengubah SVG: render `nabungo-icon.svg` ke PNG 1024 (mis. Chromium Playwright), `pnpm tauri icon <png>` (hapus folder `android/`, `ios/` yang ikut dibuat), lalu timpa `32x32.png`, `Square30x30Logo.png`, `Square44x44Logo.png`, entri 16/24/32 di `icon.ico`, dan `ui/public/favicon.png` (64px) dengan render `nabungo-icon-small.svg`.

## 4. Tipografi

**Satu keluarga: Plus Jakarta Sans** (variable, dibundel via `@fontsource-variable/plus-jakarta-sans`; tidak ada aset font remote). Basis 14px, skala ~1.2:

| Token | Ukuran / line-height | Berat | Pemakaian |
|---|---|---|---|
| `text-display` | 32 / 38 | 600 | angka tunggal paling penting (jarang; mis. dialog ringkasan) |
| `text-title` | 22 / 28 | 600 | angka KPI di section card |
| `text-heading` | 16 / 24 | 600 | judul halaman di SiteHeader, judul kartu, sheet, dialog |
| `text-body` | 14 / 20 | 400 / 500 | teks dasar, isi baris, input |
| `text-meta` | 12.5 / 16 | 400 | meta baris, deskripsi kartu, helper text, header tanggal |

**Aturan**

- **Angka uang selalu `tabular-nums`** (utilitas `.num`) dan rata kanan di kolom. Komponen: `MoneyText`.
- Prefiks `Rp` berat 400 dengan warna yang sama dengan angka (tanpa opasitas: `Rp` 65% hanya ~3:1, gagal kontras teks); angka berat 500/600. Ini satu-satunya "gaya" angka.
- Tidak ada huruf kapital semua, tidak ada eyebrow di atas judul, tidak ada letter-spacing lebar.
- Tebal maksimal 600. Paragraf maksimal ~65 karakter per baris.

## 5. Ruang, ukuran, sudut, bayangan

- **Grid 4px.** Jarak: 4, 8, 12, 16, 24, 32, 48. Hindari nilai lain dan nilai arbitrer.
- **Tinggi kontrol:** 32px (input, select, tombol default), 28px (tombol kecil di toolbar/baris, chip), 40px (field nominal di form transaksi), 20px (kbd). SiteHeader 48px (`--header-height`).
- **Tinggi baris daftar:** 48px (dua baris teks). Baris header tanggal 32px. Baris tabel data 40px.
- **Sudut, berhierarki:** 6px kontrol & menu melayang (`rounded-lg`); 10px kartu, panel, dialog, inset (`rounded-xl`); 12px sheet (`rounded-2xl`); pill untuk chip & badge.
- **Bayangan:** lapisan melayang (popover, dropdown, dialog, sheet, toast) memakai bayangan penuh. Inset konten `shadow-sm` (bawaan varian inset). Kartu `shadow-xs`. Elemen lain tanpa bayangan.
- **Gradient:** satu-satunya gradient adalah latar section card (`bg-linear-to-t from-primary/5 to-card`, §7.11). Tidak ada gradient lain, kecuali tanda logo (§3.4).

## 6. Layout

### 6.1 Kerangka aplikasi

```
┌─ Kanvas ──────────────────────────────────────────────────────────────────┐
│ ┌ Sidebar 256 ─┐ ┌ Inset (Lembar, radius 10, shadow-sm, margin 8) ───────┐ │
│ │ ◓ nabungo    │ │ [≡] │ Dasbor                         [🔍 Cari Ctrl K] │ │ ← SiteHeader 48px, sticky, border-b
│ │ [+ Transaksi][🔍]│ ├──────────────────────────────────────────────────────┤ │
│ │ ▣ Dasbor     │ │ Periode 25 Sep – 24 Okt            [Sekunder] [Utama] │ │ ← PageHeader (opsional)
│ │ ▣ Transaksi  │ │                                                      │ │
│ │ …            │ │ Konten @container/main                               │ │
│ │ Kelola       │ │ px-4 (lg: px-6), py-4 (md: py-6), gap 16 (md: 24)    │ │
│ │ …            │ │ lebar penuh                                          │ │
│ │ ⚙ Pengaturan │ │                                                      │ │
│ │ Kekayaan…    │ │                                                      │ │
│ └──────────────┘ └──────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────────┘
```

- `SidebarProvider` men-set `--sidebar-width: 16rem` dan `--header-height: 3rem`. `Sidebar variant="inset" collapsible="icon"`: bisa diciutkan ke kolom ikon lewat `SidebarTrigger` atau `Ctrl+B`.
- Latar luar dan sidebar = Kanvas; konten ada di **inset** Lembar. Bedanya cukup untuk memisahkan navigasi dari isi tanpa border tebal.
- Area scroll ada **di dalam inset**, di bawah SiteHeader. SiteHeader tidak ikut scroll.
- Konten lebar penuh. Breakpoint isi halaman memakai **container query** `@container/main` (`@xl/main`, `@5xl/main`), bukan viewport, sehingga tata letak ikut berubah saat sidebar diciutkan.
- Halaman yang isinya form atau teks (Pengaturan, Onboarding) membatasi lebarnya sendiri (`max-w-2xl`). Tabel dan daftar memakai lebar penuh.
- Lebar minimum jendela 960px.

### 6.2 Sidebar

```
 ◓ nabungo                        ← AppLogo dalam SidebarMenuButton (ciut: tanda koin saja)
 [ + Transaksi baru         N ]  ← Quick Create: tombol Tunas (Cari hanya di SiteHeader)
 ▣ Dasbor                    G D  ← NavMain: ikon 16px + label, Kbd saat hover/aktif
 ▣ Transaksi                 G T
 ▣ Budget                    G B
 ▣ Alokasi                   G A
 ▣ Laporan                   G R

 Kelola                           ← SidebarGroupLabel
 ▣ Akun
 ▣ Kategori
 ▣ Tag & Payee
 ▣ Terjadwal                   2  ← SidebarMenuBadge jumlah jatuh tempo (sembunyi saat 0)
 ▣ Inbox                       5  ← hanya bila email capture aktif / ada item; badge = perlu ditinjau

 ▣ Pengaturan             Ctrl ,  ← NavSecondary, menempel di bawah (mt-auto)
 ──────────────────────────────
 Kekayaan bersih                  ← footer NavSummary: link ke Akun
 4.343.000                  [🔒]  ← angka tanpa Rp; tombol Kunci hanya bila DB terenkripsi
```

- Item aktif & hover: gaya bawaan shadcn (latar Kanvas aktif, teks Tinta 500). Tidak ada garis aksen.
- Quick Create adalah satu-satunya blok Tunas di sidebar.
- Petunjuk shortcut hanya saat hover/fokus dan pada item aktif.
- Mode ikon: label & Kbd disembunyikan, setiap item punya tooltip berisi label; Quick Create menjadi tombol ikon `+`; footer menampilkan ikon saja.
- Daftar saldo per akun **tidak** ada di sidebar; tempatnya kartu Akun di Dasbor (§8.1) dan layar Akun.
- Di sidebar angka tanpa prefiks `Rp`.

### 6.3 SiteHeader

`ui/src/components/layout/SiteHeader.vue`. Tinggi 48px, `border-b`, isi `px-4 lg:px-6`:

- `SidebarTrigger` · `Separator` vertikal 16px · **h1** judul halaman (`text-heading` 500, sama dengan label navigasi `nav.<route>`).
- Kanan: aksi global saja (pemicu Cari berbentuk kolom isian — `outline`, tinggi 36px, lebar 224px / 320px di `lg`, teks "Cari…" + `Kbd` `Ctrl K` — yang membuka command palette; satu-satunya tombol Cari di aplikasi). Aksi khusus halaman **tidak** di sini.

### 6.4 PageHeader

`ui/src/components/PageHeader.vue`, baris pertama konten bila halaman punya ringkasan atau aksi. Tidak ada judul (judul ada di SiteHeader).

- Kiri: ringkasan satu baris `text-meta` Tinta pudar, mis. "Kekayaan bersih Rp 4.343.000".
- Kanan: maksimal satu tombol utama (Tunas) dan dua sekunder; sisanya masuk menu `⋯`. Kontrol halaman (segmented control, navigasi periode) juga boleh di sini.
- Tidak dirender bila keduanya kosong.

### 6.5 Kartu vs permukaan datar

- **Kartu** (`Card`: border Garis, radius 10, `shadow-xs`) untuk blok yang berdiri sendiri di Dasbor dan ringkasan: section card, chart card, tabel ringkas.
- **Datar** untuk layar kerja (Transaksi, Akun, Kategori, Budget): daftar, tabel, dan form langsung di Lembar dengan pemisah Garis 1px.
- Kartu tidak bersarang. Tidak ada latar berwarna untuk "menghias" seksi.

## 7. Komponen

### 7.1 Tombol

| Varian | Pemakaian | Tampilan |
|---|---|---|
| **Utama** | satu aksi terpenting per layar/dialog | latar Tunas, teks `--primary-foreground` |
| **Sekunder** | aksi lain yang sejajar | border Garis, latar Lembar |
| **Hantu** | aksi di baris, toolbar, header, ikon | tanpa border, hover Kanvas |
| **Bahaya** | hapus permanen, gabung kategori | latar Bahaya; hanya di dialog konfirmasi |

- Label berupa kata kerja yang menyebut hasilnya: "Simpan transaksi", "Arsipkan akun", bukan "OK"/"Submit".
- Petunjuk shortcut di dalam tombol memakai `Kbd` di sisi kanan, opasitas 70%.

### 7.2 Kbd

Satu komponen (`ui/kbd`) untuk semua petunjuk shortcut: tinggi 20px, padding 0 6px, radius 4, border Garis, `text-meta` 500, font UI. Kombinasi ditulis per tombol: `Ctrl` `Enter`. `G D` = dua `Kbd` berurutan.

### 7.3 Input & form

- Label di atas field, `text-body` 500. Helper/error di bawah field, `text-meta`. Error berwarna Bahaya dan **menempel di field yang salah**.
- Field error: border Bahaya + teks error. Fokus: ring 2px Tunas (semua elemen interaktif).
- **Nominal:** rata kanan, `num`, 20px di form transaksi. Prefiks `Rp` di dalam field sebagai addon pudar.
- **Tanggal:** `DateInput` (Popover + Calendar). Tampilan `17 Okt 2026`, nilai `YYYY-MM-DD`.
- **Select:** shadcn `Select`/`Combobox`, tidak ada `<select>` native.
- Lebar field mengikuti isi: tanggal ~160px, nominal min/maks ~140px.

### 7.4 Sheet transaksi

```
┌ Transaksi baru ──────────────────── ✕ ┐
│ [ Pengeluaran | Pemasukan | Transfer ] │  segmented control
│ Nominal                                │
│ Rp                    28.000           │  20px, rata kanan
│ Kategori                  [Bagi ▸]     │  tombol hantu "Bagi" = split
│ [ Makan & Minum › Kopi & jajan      ]  │
│ Akun                Tanggal            │  dua kolom
│ [ BCA          ▾ ]  [ 17 Okt 2026   ]  │
│ Toko / pihak                           │
│ Tag · Catatan  (terlipat)              │
├────────────────────────────────────────┤
│                [Batal] [Simpan  Ctrl ⏎]│  footer menempel
└────────────────────────────────────────┘
```

- Lebar 440px. Footer aksi selalu terlihat. Toast bergeser ke kiri selama sheet terbuka.
- Urutan field mengikuti urutan Tab yang diuji e2e: nominal → kategori → … Jangan diubah tanpa memperbarui test.
- Field opsional (tag, catatan, lampiran) terlipat secara default dan terbuka otomatis bila sudah berisi.
- Lampiran: daftar baris 36px (ikon gambar/dokumen, nama, ukuran, `Buka`, hapus dengan konfirmasi) + tombol outline `Lampirkan file`. File hilang ditandai "File hilang" dan `Buka` nonaktif. Transaksi baru hanya menampilkan "Simpan dulu untuk melampirkan struk".

### 7.5 Baris transaksi

```
 17 Okt 2026, Jumat                                  −Rp 662.000  ← header tanggal 32px
 [☕] Janji Jiwa                                   −Rp 22.000   ⋯
      Kopi & jajan   BCA   #kantor                  4.321.000   ← saldo berjalan
```

- Kiri: tile ikon kategori (28px, §3.3). Transfer: ikon panah dua arah di tile Kanvas. Saldo awal/penyesuaian: ikon bendera.
- Baris 1: payee (atau kategori bila payee kosong), `text-body` 500.
- Baris 2 (`text-meta`, Tinta pudar): kategori anak (induk di tooltip), akun, `#tag`. Pemisah jarak 12px.
- Kanan: nominal bertanda dengan warna semantik; di bawahnya saldo berjalan (bila filter satu akun).
- Filter satu akun: ikon lingkaran centang 14px untuk transaksi terekonsiliasi ("Sudah direkonsiliasi"). Ikon klip 14px untuk transaksi berlampiran ("Ada lampiran").
- Menu `⋯` saat hover/fokus. Baris bisa difokus (Tab/↑↓), `Enter` edit, `Delete` hapus. Hover Kanvas, fokus ring Tunas di dalam baris.

### 7.6 Filter (Transaksi)

```
 [🔍 Cari…            ]  [Periode: Okt 2026 ▾]  [Akun ▾]  [+ Filter]      12 transaksi
 chip aktif:  (Kategori: Kopi ✕) (≥ Rp 50.000 ✕)                     Hapus semua filter
```

- Selalu terlihat: cari, periode, akun. Kategori, tag, nominal min/maks di popover "+ Filter". Filter aktif tampil sebagai chip. Jumlah hasil di kanan.

### 7.7 Empty state

Teks rata kiri di posisi konten pertama, satu kalimat penjelas dan satu tombol aksi:

> **Belum ada transaksi bulan ini.**
> Catat pengeluaran pertama dan saldo akun langsung ikut berubah.
> [Catat transaksi `N`]

Kosong karena filter: "Tidak ada transaksi yang cocok dengan filter ini." + [Hapus filter]. Tidak ada ilustrasi. Di dalam kartu, empty state memakai padding kartu.

### 7.8 Toast

- Kanan bawah, **maksimal satu terlihat**.
- Hasil aksi dengan kata kerja tombolnya ("Transaksi disimpan"). Undo sebagai tombol di toast.
- Error tidak memakai toast bila bisa ditempel ke field.

### 7.9 Dialog konfirmasi

Judul berupa pertanyaan dengan objeknya ("Hapus akun Dompet?"), isi menjelaskan akibat dan alternatif. Tombol: [Batal] [aksi spesifik, Bahaya bila destruktif]. Fokus awal di Batal untuk aksi destruktif.

### 7.10 Angka besar & batang progres

- `MoneyText` punya ukuran `sm | md | lg | display`.
- Batang progres budget (`BudgetBar`): tinggi 6px, pill, track Garis. Isi Tinta pudar < 80%, Waspada ≥ 80%, Bahaya ≥ 100% (bagian lewat menonjol keluar track). Warna mengikuti `level` backend (`ok`/`warning`/`exceeded`), tidak dihitung ulang di UI.

### 7.11 Section card (KPI)

```
┌──────────────────────────────────┐
│ Pengeluaran            [↗ +12%]  │  CardDescription (meta) + badge outline di CardAction
│ Rp 5.088.000                     │  MoneyText, text-title 600, num
│                                  │
│ Naik dari periode lalu ↗         │  CardFooter: baris 1 text-body 500
│ Periode 25 Sep – 24 Okt          │             baris 2 meta Tinta pudar
└──────────────────────────────────┘
```

- Grid `grid-cols-1 @xl/main:grid-cols-2 @5xl/main:grid-cols-4`, gap 16. Setiap kartu `@container/card`, latar `bg-linear-to-t from-primary/5 to-card`, `shadow-xs`.
- Badge tren: `Badge variant="outline"` berisi ikon `trending-up`/`trending-down` + persen bertanda (`+12%`, `−4%`). Tanpa data periode lalu: badge tidak tampil.
- Warna ikon tren mengikuti makna, bukan arah: pengeluaran naik = Keluar, pemasukan/sisa/kekayaan naik = Masuk. Teks badge tetap Tinta.
- Persen tren adalah presentasi dari dua angka backend (periode ini vs lalu); saldo dan total tidak pernah dihitung di UI.

### 7.12 Chart card

- `Card` dengan `CardHeader` (judul `text-heading`, deskripsi meta) dan pemilih rentang dropdown `Select` (trigger `size="sm"`, label santai: "3 bulan terakhir", "6 bulan terakhir", "1 tahun terakhir") di `CardAction`.
- Chart memakai komponen di `ui/src/components/charts/` (SVG sendiri). Area di bawah garis berisi 15% alfa warna seri. Seri masuk/keluar memakai `--income`/`--expense`; seri lain `--chart-1..5` (Tunas, Biru `#24508F`/`#8FB0E8`, Keluar, Waspada, Tinta pudar).
- Tidak ada gradient atau 3D. Chart di layar Laporan tetap punya tabel angka di bawahnya.

### 7.13 Tabel data

- shadcn `Table` di dalam `Card` (Dasbor) atau langsung di Lembar (layar kerja). Header `text-meta` Tinta pudar 500, baris 40px, pemisah Garis.
- Kolom angka rata kanan, `num`. Kolom teks terpotong dengan `truncate`, nilai penuh di `title`.
- Baris yang bisa dibuka: seluruh baris bisa diklik/difokus, hover Kanvas.
- Header yang bisa disortir: `button` berisi label + ikon `arrow-up-down` (tidak aktif) / `arrow-up` / `arrow-down`, `aria-sort` di `<th>`, `aria-label` "Urutkan menurut {kolom}". Sortir dijalankan backend agar benar lintas paginasi.
- Tabel panjang divirtualisasi dengan baris pengisi `<tr>` atas/bawah (struktur `<table>` tetap utuh). Header lengket memakai `<table>` langsung di dalam scroller, bukan komponen `Table` (pembungkus `overflow-x-auto`-nya mematahkan `sticky`).

## 8. Pola layar

### 8.1 Dasbor

Susunan "balance details": hero kekayaan bersih di atas, lalu grid kartu 12 kolom. Semua angka dari backend; UI hanya membagi untuk persen/lebar batang.

```
 SiteHeader: [≡] │ Dasbor                                  [🔍 Cari  Ctrl K]
 Periode 1 Okt – 31 Okt                                    [+ Transaksi baru]

 Kekayaan bersih                          │ Sisa periode   Pemasukan    Pengeluaran
 Rp 4.597.000  [↗ +3%]                    │ 3.412.000 ↗    8.500.000    5.088.000 ↘
 Aset dikurangi liabilitas                │

 ┌ Akun ────────────── [+] ┐ ┌ [Masuk][Transfer][Keluar] ┐ ┌ Tren ─ [6 bulan ▾] ────┐
 │ ◉ 🏛 BCA       4.343.000 │ │ BCA · Bank                │ │ Arus kas │ Kekayaan bersih│
 │   Bank                   │ │ Rp 4.343.000              │ │  ╱╲  ╱───╱             │
 │ ○ 💵 Dompet      250.000 │ │ Grup Harian  Rekon 15 Okt │ │ ╱  ╲╱                  │
 │ ○ 💳 Kartu     −1.200.000│ ├ Budget periode ini ───────┤ │                        │
 │   (maks tinggi, scroll)  │ │ ▓▓▓▓▓▓▓▓▓░░░░░  62%       │ │                        │
 └──────────────────────────┘ └───────────────────────────┘ └────────────────────────┘
 ┌ Transaksi terakhir ─── Lihat semua ┐ ┌ Pengeluaran periode ini ┐ ┌ Komposisi kekayaan ┐
 │ 17 Okt  Janji Jiwa  Kopi  −22.000  │ │ Ini   Lalu   Sisa budget │ │      ◯             │
 │ …  (8 baris)                       │ │  ◯   ● Makan   1.250.000 │ │ ● Bank   78%       │
 │                                    │ │      ● Lainnya   300.000 │ │ Aset · Liab · Net  │
 │                                    │ ├ Budget perlu dilihat ────┤ │                    │
 │                                    │ │ Makan  ▓▓▓▓▓▓▓░ 86%      │ │                    │
 └────────────────────────────────────┘ └──────────────────────────┘ └────────────────────┘
 ┌ Terjadwal jatuh tempo ────────────┐ ┌ Ringkasan alokasi ────────────────┐
 └───────────────────────────────────┘ └───────────────────────────────────┘
```

- **Hero** (tanpa kartu): label meta, Kekayaan bersih `text-display` 600 + badge tren (§7.11), keterangan "Aset dikurangi liabilitas". Di kanan (≥ `@3xl/main`, di bawah pada layar sempit) tiga statistik ringkas dipisah garis vertikal: Sisa periode ini, Pemasukan, Pengeluaran, masing-masing nominal `text-heading` + badge tren. Periode berjalan tetap di PageHeader.
- **Grid**: `grid-cols-1 @3xl/main:grid-cols-2 @5xl/main:grid-cols-12`, gap 16/24. Baris 1: Akun `col-span-4`, kolom tengah `col-span-3` (Detail akun + Budget periode ini), Tren `col-span-5` (di 2 kolom: Tren `col-span-2`). Baris 2: Transaksi terakhir `col-span-5` (2 kolom: `col-span-2`), kolom tengah `col-span-4` (Pengeluaran + Budget perlu dilihat), Komposisi kekayaan `col-span-3`. Baris 3 `@3xl/main:grid-cols-2`: Jatuh tempo, Alokasi. Kartu yang datanya kosong menyembunyikan dirinya; kolom yang kosong ikut hilang. Grid item meregang setinggi baris, dan di kolom bertumpuk kartu terakhir `flex-1`, jadi tepi bawah satu baris selalu sejajar (isi kartu tetap rata atas).
- **Akun**: akun aktif (urutan grup lalu `sortOrder`) sebagai baris yang bisa dipilih: radio, tile ikon jenis/ikon akun, nama + jenis, saldo `num` (`balanceTone`). Baris terpilih `border-primary bg-primary/5`. Isi kartu di-scroll bila melebihi ±22rem. Tombol [+] di `CardAction` → layar Akun. Pilihan default: akun aset aktif pertama.
- **Detail akun** (akun terpilih): tiga aksi cepat (Pemasukan, Transfer, Pengeluaran) membuka sheet transaksi baru dengan jenis dan akun itu terisi. Lalu nama · jenis, saldo `text-title`, dan baris meta: grup, status rekonsiliasi (badge "Rekon s.d. tanggal" / "Belum direkonsiliasi"); kartu kredit menambah limit dan tanggal tagihan.
- **Budget periode ini**: total terpakai dari tersedia periode berjalan, batang dua nada (`bg-primary` terpakai, `bg-primary/15` sisa; `bg-expense` bila lewat), persen dibulatkan. Belum ada budget: kartu tetap tampil dengan keterangan "Belum ada budget untuk periode ini." dan tombol outline sm **Atur budget** → layar Budget.
- **Tren**: `Tabs` varian `line` Arus kas | Kekayaan bersih (default Arus kas), rentang 3/6/12 periode (default 6) di `CardAction`. Kekayaan bersih = satu seri `--chart-1` dengan area; Arus kas = pemasukan vs pengeluaran (§7.12).
- **Transaksi terakhir**: seperti sebelumnya (8 baris, klik/Enter → sheet edit). Tanpa transaksi: empty state §7.7 di dalam kartu.
- **Pengeluaran periode ini**: tiga angka (Periode ini, Periode lalu, Sisa budget bila ada budget), lalu `DonutChart` 5 kategori root teratas + "Lainnya" (palet `--chart-1..5`, Lainnya `--muted-foreground`) dengan kategori terbesar di tengah donut, legenda nama + nominal `num`. Judul → Laporan. Disembunyikan bila belum ada pengeluaran (kolom ini ikut hilang).
- **Budget perlu dilihat**: maks 5, persen tertinggi, `BudgetBar`. Bila semua level `ok`, deskripsi kartu "Semua budget masih aman.". Disembunyikan bila belum ada budget (ajakannya sudah ada di Budget periode ini).
- **Komposisi kekayaan**: `DonutChart` porsi aset per jenis akun (saringan sama dengan net worth Rust: akun `includeInNetWorth`, termasuk arsip, tanpa liabilitas), legenda jenis + nominal + persen; warna tetap per jenis (Bank `--chart-1`, E-wallet `--chart-2`, Investasi `--chart-3`, Tunai `--chart-4`, Piutang `--chart-5`). Di bawahnya Aset, Liabilitas, Kekayaan bersih. Kartu sempit: donut di atas legenda (`@container/card`). Disembunyikan bila aset ≤ 0. Judul → Akun.
- **Terjadwal jatuh tempo** dan **Ringkasan alokasi**: seperti sebelumnya (maks 5 / bucket root).
- Data dimuat ulang setelah aksi tulis (`ledgerVersion` di store transaksi).

### 8.2 Transaksi

Tanpa PageHeader. Toolbar filter (§7.6) di atas, menempel saat daftar di-scroll, lalu isi yang mengisi sisa tinggi inset. Tidak ada kotak ber-border di sekeliling isi.

Dua tampilan, dipilih lewat ToggleGroup outline ikon `list` (**Riwayat**) / `table-2` (**Tabel**) di ujung kanan toolbar, sesudah jumlah hasil. Pilihan dan kolom tersembunyi disimpan per perangkat (`localStorage`), bukan setting backend.

- **Riwayat** (default): daftar virtual §7.5 dikelompokkan per tanggal; header tanggal aktif menempel di atas saat scroll. Selalu urut tanggal terbaru (kembali ke Riwayat mereset sortir).
- **Tabel**: tabel datar §7.13 bergaya "Transaksi terakhir" Dasbor. Kolom: Tanggal ↕ · Deskripsi ↕ (tile 24px + judul, sortir = pihak/catatan, kosong di akhir) · Kategori · Akun (asal → tujuan, ≥ `@3xl/main`) · Tag (≥ `@5xl/main`) · Nominal ↕ (bertanda, rata kanan) · Saldo (hanya saat filter satu akun **dan** urut tanggal) · ikon lampiran/rekonsiliasi · `⋯`. Kategori, Akun, Tag bisa disembunyikan lewat menu **Kolom** (muncul di toolbar hanya di Tabel).
- Filter cepat di Tabel: klik nama kategori/akun/`#tag` di sel menerapkan filter itu (chip muncul di toolbar) tanpa membuka sheet edit. Klik di bagian lain baris → sheet edit; keyboard sama dengan Riwayat.
- Sortir bukan filter: tidak dihitung di jumlah filter dan tidak ikut "Hapus semua".

### 8.3 Akun

Tabel datar per grup. Grup berupa header baris dengan subtotal di kanan. Kolom: tile ikon jenis, nama + jenis (meta), saldo (kanan, `num`), `⋯`. Akun terarsip di seksi terlipat "Diarsipkan (n)".

- Piutang/Hutang: meta menampilkan arah dari tanda saldo ("Piutang" positif, "Hutang" negatif, "Lunas" 0), saldo tanpa tanda minus (Hutang tetap berwarna Keluar). Nama kontak menyusul di meta.
- Meta memuat "Direkonsiliasi s.d. {tanggal}" bila pernah direkonsiliasi. Bagian meta dipisah ` · `.
- Menu `⋯`: "Rekonsiliasi…" untuk semua jenis kecuali investasi ("Perbarui nilai…"). Dialog rekonsiliasi menampilkan saldo tercatat per tanggal terpilih, input saldo riil (utang diketik positif) dan tanggal; hasil lewat toast.
- Form Piutang/Hutang: input Kontak (saran dari kontak yang ada) dan segmented control arah "Mereka berutang ke saya | Saya berutang ke mereka".

### 8.4 Kategori

Segmented control **Pengeluaran | Pemasukan** di PageHeader. Daftar pohon: induk 48px, anak menjorok 24px, aksi `+ sub` dan `⋯` saat hover. Kategori terarsip di seksi terlipat.

### 8.5 Tag & Payee

Segmented control **Tag | Payee** di PageHeader (tab di query `?tab=payee`) + tombol `+ Tag baru` / `+ Payee baru`. Baris 48px: nama (tag berawalan `#`), meta jumlah transaksi (payee: `· kategori default`), `⋯` berisi Ubah, Gabungkan…, Lihat transaksi, Hapus. "Lihat transaksi" mereset filter lain lalu membuka Transaksi dengan chip `#tag` / `Payee: nama`. Hapus memakai dialog yang menyebut jumlah transaksi terdampak (tanpa undo).

### 8.6 Terjadwal

PageHeader dengan tombol `+ Jadwal baru`. Dua seksi berjudul kecil (meta, pudar):

- **Jatuh tempo** (hanya bila ada): baris 48px per kejadian, urut tanggal — tanggal, nama, akun, nominal bertanda. Kejadian tertua tiap jadwal punya `Catat` (utama kecil), `Lewati` (hantu) dan `⋯` (Edit lalu catat…, Konfirmasi semua (n) bila n > 1). Kejadian berikutnya dari jadwal yang sama nonaktif dengan tooltip "Tangani kejadian sebelumnya dulu".
- **Jadwal**: nama + deskripsi pengulangan ("Tiap bulan, tgl 25"), badge `Otomatis` / `Selesai`, "Berikutnya: tanggal", nominal, `⋯` (Ubah, Hapus).

Nama jadwal = payee → kategori pertama → catatan → jenis. Item sidebar Terjadwal memakai badge jumlah jatuh tempo. Form jadwal memakai sheet kanan yang sama dengan form transaksi (`RuleSheet`) + seksi pengulangan (Frekuensi, Setiap n, Berakhir, switch Catat otomatis, pratinjau 3 tanggal). "Jadikan berulang…" ada di menu `⋯` baris transaksi.

### 8.7 Budget

```
 ‹ Okt 2026 ›                                              [Salin dari bulan lalu]
 ⊘ 1 kategori terlampaui   △ 1 kategori hampir habis
 Kategori            Budget    Bawaan   Terpakai      Sisa  Per hari
 Makan & Minum    1.500.000  +120.000  1.290.000   330.000    33.000  ▓▓▓▓▓▓▓░ 79%  ⋮
 ─────────────────────────────────────────────────────────────────────────────────
 Total            2.300.000   +70.000  2.120.000   250.000    25.000
```

- Navigasi periode dan aksi di PageHeader. Tabel datar, kolom angka rata kanan `num`. Nominal budget bisa diedit di sel (klik/Enter, `Esc` batal, `Tab` ke baris berikutnya).
- Sisa negatif berwarna Bahaya dengan `−`. **Bawaan** (rollover) bertanda; `—` bila tidak ada. Sisa = budget + bawaan − terpakai; persen dari budget + bawaan.
- **Per hari** hanya untuk periode berjalan: sisa ÷ hari tersisa (termasuk hari ini), tidak pernah negatif.
- Rollover per kategori × periode lewat menu baris (`⋮`); hanya pada baris ber-budget.
- Ringkasan Waspada/Bahaya: ikon + teks.
- Saat transaksi membuat budget melewati 80%/100%: toast peringatan (satu per kategori, tidak berulang selama level tidak naik).

### 8.8 Alokasi & Laporan

- Alokasi: tabel target (`target_bp` sebagai persen) vs porsi aktual dengan batang ganda tipis (target = garis penanda, aktual = isi).
- Target bucket bisa persen atau nominal (CAP-01). Dialog bucket memakai ToggleGroup outline kecil **Persen / Nominal**; mode Nominal memakai `AmountInput` (parse saat blur/submit) dan deskripsi dialog berganti ke penjelasan "nominal dipenuhi dulu, persen membagi sisanya".
- Kolom Target di tabel selalu persen efektif terhadap total aset; bucket nominal menampilkan baris keterangan `text-meta` muted "Nominal tetap Rp…" di bawah nama (menggantikan keterangan "x% dari parent").
- Peringatan di atas tabel (`text-warning`, `role="status"`): Σ target persen per tingkat ≠ 100% (hanya bila tingkat itu punya bucket persen) dan Σ target nominal melebihi basisnya (total aset atau target parent). Peringatan tidak memblokir penyimpanan.
- Laporan: chart sesuai §7.12, setiap chart punya tabel angka di bawahnya.
- **Sumbu Y**: tick dibulatkan ke integer tanpa label kembar (`charts/axis.ts`); tanpa data hanya tick 0.
- **Tooltip chart** (`ChartTooltip`, semua chart SVG): kotak `bg-popover` border `shadow-md` `text-meta`, baris titik warna seri + label muted + nilai `num`. Garis/batang: ditambatkan ke titik data dan terbuka menjauhi tepi chart. Donut: ditambatkan di tepi kanan donut setinggi segmen, terbuka ke kanan (arah legenda). Tooltip yang tetap melewati tepi kartu digeser masuk (jarak 8px), jadi tidak pernah tumpah ke kartu tetangga. Nilai memakai nominal penuh (`formatIdr`, prop `valueFormat`), bukan format ringkas sumbu.
  - Garis: hover/fokus memilih periode terdekat → garis kursor putus-putus + titik per seri + tooltip semua seri. Chart bisa difokus (Tab); ←/→, Home/End memindah, Escape menutup.
  - Batang: hover/fokus kelompok → tooltip; kelompok lain opasitas 0.4.
  - Donut: segmen aktif menonjol (jari-jari luar +4) dan lainnya opasitas 0.4; tooltip nama, nominal, persen hanya saat donut sendiri di-hover/difokus. Hover baris legenda cukup menyorot segmen yang sama (`v-model:active`, tanpa tooltip karena angkanya sudah di legenda) dan meredupkan baris legenda lain ke 50%. Fokus + panah memutar segmen.

### 8.9 Onboarding & Kunci

Layar `bare`: tanpa sidebar dan SiteHeader. Satu kolom 440px di tengah layar. Langkah bernomor boleh dipakai di Onboarding. `Select` untuk bahasa dan tanggal periode ("Tanggal 25 setiap bulan"). Tombol utama tetap aktif; validasi muncul di field.

### 8.10 Inbox (email capture)

```
 Inbox                                   [Sync sekarang] [⋯]   ← ⋯: Abaikan sampai tanggal…
 [ Perlu ditinjau  12 | Gagal dibaca  2 | Rule  8 ]            ← Tabs
 9 Mei 2026
 [✉] IDM INDOMARET D                  −Rp 41.700  [Catat] [Tinjau] ⋯
     BCA Pembayaran (QRIS/VA)   BCA   Makan & Minum
 [✉] MUHAMMAD NUR  (Mungkin duplikat)   −Rp 10.000          [Tinjau] ⋯
     BCA Transfer   BCA   Pilih kategori                       ← field kosong: teks warning
```

- Judul halaman di SiteHeader; PageHeader hanya berisi aksi (`Sync sekarang` outline bila email aktif, `⋯` dengan "Abaikan sampai tanggal…").
- Baris dikelompokkan per tanggal (header 32px, pola §7.5); tile ikon amplop 28px di latar Kanvas. Baris 1: payee, atau nama rule bila payee kosong. Baris 2 `text-meta`: nama rule, akun, kategori/akun tujuan. Referensi kosong/arsip ditulis "Pilih akun"/"Pilih kategori" berwarna warning.
- Nominal bertanda dan berwarna semantik; transfer netral.
- Badge `Mungkin duplikat` (garis warning) adalah tombol yang membuka dialog duplikat.
- `Catat` (utama, kecil) hanya bila draft siap dicatat sekali klik: akun & kategori/tujuan aktif dan tanpa kandidat duplikat. `Tinjau` (outline, kecil) selalu ada. `⋯` berisi `Abaikan`, plus `Hubungkan ke transaksi…` bila ada duplikat.
- Abaikan (satu maupun massal) memunculkan toast dengan aksi `Urungkan`.
- `Tinjau` membuka sheet kanan "Tinjau transaksi" berisi form transaksi yang sama (§7.4), terisi dari draft. Di atas form: kotak warning "Mirip dengan transaksi yang sudah ada" berisi kandidat (tanggal, payee, akun, nominal, `Hubungkan`) bila ada, lalu "Email asli" yang tertutup secara default. Isian form tidak tertimpa saat daftar dimuat ulang. Badge duplikat membuka dialog "Kemungkinan duplikat" dengan daftar kandidat yang sama.
- Tab Gagal dibaca: baris subjek ("(tanpa subjek)" bila kosong) + pengirim + waktu terima; klik membuka cuplikan isi. Baris yang pengirimnya terbaca punya tombol `Buat rule` (outline, kecil) yang membuka sheet rule terisi saran dari email itu, dengan panel uji memakai email yang sama.

Tab Rule (`/inbox?tab=rules`; tab tersimpan di query, Pengaturan → Email punya link `Kelola rule`):

```
 [ Perlu ditinjau 12 | Gagal dibaca 2 | Rule 8 ]                [+ Rule baru]
 BCA Pembayaran (QRIS/VA)                               [●] ⋯
     bca.co.id · Pengeluaran · BCA · Tanpa kategori        ← kosong: warning
 Jago Terima Uang                    (Nonaktif)          [○] ⋯
     jago.com · Transfer masuk · Tanpa akun
```

- Urut prioritas (tertinggi dulu). Baris 48px: nama (tombol, membuka sheet ubah) + badge `Nonaktif` (outline); baris 2 `text-meta`: pengirim, jenis, akun, kategori default/akun lawan. Referensi kosong/arsip ditulis "Tanpa akun"/"Tanpa kategori" berwarna warning. Switch aktif di kanan, lalu `⋯` (Ubah, Duplikat, Hapus).
- Hapus lewat dialog konfirmasi yang menyebut jumlah item Perlu ditinjau dari rule itu (item tetap ada), lalu toast `Urungkan`. Simpan/aktifkan yang memindahkan email gagal dibaca memunculkan toast "Rule disimpan · n email dipindah ke Perlu ditinjau".
- Sheet rule: sheet kanan 440px (pola §7.4). Field: Nama, Jenis (select 4 jenis), Pengirim (+ hint), Akun, Akun lawan (hanya transfer) atau Kategori default (hanya pemasukan/pengeluaran; kategori beda jenis dikosongkan saat jenis diganti), kotak lipat `Pola` (subjek, nominal, payee, tanggal dalam font mono + format tanggal, dengan hint regex), Prioritas + switch Aktif. `Pola` terbuka untuk rule baru/dari email, tertutup saat mengubah; error pola membukanya. Error validasi tampil di bawah field (`text-destructive`, `aria-invalid`), sheet tetap terbuka.
- Panel "Uji rule" di bawah form (kotak bergaris): tab `Email gagal dibaca` (select) / `Tempel email` (pengirim, subjek, isi; tidak disimpan), tombol `Uji` outline kecil. Hasil: daftar langkah (Pengirim cocok, Subjek cocok, Nominal ditemukan, Payee, Tanggal terbaca) dengan ikon ✓ (income) / ✗ (destructive) sampai langkah yang gagal, lalu alasan gagal atau hasil (tanggal, payee, nominal bertanda), dan "Cocok dengan n email di Gagal dibaca".
- Email capture belum aktif dan tidak ada item: empty state dengan link `Buka Pengaturan`. Item sidebar dan palet perintah hanya muncul bila email aktif atau masih ada item.

## 9. Gerak

- Hanya gerak yang menjawab aksi pengguna: sheet/dialog/popover masuk 160ms ease-out, keluar 120ms; baris dihapus collapse 160ms; sidebar ciut/buka 200ms linear (bawaan shadcn).
- Tidak ada animasi masuk halaman, count-up angka, atau hover yang mengangkat elemen.
- `prefers-reduced-motion: reduce` mematikan semua transisi kecuali opasitas.

## 10. Ikon

- Lucide (`@lucide/vue`), 16px di baris/tombol/sidebar, stroke bawaan.
- Ikon tanpa label wajib punya `aria-label` dan tooltip (termasuk item sidebar saat mode ikon).
- Satu makna = satu ikon di seluruh aplikasi:

| Makna | Ikon |
|---|---|
| pengeluaran | `arrow-up-right` |
| pemasukan | `arrow-down-left` |
| transfer | `arrow-left-right` |
| saldo awal/penyesuaian | `flag` |
| arsip | `archive` |
| tren naik / turun | `trending-up` / `trending-down` |
| Dasbor | `layout-dashboard` |
| Transaksi | `receipt-text` |
| Budget | `wallet` |
| Alokasi | `chart-pie` |
| Laporan | `chart-column` |
| Akun | `landmark` |
| Kategori | `shapes` |
| Tag & Payee | `tags` |
| Terjadwal | `calendar-clock` |
| Pengaturan | `settings` |
| Cari / palette | `search` |
| Kunci | `lock` |

## 11. Teks & bahasa

- Bahasa Indonesia baku santai, sentence case ("Akun baru"). Hindari "Anda" bila kalimat bisa tanpa subjek.
- Satu aksi, satu nama di seluruh alur: "Arsipkan" → "Akun diarsipkan" → seksi "Diarsipkan".
- Error menjelaskan masalah dan perbaikannya, tanpa minta maaf.
- Angka format id-ID (`Rp 4.343.000`, spasi tak-putus), tanggal `17 Okt 2026`, minus `−` (U+2212).
- "Split" → **"Bagi"**. "Tag" tetap.
- Hindari pola template: label `KATA — fragmen`, deret dipisah titik tengah, panah `→` di akhir tombol, eyebrow di atas judul.

## 12. Aksesibilitas (lantai minimum)

- Kontras teks ≥ 4.5:1, elemen UI ≥ 3:1, di light dan dark. Jangan pudarkan teks dengan opacity (`text-current/75`, `opacity-65`); pakai `text-muted-foreground`.
- Semua aksi bisa dilakukan dengan keyboard; fokus selalu terlihat (ring Tunas **penuh**: `ring-ring`, bukan `ring-ring/50` yang hanya ~2.3:1). Setelah hapus baris, fokus pindah ke baris berikutnya.
- Ukuran teks bisa diubah (Pengaturan → Ukuran teks, atau Ctrl/⌘ `+` / `-` / `0`): 90, 100, 110, 125, 150%, lewat `font-size` persen di `<html>`. Karena itu ukuran pakai rem, bukan px (`max-w-[27.5rem]`, bukan `max-w-[440px]`); nilai yang dihitung di JS (tinggi baris virtual, teks sumbu chart) dikalikan `useRootFontScale()`. Pada 150% dan jendela 960px halaman tidak boleh scroll horizontal dan judul header tidak boleh terpotong; tabel data dengan kolom tetap (budget, laporan) dibungkus `TableScroll` (`role=region` berlabel, bisa difokus bila memang meluap) sehingga hanya tabel itu yang di-scroll (tabel dikecualikan dari reflow, WCAG 1.4.10). Toast yang digeser di samping sheet tetap di dalam jendela.
- ARIA: `role="row"` hanya berisi `cell`/`columnheader`; `svg role="img"` tidak boleh berisi elemen yang bisa difokus (pakai `role="group"`); progressbar wajib punya `aria-label`.
- Audit otomatis: `ui/e2e/a11y.spec.ts` menjalankan axe (`wcag2a`, `wcag2aa`, `wcag21aa`) di semua layar, light dan dark, termasuk sheet, dialog konfirmasi, toast, dan tooltip chart; harus 0 pelanggaran. Pengecualian hanya lewat `.exclude()` per selektor dengan alasan, tanpa `disableRules`.
- Hanya satu `h1` per layar (di SiteHeader).
- Target klik minimal 28×28px.
- Warna tidak pernah jadi satu-satunya pembawa informasi (§3.2).
