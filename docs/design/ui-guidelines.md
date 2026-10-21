# Nabungo — Panduan desain UI

Dokumen ini adalah acuan visual & interaksi untuk semua layar Nabungo. Setiap PR UI (mulai Fase 5) dicek terhadap dokumen ini. Requirement fungsional tetap di `PRD.md`; dokumen ini hanya mengatur *bagaimana tampilannya*.

## 1. Subjek, pengguna, tugas utama

- **Subjek:** buku kas pribadi orang Indonesia: rekening bank, e-wallet, dompet tunai, siklus gajian.
- **Pengguna:** satu orang, di laptop/desktop, membuka aplikasi beberapa kali seminggu untuk mencatat dan mengecek "masih aman nggak sampai gajian berikutnya".
- **Tugas utama layar:** mencatat transaksi secepat mungkin (keyboard), lalu membaca angka dengan yakin.

Konsekuensinya: angka adalah konten utama, bukan dekorasi. UI harus padat tapi tenang, mirip buku kas yang rapi, bukan dasbor SaaS.

## 2. Kritik UI saat ini (Oktober 2026)

Diambil dari screenshot 1280×800 setelah Fase 4.

| Masalah | Di mana | Dampak |
|---|---|---|
| Tidak ada kerangka halaman yang konsisten: Dasbor & Transaksi tanpa judul, Akun & Kategori punya judul dan konten di-center (`max-w` ~768px, mulai x=384), Transaksi full-bleed dalam kotak ber-border | semua layar | Tiap halaman terasa dibuat terpisah |
| Dasbor hanya satu kartu kecil di pojok kiri atas, sisanya kosong | Dasbor | Halaman pertama tidak menjawab apa pun |
| Tab Kategori vertikal melar setinggi daftar (kotak "Pengeluaran" ~260px) | Kategori | Terlihat rusak |
| Input tanggal native menampilkan `mm/dd/yyyy` (format AS) | form transaksi, filter | Salah lokal untuk pengguna Indonesia |
| Filter transaksi berupa 9 kontrol dalam 2 baris dengan lebar acak | Transaksi | Beban visual tinggi, padahal filter jarang dipakai |
| "Split" berupa teks kecil tanpa affordance; pesan error muncul di bawah form, jauh dari field | form transaksi | Mudah terlewat |
| Satu aksen hitam pekat untuk semua tombol utama, ikon arah transaksi abu-abu semua | global | Tidak ada hierarki; income/expense hanya dibedakan warna angka |
| Petunjuk shortcut ditulis dengan gaya berbeda-beda (`G D` teks abu-abu, `N` dalam kotak, `Ctrl+Enter` monospace) | sidebar, tombol simpan | Tidak konsisten |
| Toast menumpuk tiga lapis di pojok kanan bawah | global | Bising |
| Dark mode memakai hitam murni di area konten | dark | Kontras berlebihan, panel tidak terbaca sebagai lapisan |

## 3. Arah visual: "Zamrud"

Satu ide yang dibawa ke seluruh aplikasi: **koin zamrud yang masuk ke celah tabungan, di atas kertas buku kas yang dingin dan bersih**. Zamrud (emerald) adalah warna logo Nabungo (`docs/design/brand/`) dan diambil dari tema emerald shadcn (skala emerald Tailwind). Warna ini dipakai hemat, hanya untuk aksi utama dan fokus. Semua yang lain netral, dan angka yang bicara.

**Satu hal yang berani: Garis periode.** Strip horizontal yang menggambarkan periode budget berjalan (dari tanggal mulai periode, misalnya tanggal gajian, sampai periode berikutnya). Strip ini memuat penanda "hari ini" dan porsi uang yang sudah keluar. Ini elemen pengenal Nabungo dan muncul di Dasbor serta header Budget. Selain elemen ini, semuanya sunyi.

### Keputusan yang direvisi saat review (dan alasannya)

- *Draf awal:* Dasbor dibuka dengan angka kekayaan bersih besar di kartu, plus label kecil. → **Diganti** dengan Garis periode. Kartu "angka besar + label" adalah pola default dasbor mana pun, dan pertanyaan pengguna Indonesia sehari-hari adalah "sisa sampai gajian", bukan total aset.
- *Draf awal:* tetap Inter. → **Diganti** Plus Jakarta Sans. Inter adalah default shadcn; Plus Jakarta Sans dirancang Tokotype (foundry Indonesia) untuk identitas kota Jakarta, punya figur tabular (`tnum`, sudah diverifikasi), dan tersedia di `@fontsource-variable/plus-jakarta-sans`.
- *Draf awal:* label shortcut & data kecil memakai monospace. → **Dihapus.** Mono untuk label adalah ciri template. Shortcut memakai komponen `Kbd` dengan font yang sama.
- *Draf awal:* tiap jenis akun diberi warna uang kertas berbeda. → **Ditolak**, terlalu ramai dan bentrok dengan warna semantik income/expense.
- *Draf awal:* semua blok konten dibungkus kartu. → **Diganti** dengan permukaan datar + garis pemisah. Panel hanya dipakai bila isinya memang satu kelompok (lihat §7.3).

## 4. Warna

Token didefinisikan sebagai CSS variable di `ui/src/style.css` (`:root` dan `.dark`), lalu dipetakan ke nama shadcn yang sudah ada. Komponen **tidak boleh** memakai warna Tailwind mentah (`text-red-600`, `bg-gray-100`); selalu pakai token.

### 4.1 Palet dasar

| Nama | Peran | Light | Dark | Token shadcn |
|---|---|---|---|---|
| **Tinta** | teks utama | `#17202A` | `#E6E9EE` | `--foreground` |
| **Tinta pudar** | teks sekunder, meta | `#5B6472` | `#98A1AE` | `--muted-foreground` |
| **Kanvas** | sidebar, hover, latar muted | `#F4F6F8` | `#12161C` | `--sidebar`, `--muted`, `--secondary`, `--accent` (light) |
| **Lembar** | area konten, sheet, dialog | `#FFFFFF` | `#1A1F27` | `--background`, `--card`, `--popover` |
| **Garis** | border & pemisah | `#E1E5EA` | `#2A313B` | `--border` (border input sedikit lebih gelap: `--input` `#CFD5DC` / `#3A434F`) |
| **Zamrud** | aksi utama, fokus, seleksi | `#047857` (emerald-700) | `#34D399` (emerald-400) | `--primary`, `--ring` |

Teks di atas Zamrud memakai `#FFFFFF` (light) / `#022C22` (emerald-950, dark). Light memakai emerald-700, bukan emerald-600 default tema shadcn: emerald-600 dengan teks putih hanya 3.8:1.

### 4.2 Warna semantik

| Nama | Arti | Light | Dark |
|---|---|---|---|
| **Masuk** | income, sisa budget positif | `#1E7A4F` | `#5CC38E` |
| **Keluar** | expense | `#B4382F` | `#F08A80` |
| **Pindah** | transfer | `#5B6472` (= Tinta pudar) | `#98A1AE` |
| **Waspada** | budget ≥ 80%, peringatan | `#9A5B00` | `#E5A84B` |
| **Bahaya** | destruktif, error, budget lewat | `#B4382F` (= Keluar) | `#F08A80` |

Semua pasangan teks/latar di atas lolos kontras ≥ 4.5:1 di Lembar dan Kanvas. Cek ulang bila mengubah nilai.

**Aturan warna semantik**

- Warna income/expense **tidak pernah jadi satu-satunya pembeda**. Selalu ada tanda (`+` / `−` U+2212) atau ikon. Pengguna buta warna merah-hijau harus tetap bisa membaca.
- Warna hanya pada **angka**, bukan latar baris atau seluruh teks baris.
- Total, saldo, dan kekayaan bersih memakai **Tinta**, bukan hijau/merah. Saldo negatif memakai Keluar.
- Zamrud tidak dipakai untuk angka (Masuk juga hijau; Zamrud di angka akan terbaca sebagai income).

### 4.3 Warna kategori

Kategori boleh punya warna sendiri (dari `IconColorPicker`). Warna itu hanya muncul sebagai **tile ikon** 28px, dengan latar warna kategori 14% alfa dan ikon warna penuh. Warna kategori tidak dipakai untuk teks atau batang chart utama, kecuali di laporan per kategori.

## 5. Tipografi

**Satu keluarga: Plus Jakarta Sans** (variable, dibundel via `@fontsource-variable/plus-jakarta-sans`; hapus `@fontsource-variable/inter`). Tidak ada font kedua.

Basis 14px karena ini aplikasi desktop yang padat. Skala mengikuti rasio ~1.2:

| Token | Ukuran / line-height | Berat | Pemakaian |
|---|---|---|---|
| `text-display` | 32 / 38 | 600 | angka utama Garis periode (hanya satu per layar) |
| `text-title` | 22 / 28 | 600 | judul halaman |
| `text-heading` | 16 / 24 | 600 | judul seksi, judul sheet/dialog |
| `text-body` | 14 / 20 | 400 / 500 | teks dasar, isi baris, input |
| `text-meta` | 12.5 / 16 | 400 | meta baris, helper text, header tanggal |

**Aturan**

- **Angka uang selalu `tabular-nums`** dan rata kanan di kolom. Buat utilitas `.num` (`font-variant-numeric: tabular-nums; font-feature-settings: "tnum"`), dipakai `MoneyText`.
- Prefiks `Rp` diset di berat 400 dengan opasitas 65% dari warna angkanya; angkanya di berat 500/600. Contoh: `Rp` pudar, `4.343.000` tegas. Komponen: `MoneyText`. Hierarki ini adalah satu-satunya "gaya" angka.
- Tidak ada huruf kapital semua, tidak ada eyebrow di atas judul, tidak ada letter-spacing lebar.
- Huruf tebal maksimal 600. Teks paragraf (deskripsi, empty state) maksimal ~65 karakter per baris.

## 6. Ruang, ukuran, sudut, bayangan

- **Grid 4px.** Jarak yang dipakai: 4, 8, 12, 16, 24, 32, 48. Tidak ada nilai lain (hindari `gap-5`, `p-7`, nilai arbitrer).
- **Tinggi kontrol:** 32px (input, select, tombol default, mengikuti kit reka-nova), 28px (tombol kecil di toolbar/baris, chip), 40px (field nominal di form transaksi), 20px (kbd).
- **Tinggi baris daftar:** 48px (dua baris teks: utama + meta). Baris header tanggal 32px.
- **Sudut, berhierarki:**
  - 6px untuk kontrol (input, tombol) dan menu melayang (popover, dropdown, select): token `rounded-lg`
  - 10px untuk panel, kartu, dialog: token `rounded-xl`
  - 12px hanya untuk sheet (sisi dalam): token `rounded-2xl`
  - penuh (pill) untuk chip/tag dan badge

  Jangan satu radius untuk semuanya.
- **Bayangan hanya untuk lapisan melayang:** popover, dropdown, dialog, sheet, toast. Panel di dalam halaman **tidak** berbayangan, cukup border Garis.

## 7. Layout

### 7.1 Kerangka aplikasi

```
┌──────────────┬──────────────────────────────────────────────────────┐
│ Sidebar 248  │  Header halaman (padding 32 kiri/kanan, 24 atas)     │
│ (Kanvas)     │  Judul ............................. [Sekunder][Utama]│
│              │  meta/ringkasan satu baris                           │
│              ├──────────────────────────────────────────────────────┤
│              │  Konten (Lembar)                                     │
│              │  rata kiri, max-width 1120, padding 32               │
│              │                                                      │
└──────────────┴──────────────────────────────────────────────────────┘
```

- Area konten memakai latar **Lembar**, sidebar memakai **Kanvas**. Bedanya halus tapi memisahkan navigasi dari isi tanpa border tebal.
- **Konten rata kiri**, bukan di-center, dengan `max-width: 1120px`. Di layar lebar, ruang sisa ada di kanan. Ini berlaku sama untuk semua halaman, termasuk Akun & Kategori yang sekarang di-center.
- Pengecualian: daftar Transaksi boleh melebar penuh (tanpa max-width) karena kolomnya banyak.
- Lebar minimum jendela 960px. Di bawah 1100px sidebar boleh diciutkan ke ikon (fitur `sidebar` shadcn sudah mendukung).

### 7.2 Header halaman (`PageHeader`, komponen baru)

Setiap halaman wajib memakainya. Isinya:

- **Judul** (`text-title`): satu kata atau frasa pendek, sama dengan label navigasi.
- **Ringkasan** opsional, satu baris `text-meta`, misalnya "Kekayaan bersih Rp 4.343.000".
- **Aksi** di kanan: maksimal satu tombol utama (Zamrud) dan dua tombol sekunder. Aksi lain masuk menu `⋯`.

Header tidak sticky, kecuali di Transaksi, di mana header + toolbar filter menempel saat daftar di-scroll.

### 7.3 Panel vs. permukaan datar

- **Default: datar.** Daftar, tabel, dan form langsung di atas Lembar dengan pemisah Garis 1px.
- **Panel** (border Garis, radius 10, tanpa bayangan) hanya untuk kelompok yang berdiri sendiri, misalnya satu grup akun atau satu blok ringkasan di Dasbor. Panel tidak boleh bersarang.
- Tidak ada gradient dekoratif, tidak ada latar berwarna untuk "menghias" seksi.

### 7.4 Sidebar

```
 ◓ nabungo                    ← wordmark (AppLogo): tanda koin Zamrud + teks
                                 huruf kecil, text-heading bold, Tinta
 [ + Transaksi baru      N ]  ← tombol utama Zamrud, lebar penuh
                                 (satu-satunya blok Zamrud di sidebar)
 Dasbor                   G D ← item 32px, Kbd di kanan saat hover/aktif saja
 Transaksi                G T
 Budget                   G B
 Alokasi                  G A
 Laporan                  G R

 Akun                         ← judul grup: text-meta, Tinta pudar, sentence case
   Kekayaan bersih   4.343.000  ← angka saja (tanpa "Rp" di sidebar), num
   ▾ Bank            4.343.000
     BCA             4.343.000

 Kelola
   Akun
   Kategori
   Tag & Payee
   Terjadwal
 ─────────────
 Pengaturan            Ctrl ,
```

- Item aktif: latar Lembar + teks Tinta 500 + garis 2px Zamrud di kiri. Bukan latar abu-abu blok.
- Petunjuk shortcut hanya muncul saat hover/fokus dan pada item aktif, agar sidebar tidak penuh huruf.
- Di sidebar, angka tanpa prefiks `Rp` (konteksnya sudah jelas) agar kolom angka sejajar dan tidak sesak.

## 8. Komponen

### 8.1 Tombol

| Varian | Pemakaian | Tampilan |
|---|---|---|
| **Utama** | satu aksi terpenting per layar/dialog | latar Zamrud, teks putih |
| **Sekunder** | aksi lain yang sejajar | border Garis, latar Lembar |
| **Hantu** | aksi di baris, toolbar, ikon | tanpa border, hover Kanvas |
| **Bahaya** | hapus permanen, gabung kategori | latar Bahaya; hanya di dialog konfirmasi |

- Label tombol berupa kata kerja yang menyebut hasilnya: "Simpan transaksi", "Arsipkan akun", bukan "OK"/"Submit".
- Petunjuk shortcut di dalam tombol memakai `Kbd` di sisi kanan, dengan opasitas 70%.

### 8.2 Kbd (komponen baru)

Satu komponen (`ui/kbd`) untuk semua petunjuk shortcut: tinggi 20px, padding 0 6px, radius 4, border Garis, `text-meta` 500, font yang sama dengan UI. Kombinasi ditulis per tombol: `Ctrl` `Enter`. `G D` ditulis sebagai dua `Kbd` berurutan.

### 8.3 Input & form

- Label di atas field, `text-body` 500. Helper/error di bawah field, `text-meta`. Error berwarna Bahaya dan **menempel di field yang salah**, bukan di bawah form.
- Field error: border Bahaya + teks error. Fokus: ring 2px Zamrud, offset 2px (berlaku untuk semua elemen interaktif).
- **Nominal:** rata kanan, `num`, ukuran 20px di form transaksi (field terpenting). Prefiks `Rp` di dalam field sebagai addon pudar.
- **Tanggal:** ganti `<input type="date">` dengan `DateInput` (Popover + Calendar shadcn). Tampilan `17 Okt 2026` sesuai locale, nilai tetap `YYYY-MM-DD`. Ketik manual tetap bisa (`17/10`, `kemarin`, `hari ini` boleh ditambahkan nanti).
- **Select:** selalu shadcn `Select`/`Combobox`, tidak ada `<select>` native.
- Lebar field mengikuti isi yang diharapkan: tanggal ~160px, nominal min/maks ~140px, bukan semua stretch.

### 8.4 Sheet transaksi

```
┌ Transaksi baru ──────────────────── ✕ ┐
│ [ Pengeluaran | Pemasukan | Transfer ] │  segmented control
│                                        │
│ Nominal                                │
│ Rp                    28.000           │  20px, rata kanan
│                                        │
│ Kategori                  [Bagi ▸]     │  tombol hantu "Bagi" = split
│ [ Makan & Minum › Kopi & jajan      ]  │
│                                        │
│ Akun                Tanggal            │  dua kolom
│ [ BCA          ▾ ]  [ 17 Okt 2026   ]  │
│                                        │
│ Toko / pihak                           │
│ Tag · Catatan  (terlipat: "+ Tambah    │  detail opsional
│                 tag atau catatan")     │
├────────────────────────────────────────┤
│                [Batal] [Simpan  Ctrl ⏎]│  footer menempel di bawah
└────────────────────────────────────────┘
```

- Lebar 440px. Footer aksi selalu terlihat (sticky), tidak ikut scroll.
- Urutan field mengikuti urutan Tab yang sudah diuji e2e: nominal → kategori → … Jangan diubah tanpa memperbarui test.
- Field opsional (tag, catatan, lampiran) terlipat secara default dan terbuka otomatis bila sudah berisi (mode edit, termasuk transaksi yang punya lampiran).
- Lampiran (Fase 7c) ada setelah catatan: daftar baris 36px (ikon gambar/dokumen, nama, ukuran KB/MB, `Buka`, hapus dengan konfirmasi) + tombol outline `Lampirkan file`. File yang hilang dari folder data ditandai "File hilang" dan `Buka` nonaktif. Saat membuat transaksi baru hanya tampil petunjuk "Simpan dulu untuk melampirkan struk".

### 8.5 Baris transaksi

```
 17 Okt 2026, Jumat                                  −Rp 662.000  ← header tanggal 32px:
                                                                    tanggal + net hari itu (Tinta pudar)
 [☕] Janji Jiwa                                   −Rp 22.000   ⋯
      Kopi & jajan   BCA   #kantor                  4.321.000   ← saldo berjalan, Tinta pudar
```

- Kiri: tile ikon kategori (28px, §4.3). Transfer memakai ikon panah dua arah di tile Kanvas. Saldo awal/penyesuaian memakai ikon bendera.
- Baris 1: payee (atau nama kategori bila payee kosong), `text-body` 500.
- Baris 2 (`text-meta`, Tinta pudar): kategori anak saja, dengan induk di tooltip; akun; tag sebagai teks `#tag`. Pemisah berupa jarak 12px, bukan `·` atau `›`.
- Kanan: nominal bertanda dengan warna semantik; di bawahnya saldo berjalan (bila filter satu akun).
- Bila filter satu akun, transaksi yang sudah direkonsiliasi diberi ikon lingkaran centang kecil (14px, Tinta pudar) di akhir baris 2, dengan label/tooltip "Sudah direkonsiliasi". Tanpa filter akun ikon tidak tampil.
- Transaksi berlampiran diberi ikon klip kecil (14px, Tinta pudar) di baris 2 sebelum ikon rekonsiliasi, dengan label "Ada lampiran".
- Menu `⋯` hanya terlihat saat hover/fokus baris. Baris bisa difokus (Tab/↑↓), dan `Enter` membuka edit, `Delete` menghapus.
- Hover: latar Kanvas. Terfokus: ring Zamrud di dalam baris.

### 8.6 Filter (Transaksi)

Satu baris toolbar, bukan dua:

```
 [🔍 Cari…            ]  [Periode: Okt 2026 ▾]  [Akun ▾]  [+ Filter]      12 transaksi
 chip aktif:  (Kategori: Kopi ✕) (≥ Rp 50.000 ✕)                     Hapus semua filter
```

- Selalu terlihat: cari, periode (preset: bulan ini, periode budget ini, 30 hari, kustom), akun.
- Kategori, tag, nominal min/maks ada di popover "+ Filter". Filter aktif tampil sebagai chip yang bisa dihapus.
- Jumlah hasil ditampilkan di kanan.

### 8.7 Empty state

Teks rata kiri di posisi konten pertama (bukan di tengah layar), dengan satu kalimat yang menjelaskan apa yang akan muncul dan satu tombol aksi:

> **Belum ada transaksi bulan ini.**
> Catat pengeluaran pertama dan saldo akun langsung ikut berubah.
> [Catat transaksi `N`]

Bila kosong karena filter: "Tidak ada transaksi yang cocok dengan filter ini." + [Hapus filter]. Tidak ada ilustrasi.

### 8.8 Toast

- Kanan bawah, **maksimal satu terlihat** (toast baru menggantikan yang lama).
- Format: hasil aksi dengan kata kerja yang sama dengan tombolnya ("Simpan transaksi" → "Transaksi disimpan"). Aksi undo sebagai tombol di toast: "Transaksi dihapus [Urungkan]".
- Error tidak memakai toast bila bisa ditempel ke field (§8.3).

### 8.9 Dialog konfirmasi

Judul berupa pertanyaan dengan objeknya ("Hapus akun Dompet?"), isi menjelaskan akibat dan alternatif ("Akun yang punya transaksi tidak bisa dihapus, hanya diarsipkan."). Tombol: [Batal] [aksi spesifik, varian Bahaya bila destruktif]. Fokus awal di Batal untuk aksi destruktif.

### 8.10 Angka besar & batang progres

- `MoneyText` punya ukuran `sm | md | lg | display`. `display` hanya untuk Garis periode.
- Batang progres budget: tinggi 6px, radius penuh, track Garis. Isi:
  - Tinta pudar bila < 80%
  - Waspada bila ≥ 80%
  - Bahaya bila ≥ 100% (atau tersedia ≤ 0 dengan realisasi); bagian yang lewat ditampilkan menonjol keluar dari track
  - Warna mengikuti `level` dari backend (`ok`/`warning`/`exceeded`), bukan dihitung ulang di UI

## 9. Pola layar

### 9.1 Dasbor

```
 Dasbor                                                  [+ Transaksi baru]
 Periode 25 Sep – 24 Okt

 ┌ Garis periode ─────────────────────────────────────────────────────────┐
 │ Sisa periode ini                                     7 hari lagi gajian │
 │ Rp 3.412.000                                                            │
 │ ├■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■│░░░░░░░░░░░░░░┤              │
 │ 25 Sep                               hari ini ▲             24 Okt      │
 │ Masuk Rp 8.500.000     Keluar Rp 5.088.000     Budget terpakai 62%      │
 └─────────────────────────────────────────────────────────────────────────┘

 Budget yang perlu dilihat          │  Transaksi terakhir
 Makan & Minum   ▓▓▓▓▓▓▓▓░ 86%      │  Janji Jiwa         −Rp 22.000
 Transportasi    ▓▓▓▓░░░░░ 41%      │  PLN               −Rp 412.000
 (maks 5, urut % tertinggi)         │  (5 terakhir, klik → Transaksi)

 Akun                                                    Kekayaan bersih
 BCA  4.343.000   Dompet 200.000   GoPay 54.000            Rp 4.597.000
```

- Garis periode: lebar penuh konten, panel. Batang menggambarkan **waktu** (tanggal awal → akhir), dengan isi Zamrud sampai hari ini. Teks di dalamnya membandingkan uang keluar terhadap uang masuk/budget. Sebelum Fase 5 (budget belum ada), tampilkan masuk vs keluar periode saja.
- Dua kolom di bawahnya (≥ 1100px), satu kolom di bawah itu.
- Kekayaan bersih turun ke baris akun, tidak lagi menjadi hero.

### 9.2 Transaksi

Header sticky (judul + toolbar §8.6), lalu daftar virtual §8.5. Tidak ada kotak ber-border di sekeliling daftar: daftar langsung di Lembar, header tanggal menempel saat scroll.

### 9.3 Akun

Tabel datar per grup (bukan kartu bersarang). Grup berupa header baris dengan subtotal di kanan. Kolom: tile ikon jenis, nama + jenis (meta), saldo (kanan, `num`), `⋯`. Akun terarsip di seksi terlipat "Diarsipkan (n)" di bawah.

- Akun Piutang/Hutang: meta menampilkan arah dari tanda saldo ("Piutang" bila positif, "Hutang" bila negatif, "Lunas" bila 0) menggantikan nama jenis, dan saldo tampil tanpa tanda minus (Hutang tetap berwarna Bata). Nama kontak menyusul di meta.
- Meta juga memuat "Direkonsiliasi s.d. {tanggal}" bila akun pernah direkonsiliasi. Bagian meta dipisah ` · `.
- Menu `⋯` berisi "Rekonsiliasi…" untuk semua jenis kecuali investasi (investasi memakai "Perbarui nilai…"). Dialog rekonsiliasi menampilkan saldo tercatat per tanggal terpilih (dibaca ulang saat tanggal diganti), input saldo riil (utang diketik positif) dan tanggal; hasilnya disampaikan lewat toast (selisih yang dicatat, atau saldo cocok).
- Form akun Piutang/Hutang punya input Kontak (saran dari kontak yang ada) dan segmented control arah "Mereka berutang ke saya | Saya berutang ke mereka".

### 9.4 Kategori

Segmented control horizontal **Pengeluaran | Pemasukan** di header (mengganti tab vertikal yang rusak). Daftar pohon: induk 48px, anak menjorok 24px, aksi `+ sub` dan `⋯` saat hover. Kategori terarsip di seksi terlipat.

### 9.4a Tag & Payee (Fase 7c)

Segmented control **Tag | Payee** di header (tab disimpan di query `?tab=payee`) + tombol `+ Tag baru` / `+ Payee baru`. Daftar baris 48px: nama (tag berawalan `#`), meta jumlah transaksi (payee: `· kategori default`), `⋯` saat hover berisi Ubah, Gabungkan…, Lihat transaksi, Hapus. "Lihat transaksi" mereset filter lain lalu membuka Transaksi dengan chip `#tag` / `Payee: nama`. Hapus memakai dialog konfirmasi yang menyebut jumlah transaksi terdampak (tanpa undo; saldo tidak berubah).

### 9.5 Budget (Fase 5)

```
 Budget                     ‹ Okt 2026 ›                  [Salin dari bulan lalu]
 ⊘ 1 kategori terlampaui   △ 1 kategori hampir habis
 Kategori            Budget    Bawaan   Terpakai      Sisa  Per hari
 Makan & Minum    1.500.000  +120.000  1.290.000   330.000    33.000  ▓▓▓▓▓▓▓░ 79%  ⋮
   Restoran         600.000         —    512.000    88.000     8.800  ▓▓▓▓▓▓▓░ 85%  ⋮
 Transportasi       800.000   −50.000    830.000   −80.000         0  ▓▓▓▓▓▓▓▓▓▓ 110% ⋮
 ─────────────────────────────────────────────────────────────────────────────────
 Total            2.300.000   +70.000  2.120.000   250.000    25.000
```

- Tabel dengan kolom angka rata kanan, `num`. Nominal budget bisa diedit langsung di sel (klik/Enter, `Esc` batal, `Tab` ke baris berikutnya).
- Sisa negatif berwarna Bahaya dengan tanda `−`.
- **Bawaan** (rollover): nominal bertanda (`+`/`−`, negatif berwarna Bahaya); `—` bila tidak ada yang dibawa. Sisa = budget + bawaan − terpakai; persen dan bar dihitung dari budget + bawaan.
- **Per hari** hanya tampil untuk periode berjalan: sisa ÷ hari tersisa (termasuk hari ini), tidak pernah negatif.
- Rollover dinyalakan per kategori × periode lewat menu baris (`⋮`, "Bawa sisa ke periode berikutnya"); menu hanya ada pada baris ber-budget.
- Ringkasan di bawah header muncul bila ada kategori Waspada/Bahaya: ikon + teks (bukan warna saja).
- Saat transaksi disimpan dan budget kategori baru melewati 80% atau 100%, tampilkan toast peringatan (satu per kategori; tidak berulang selama level tidak naik).

### 9.6 Alokasi & Laporan (Fase 5)

- Alokasi: tabel target (`target_bp` sebagai persen) vs porsi aktual dengan batang ganda tipis (target berupa garis penanda, aktual berupa isi).
- Laporan: chart memakai palet `--chart-1..5`: Zamrud, Biru (`#24508F` / `#8FB0E8`, agar irisan donut tidak dua hijau berdampingan), Keluar, Waspada, dan Tinta pudar. Seri yang bermakna masuk/keluar (cashflow) memakai `--income`/`--expense` langsung, bukan nomor chart. Tidak ada gradient atau 3D. Setiap chart punya tabel angka di bawahnya (aksesibilitas, dan pengguna keuangan memang membaca angka).

### 9.7 Onboarding

Satu kolom 440px, rata kiri di tengah layar. Langkah bernomor boleh dipakai di sini karena memang berurutan. Pakai `Select` shadcn untuk bahasa dan tanggal periode (tanggal 1–28 dengan teks "Tanggal 25 setiap bulan"). Tombol utama Zamrud tetap aktif; validasi muncul di field.

## 10. Gerak

- Hanya gerak yang menjawab aksi pengguna:
  - sheet/dialog/popover masuk: 160ms ease-out
  - keluar: 120ms
  - baris dihapus: collapse 160ms
- Tidak ada animasi masuk halaman, count-up angka, atau hover yang mengangkat elemen.
- Satu momen yang disengaja: penanda "hari ini" di Garis periode bergeser ke posisinya saat Dasbor pertama dibuka (300ms).
- `prefers-reduced-motion: reduce` mematikan semua transisi kecuali perubahan opasitas.

## 11. Ikon

- Lucide, 16px di baris/tombol, 18px di sidebar, stroke 1.75.
- Ikon tanpa label wajib punya `aria-label` dan tooltip.
- Satu makna = satu ikon di seluruh aplikasi:
  - pengeluaran `arrow-up-right`
  - pemasukan `arrow-down-left`
  - transfer `arrow-left-right`
  - saldo awal/penyesuaian `flag`
  - arsip `archive`

## 12. Teks & bahasa

- Bahasa Indonesia baku santai, sentence case ("Akun baru", bukan "Akun Baru"). Sapaan tidak langsung; hindari "Anda" bila kalimat bisa tanpa subjek.
- Satu aksi, satu nama di seluruh alur: tombol "Arsipkan" → toast "Akun diarsipkan" → seksi "Diarsipkan".
- Error menjelaskan masalah dan perbaikannya, tanpa minta maaf: "Nominal terlalu besar. Periksa jumlah digitnya.", bukan "Terjadi kesalahan".
- Angka: format id-ID (`Rp 4.343.000`, spasi tak-putus), tanggal `17 Okt 2026`, tanda minus `−` (U+2212).
- Hindari campuran Inggris bila ada padanan umum: "Split" → **"Bagi"**. "Payee" → **"Toko / pihak"** (perlu konfirmasi sebelum diganti di i18n). "Tag" tetap.
- Hindari pola template: label `KATA — fragmen`, deret dipisah titik tengah, panah `→` di akhir tombol, eyebrow di atas judul.

## 13. Aksesibilitas (lantai minimum)

- Kontras teks ≥ 4.5:1, elemen UI ≥ 3:1, di light dan dark.
- Semua aksi bisa dilakukan dengan keyboard. Fokus selalu terlihat (ring Zamrud). Setelah hapus baris, fokus pindah ke baris berikutnya.
- Target klik minimal 28×28px.
- Warna tidak pernah jadi satu-satunya pembawa informasi (§4.2).

## 14. Implementasi

Urutan yang disarankan, masing-masing satu PR kecil:

1. **Token & font.** Ganti nilai `:root`/`.dark` di `ui/src/style.css` sesuai §4. Tambahkan `--warning`, `--radius-control`/`--radius-panel`, dan skala teks §5 di `@theme inline`. Ganti Inter → Plus Jakarta Sans. Tambahkan utilitas `.num`.
2. **Primitif.**
   - Komponen baru: `PageHeader`, `Kbd`, `EmptyState`, `DateInput`.
   - `MoneyText` mendapat ukuran, prefiks `Rp` pudar, dan `−`.
   - Toast dibatasi satu (`visibleToasts: 1`).
3. **Kerangka.** Layout konten rata kiri max-width 1120 di `App.vue`/layout, sidebar sesuai §7.4. Semua view memakai `PageHeader`.
4. **Layar.** Transaksi (toolbar filter, baris, sheet), lalu Akun, Kategori (perbaiki tab), Onboarding.
5. **Dasbor + Garis periode.** Bisa dikerjakan bersama Fase 5 karena butuh data periode/budget.

Setiap langkah: perbarui test komponen yang bergantung pada teks/struktur, jalankan e2e, dan ambil screenshot light + dark untuk dibandingkan dengan dokumen ini.
