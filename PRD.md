# PRD — Aplikasi Keuangan Pribadi Desktop (Rust + SQLite)

Oct 12, 2026 · @Gilang Fatahilah

## 1. Ringkasan produk

Aplikasi desktop offline untuk mencatat seluruh keuangan pribadi — transaksi, saldo akun, budget, dan alokasi modal — dengan semua data tersimpan di satu file SQLite milik user. Working title: **Nabungo**.

**Problem.** Tracker berbasis Notion/spreadsheet fleksibel, tapi rawan inkonsistensi: saldo dihitung manual lewat formula, transfer antar akun tercatat dua kali atau tidak seimbang, dan budget sulit dibandingkan dengan realisasi. Aplikasi cloud menyelesaikan itu, tapi data finansial jadi tersimpan di server pihak lain.

**Prinsip produk**

- **Offline-first, local-only.** Tidak ada akun, login, atau network call wajib. App berjalan penuh tanpa internet. Fitur yang butuh jaringan (mis. auto-capture email, D13) hanya opt-in, mati secara default, dan tidak pernah jadi syarat app berjalan.
- **Data milik user.** Satu file `.db` yang bisa di-backup, dipindah, dan dibuka dengan tool SQLite apa pun; export CSV/JSON selalu tersedia.
- **Saldo selalu benar.** Saldo diturunkan dari transaksi (bukan diketik manual), dan setiap transfer tercatat sebagai satu kejadian yang seimbang.
- **Cepat untuk input harian.** Mencatat satu transaksi < 10 detik dengan keyboard saja.
- **Uang sebagai integer.** Semua nominal disimpan dalam satuan minor (integer), tidak pernah floating point.

## 2. Tujuan, non-goals, dan persona

**Tujuan v1**

1. User bisa melihat net worth dan saldo tiap akun yang akurat setiap saat.
2. User bisa membandingkan pengeluaran aktual vs budget per kategori per bulan.
3. User bisa melihat sebaran modalnya (mis. aset investasi 70% vs liquid cash 30%) dibanding target porsi yang ia tetapkan.
4. Seluruh data bisa di-backup, di-restore, dan di-export tanpa vendor lock-in.

**Non-goals v1**

- Sinkronisasi cloud / multi-device dan aplikasi mobile.
- Koneksi otomatis ke bank (open banking, scraping mutasi). Auto-capture dari email notifikasi transaksi (IMAP, opt-in, `EML-01`–`EML-04`, D13) bukan termasuk ini dan direncanakan di v2.
- Tracking harga pasar realtime untuk saham/reksa dana/kripto (nilai investasi di-update manual).
- Multi-user / shared household dengan permission.
- Fitur pajak dan pelaporan SPT.

**Persona utama**

| Persona                                                         | Kebutuhan                                  | Pain saat ini                              |
| --------------------------------------------------------------- | ------------------------------------------ | ------------------------------------------ |
| Profesional muda dengan banyak akun (bank, e-wallet, sekuritas) | Satu tempat untuk semua saldo, input cepat | Saldo tersebar, rekonsiliasi manual        |
| Investor ritel                                                  | Alokasi modal per kelas aset vs target     | Spreadsheet alokasi terpisah dari cashflow |
| Privacy-conscious user                                          | Data tidak keluar dari laptop              | App populer berbasis cloud                 |

**Use case utama**

1. Mencatat pengeluaran harian dari e-wallet atau kartu debit.
2. Mencatat gaji masuk lalu mentransfer sebagian ke tabungan atau rekening investasi.
3. Top-up e-wallet dari rekening bank (transfer dengan biaya admin).
4. Membayar tagihan kartu kredit (transfer dari bank ke akun liability).
5. Review bulanan: budget vs aktual, cashflow, dan progres alokasi modal.

## 3. Keputusan yang perlu dikonfirmasi

**D1 dan D6 sudah diputuskan**: Tauri 2 + Vue 3 + shadcn-vue, dan capital allocation sebagai tracker porsi modal saja. Semua keputusan D1–D13 sudah dikonfirmasi.

| #   | Keputusan                 | Opsi                                                                                                                                                                                                                         | Rekomendasi                                                                                                                                                                       | Status             |
| --- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| D1  | UI framework              | (a) **Tauri 2** — core Rust, UI web (Vue/TS); (b) **Slint** — native, deklaratif; (c) **egui / iced** — pure Rust immediate/Elm-style; (d) **Dioxus** — React-like, pure Rust                                                | Diputuskan: (a) Tauri 2 + Vue 3 + TypeScript, komponen UI dari shadcn-vue (Reka UI + Tailwind CSS v4)                                                                             | Setuju rekomendasi |
| D2  | Akses SQLite              | (a) `rusqlite` (sync, bundled); (b) `sqlx` (async, compile-time check); (c) ORM: Diesel / SeaORM                                                                                                                             | (a) `rusqlite` + `rusqlite_migration`: single-user, sync cukup, tanpa runtime async                                                                                               | Setuju rekomendasi |
| D3  | Model pencatatan          | (a) single-entry: transaksi punya from/to account; (b) double-entry penuh (akun income/expense juga ledger); (c) **header + postings** (double-entry ringan)                                                                 | (c): tiap transaksi punya ≥1 posting ke akun; transfer = 2 posting seimbang. Mendukung split, biaya admin, dan transfer beda mata uang tanpa mengekspos istilah akuntansi ke user | Setuju rekomendasi |
| D4  | Multi-currency            | (a) IDR saja; (b) currency per akun + kurs manual; (c) + kurs otomatis online                                                                                                                                                | Diputuskan: IDR saja di MVP dan v1; multi-currency + kurs manual di v2. Skema menyimpan `currency` dari hari pertama agar tidak perlu migrasi besar                               | Pilih opsi lain    |
| D5  | Model budget              | (a) limit per kategori per periode; (b) envelope / zero-based (ala YNAB); (c) keduanya                                                                                                                                       | (a) + opsi rollover sisa budget per kategori. Envelope ditunda ke v2 karena mengubah alur input                                                                                   | Setuju rekomendasi |
| D6  | Arti "capital allocation" | (a) **target alokasi** per bucket/kelas aset (mis. 40% saham, 20% emas) vs aktual + drift; (b) **earmark**: uang di akun dipecah virtual ke tujuan (dana darurat, DP rumah); (c) **aturan pembagian income** (mis. 50/30/20) | Diputuskan: (a) sebagai tracker read-only. Bucket dengan target %, akun dipetakan ke bucket, tampilkan porsi aktual vs target. Tanpa aturan pembagian income dan tanpa earmark    | Pilih opsi lain    |
| D7  | Nilai investasi           | (a) snapshot nilai manual per akun; (b) holdings: unit × harga, cost basis, P/L                                                                                                                                              | Diputuskan: (a) snapshot nilai manual bertanggal; (b) holdings di v2                                                                                                              | Setuju rekomendasi |
| D8  | Enkripsi database         | (a) tidak ada (andalkan disk encryption OS); (b) SQLCipher + master password                                                                                                                                                 | (b) sebagai opsi saat setup; default off di MVP, wajib tersedia di v1                                                                                                             | Setuju rekomendasi |
| D9  | Platform target           | Linux, Windows, macOS                                                                                                                                                                                                        | Linux (AppImage/.rpm/.deb) + Windows di v1; macOS butuh signing/notarization, tunda                                                                                               | Setuju rekomendasi |
| D10 | Hutang/piutang            | (a) cukup akun liability (kartu kredit, paylater); (b) + modul utang-piutang ke orang                                                                                                                                        | (a) di MVP; (b) di v1 sebagai tipe akun "Piutang/Hutang" dengan kontak                                                                                                            | Setuju rekomendasi |
| D11 | Import data lama          | (a) tidak ada; (b) CSV dengan column mapping; (c) importer khusus (Notion export, mutasi bank)                                                                                                                               | (b) di v1; (c) per sumber bila dibutuhkan                                                                                                                                         | Setuju rekomendasi |
| D12 | Bahasa UI                 | Indonesia, Inggris, keduanya                                                                                                                                                                                                 | Keduanya via i18n; default Indonesia, format angka `Rp 1.250.000`                                                                                                                 | Setuju rekomendasi |
| D13 | Auto-capture transaksi    | (a) tidak ada; (b) IMAP + App Password Gmail + rule engine lokal + review queue; (c) Gmail API + OAuth; (d) parsing berbasis ML/LLM                                                                                          | (b) di v2, opt-in dan mati default: credential hanya di OS keychain, rule tersimpan di DB, tidak ada transaksi tercatat tanpa konfirmasi user. Brief: `docs/plans/Brief Auto-Capture Transaksi via IMAP.md` | Setuju rekomendasi |

## 4. Konsep inti dan model domain

Semua angka di app diturunkan dari satu sumber kebenaran: **postings** (baris mutasi per akun). Saldo, laporan, budget realisasi, dan alokasi aktual hanyalah agregasi dari postings.

### 4.1 Account

Akun = tempat uang berada atau kewajiban yang dimiliki. Setiap akun punya **tipe** (ditentukan sistem, mengatur perilaku) dan **account group** (ditentukan user, untuk pengelompokan tampilan).

| Tipe akun       | Sifat     | Contoh                          | Catatan perilaku                                     |
| --------------- | --------- | ------------------------------- | ---------------------------------------------------- |
| Cash            | Aset      | Dompet, kas kecil               | —                                                    |
| Bank            | Aset      | BCA, Mandiri, Jago              | Mendukung rekonsiliasi saldo                         |
| E-wallet        | Aset      | GoPay, OVO, DANA                | Biasanya top-up dari bank                            |
| Investment      | Aset      | RDN sekuritas, reksa dana, emas | Nilai berubah lewat transaksi Valuation              |
| Receivable      | Aset      | Piutang ke teman                | v1 (D10)                                             |
| Credit card     | Liability | Kartu kredit                    | Saldo negatif = tagihan; punya limit & tanggal cetak |
| Loan / Paylater | Liability | KPR, cicilan, paylater          | Pembayaran = transfer dari aset                      |

Atribut utama: nama, tipe, group, currency, saldo awal + tanggal, ikon/warna, `include_in_net_worth`, status arsip. Akun yang sudah punya transaksi **di-arsip, tidak dihapus**.

> Catatan istilah: "account category" di brief ditafsirkan sebagai **account group** (pengelompokan akun buatan user, mis. "Harian", "Tabungan", "Investasi"). Kategori transaksi dibahas di 4.3.

### 4.2 Transaction

Satu transaksi = satu kejadian finansial dengan header (tanggal, payee, catatan, tag) dan ≥1 posting.

| Jenis            | Postings                                                                 | Efek                                       |
| ---------------- | ------------------------------------------------------------------------ | ------------------------------------------ |
| Income           | +X ke akun aset, dengan kategori income                                  | Saldo naik                                 |
| Expense          | −X dari akun, dengan kategori expense (boleh split ke beberapa kategori) | Saldo turun                                |
| Transfer         | −X dari akun A, +Y ke akun B (Y = X kecuali beda currency)               | Net worth tetap                            |
| Transfer + biaya | Transfer di atas + posting expense kategori "Biaya admin"                | Net worth turun sebesar biaya              |
| Adjustment       | Selisih antara saldo tercatat dan saldo riil                             | Hasil rekonsiliasi                         |
| Valuation        | Kenaikan/penurunan nilai akun Investment                                 | Unrealized gain/loss, tidak masuk cashflow |
| Opening balance  | Saldo awal akun                                                          | Dibuat otomatis saat akun dibuat           |

Aturan: **refund** dicatat sebagai expense bernilai negatif di kategori yang sama, sehingga mengurangi realisasi budget. Transfer tidak pernah dihitung sebagai income atau expense.

### 4.3 Category

Kategori transaksi berbentuk pohon 2 level (parent → child), dipisah menjadi kategori **Income** dan **Expense**. App menyediakan set default (Makan & Minum, Transportasi, Tagihan, Belanja, Kesehatan, Hiburan, Gaji, Bonus, dll.) yang bisa diubah. Kategori yang terpakai bisa di-merge atau di-arsip, tidak dihapus.

### 4.4 Budget

Budget = limit nominal per kategori expense per periode (bulanan default; mingguan/tahunan opsional). Realisasi = jumlah posting expense di kategori itu (termasuk child) dalam periode. Opsi per kategori: **rollover** sisa atau kelebihan ke periode berikutnya. Budget bisa disalin dari bulan sebelumnya.

### 4.5 Capital allocation

Capital allocation adalah **tracker read-only** untuk melihat sebaran modal: berapa porsi tiap pos dibanding target yang kamu tetapkan. Fitur ini tidak membuat transaksi apa pun dan tidak memberi saran rebalancing.

- **Allocation bucket** — pos dengan target porsi, mis. Aset investasi 70% dan Liquid cash 30%. Struktur bucket sepenuhnya ditentukan user dan bisa bertingkat. Target child bersifat **relatif terhadap parent** (Aset 70% → Saham 60% dari Aset = 42% dari total). Total target sesama level = 100%.
- **Target nominal** — bucket boleh bertarget nominal tetap (mis. Dana darurat Rp 30 jt). Di setiap level, target nominal dipenuhi dulu; bucket persen di level yang sama membagi sisanya (basis root = gross aset, basis child = target parent). Aturan "total = 100%" hanya berlaku untuk bucket persen; total nominal yang melebihi basis ditampilkan sebagai peringatan.
- **Basis porsi: gross aset.** Penyebut adalah total saldo akun aset (cash, bank, e-wallet, investment, receivable). Akun liability (kartu kredit, loan) tidak masuk perhitungan porsi dan ditampilkan terpisah sebagai total utang.
- **Mapping akun statis** — setiap akun aset dipetakan ke satu bucket, atau dibagi dengan persentase tetap (mis. RDN 90% Saham, 10% Liquid cash). Persentase hanya berubah bila diedit user. Akun tanpa mapping tampil sebagai "Belum dialokasikan".
- **Porsi aktual** — Σ (saldo akun × share mapping) per bucket ÷ gross aset, dalam base currency. Akun Investment memakai nilai Valuation terakhir.
- **Tampilan** — donut porsi aktual, tabel target vs aktual vs selisih (poin % dan rupiah), dan riwayat porsi bulanan yang dihitung ulang dari postings. Garis target di grafik riwayat selalu memakai **target saat ini**; riwayat perubahan target tidak disimpan.

### 4.6 Pendukung

- **Payee** — pihak lawan transaksi; dipakai untuk autocomplete dan saran kategori.
- **Tag** — label bebas lintas kategori (mis. #liburan-bali, #kantor-reimburse).
- **Recurring rule** — jadwal transaksi berulang (tagihan, langganan, cicilan) yang menghasilkan transaksi terjadwal untuk dikonfirmasi.
- **Attachment** — foto struk atau PDF, disimpan sebagai file di folder data app, direferensikan dari DB.

## 5. Kebutuhan fungsional

MVP berisi 18 requirement yang cukup untuk menggantikan tracker spreadsheet: akun, transaksi, kategori, budget dasar, dan tracker alokasi modal. Kolom Rilis bisa kamu geser.

| ID     | Modul              | Requirement                                                                                                         | Rilis |
| ------ | ------------------ | ------------------------------------------------------------------------------------------------------------------- | ----- |
| ACC-01 | Account            | CRUD akun dengan tipe, group, currency, saldo awal; arsip akun yang punya transaksi                                 | MVP   |
| ACC-02 | Account            | CRUD account group, urutan drag-and-drop, subtotal per group                                                        | MVP   |
| ACC-03 | Account            | Saldo per akun & net worth realtime (aset − liability), dikonversi ke base currency                                 | MVP   |
| ACC-04 | Account            | Rekonsiliasi: input saldo riil, app membuat transaksi Adjustment dan menandai posting sebagai _reconciled_          | v1    |
| TRX-01 | Transaction        | Input Income / Expense / Transfer dalam satu form, keyboard-first (Tab, Enter, shortcut `N`)                        | MVP   |
| TRX-02 | Transaction        | Transfer dengan biaya admin opsional (posting expense tambahan)                                                     | MVP   |
| TRX-03 | Transaction        | Split transaksi ke beberapa kategori; total harus sama dengan nominal                                               | MVP   |
| TRX-04 | Transaction        | Daftar transaksi dengan filter (tanggal, akun, kategori, tag, payee, nominal) dan pencarian teks                    | MVP   |
| TRX-05 | Transaction        | Edit, duplikasi, hapus (soft delete + undo)                                                                         | MVP   |
| TRX-06 | Transaction        | Recurring rule (harian/mingguan/bulanan/tahunan, tanggal tertentu); transaksi jatuh tempo muncul untuk dikonfirmasi | v1    |
| TRX-07 | Transaction        | Transfer beda currency dengan kurs tercatat per transaksi                                                           | v2    |
| TRX-08 | Transaction        | Attachment struk (gambar/PDF) per transaksi                                                                         | v1    |
| CAT-01 | Category           | Kategori income/expense 2 level dengan set default; CRUD, ikon, warna                                               | MVP   |
| CAT-02 | Category           | Merge dan arsip kategori; transaksi lama ikut dipindah                                                              | MVP   |
| CAT-03 | Category           | Saran kategori dari payee terakhir                                                                                  | MVP   |
| CAT-04 | Tag & Payee        | CRUD tag dan payee, autocomplete                                                                                    | v1    |
| BUD-01 | Budget             | Set limit bulanan per kategori expense; salin dari bulan lalu                                                       | MVP   |
| BUD-02 | Budget             | Tampilan budget vs aktual per kategori, progress bar, sisa per hari                                                 | v1    |
| BUD-03 | Budget             | Rollover sisa/kelebihan per kategori                                                                                | v1    |
| BUD-04 | Budget             | Peringatan in-app saat realisasi ≥ 80% dan ≥ 100%                                                                   | v1    |
| CAP-01 | Capital allocation | CRUD allocation bucket dengan target % atau nominal; mapping akun ke bucket                                         | MVP   |
| CAP-02 | Capital allocation | Dashboard porsi aktual vs target: donut, tabel selisih (poin % dan rupiah), riwayat porsi bulanan                   | MVP   |
| CAP-03 | Capital allocation | Validasi alokasi: peringatan bila total target ≠ 100% dan daftar akun "Belum dialokasikan"                          | MVP   |
| CAP-04 | Capital allocation | Transaksi Valuation untuk akun Investment + riwayat nilai                                                           | MVP   |
| RPT-01 | Report             | Dashboard: net worth, cashflow bulan ini, top 5 kategori, budget yang hampir habis                                  | v1    |
| RPT-02 | Report             | Laporan income vs expense per bulan, breakdown kategori, tren 12 bulan                                              | MVP   |
| RPT-03 | Report             | Grafik net worth over time (dari postings + valuation)                                                              | MVP   |
| DAT-01 | Data               | Backup manual & otomatis (harian, simpan N versi) memakai SQLite online backup API                                  | v1    |
| DAT-02 | Data               | Export CSV/JSON (transaksi, akun, kategori, budget)                                                                 | v1    |
| DAT-03 | Data               | Import CSV dengan column mapping, preview, dan deteksi duplikat                                                     | v2    |
| SEC-01 | Security           | Kunci app dengan password + enkripsi DB (SQLCipher), auto-lock saat idle                                            | v1    |
| SET-01 | Settings           | Base currency, format tanggal/angka, tanggal awal bulan budget (mis. tanggal gajian 25), tema, bahasa               | v2    |
| EML-01 | Email capture      | Sync IMAP opt-in dari satu label/folder; credential hanya di OS keychain; read-only (`BODY.PEEK[]`); dedup Message-ID + index | v2    |
| EML-02 | Email capture      | Rule parsing tersimpan di DB (pola sender/subject/nominal/payee/tanggal, akun & kategori default); email yang tidak cocok masuk log unparsed | v2    |
| EML-03 | Email capture      | Review queue: confirm / edit lalu confirm / ignore; tidak ada transaksi tercatat tanpa konfirmasi; duplicate hint vs transaksi manual (nominal sama, ±1 hari) | v2    |
| EML-04 | Email capture      | Rule editor di UI + buat rule dari email unparsed                                                                   | v2    |

### Acceptance criteria inti

- **Saldo konsisten.** Untuk setiap akun, saldo yang ditampilkan = saldo awal + Σ postings aktif; diuji dengan property-based test pada ribuan transaksi acak.
- **Transfer seimbang.** Transfer satu currency selalu menghasilkan postings berjumlah 0 (di luar biaya); transaksi tidak seimbang ditolak di layer domain, bukan hanya di UI.
- **Transfer tidak memengaruhi budget.** Menambah/menghapus transfer tidak mengubah angka realisasi budget mana pun.
- **Input cepat.** Expense baru bisa dicatat dari layar mana pun dengan keyboard saja dalam ≤ 5 interaksi.
- **Hapus aman.** Hapus transaksi bisa di-undo; menghapus akun/kategori yang terpakai diblokir dan menawarkan arsip/merge.
- **Backup bisa dipulihkan.** File backup bisa di-restore ke instalasi baru dan menghasilkan saldo identik.
- **Periode budget custom.** Jika awal bulan budget = tanggal 25, transaksi tanggal 26 Oktober masuk periode "November".
- **Capture idempoten (v2).** Sync email ulang atau reset cursor karena UIDVALIDITY berubah tidak membuat entri review dobel; email yang di-ignore tidak muncul lagi.
- **Tanpa jaringan secara default.** Tanpa konfigurasi email, app tidak membuka koneksi jaringan apa pun (firewall test).

## 6. Skema data SQLite

Skema memakai tabel `STRICT`, nominal sebagai `INTEGER` dalam satuan minor, dan ID berupa UUIDv7 (TEXT) agar data siap di-merge bila suatu saat ada sync. Saldo **tidak disimpan** sebagai kolom; dihitung dari `posting` (dengan view/cache bila perlu).

**Konvensi**

- Nominal: `amount INTEGER` dalam satuan minor sesuai `currency.decimals` (IDR = 0, USD = 2). Di Rust dibungkus tipe `Money { amount: i64, currency: CurrencyCode }`.
- Tanggal transaksi: `TEXT` format `YYYY-MM-DD` (tanpa jam, menghindari bug timezone). Timestamp audit: `TEXT` ISO-8601 UTC.
- Soft delete: kolom `deleted_at`; query default memfilter `deleted_at IS NULL`.
- PRAGMA saat koneksi dibuka: `foreign_keys = ON`, `journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`.

```sql
CREATE TABLE currency (
  code      TEXT PRIMARY KEY,          -- 'IDR', 'USD'
  name      TEXT NOT NULL,
  symbol    TEXT NOT NULL,
  decimals  INTEGER NOT NULL CHECK (decimals BETWEEN 0 AND 8)
) STRICT;

CREATE TABLE account_group (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE account (
  id                   TEXT PRIMARY KEY,
  group_id             TEXT REFERENCES account_group(id),
  name                 TEXT NOT NULL,
  kind                 TEXT NOT NULL CHECK (kind IN
                         ('cash','bank','ewallet','investment','receivable','credit_card','loan')),
  currency             TEXT NOT NULL REFERENCES currency(code),
  include_in_net_worth INTEGER NOT NULL DEFAULT 1,
  credit_limit         INTEGER,           -- credit_card saja
  statement_day        INTEGER,           -- tanggal cetak tagihan
  icon TEXT, color TEXT,
  sort_order           INTEGER NOT NULL DEFAULT 0,
  archived_at          TEXT,
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL
) STRICT;

CREATE TABLE category (
  id          TEXT PRIMARY KEY,
  parent_id   TEXT REFERENCES category(id),
  kind        TEXT NOT NULL CHECK (kind IN ('income','expense')),
  name        TEXT NOT NULL,
  icon TEXT, color TEXT,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  archived_at TEXT,
  UNIQUE (parent_id, name)
) STRICT;

CREATE TABLE payee (
  id                  TEXT PRIMARY KEY,
  name                TEXT NOT NULL UNIQUE,
  default_category_id TEXT REFERENCES category(id)
) STRICT;

CREATE TABLE txn (
  id         TEXT PRIMARY KEY,
  kind       TEXT NOT NULL CHECK (kind IN
               ('income','expense','transfer','adjustment','valuation','opening')),
  date       TEXT NOT NULL,                  -- YYYY-MM-DD
  payee_id   TEXT REFERENCES payee(id),
  note       TEXT,
  recurring_rule_id TEXT REFERENCES recurring_rule(id),
  status     TEXT NOT NULL DEFAULT 'posted' CHECK (status IN ('scheduled','posted')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
) STRICT;

CREATE TABLE posting (
  id          TEXT PRIMARY KEY,
  txn_id      TEXT NOT NULL REFERENCES txn(id) ON DELETE CASCADE,
  account_id  TEXT NOT NULL REFERENCES account(id),
  category_id TEXT REFERENCES category(id),  -- NULL untuk sisi transfer
  amount      INTEGER NOT NULL,              -- +masuk / -keluar, minor unit akun
  fx_rate     TEXT,                          -- desimal sbg TEXT, mis. '15850.25'
  memo        TEXT,
  reconciled  INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE tag     (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE) STRICT;
CREATE TABLE txn_tag (txn_id TEXT REFERENCES txn(id) ON DELETE CASCADE,
                      tag_id TEXT REFERENCES tag(id),
                      PRIMARY KEY (txn_id, tag_id)) STRICT;

CREATE TABLE recurring_rule (
  id           TEXT PRIMARY KEY,
  template     TEXT NOT NULL,       -- JSON: header + postings
  rrule        TEXT NOT NULL,       -- mis. 'FREQ=MONTHLY;BYMONTHDAY=25'
  next_date    TEXT NOT NULL,
  auto_post    INTEGER NOT NULL DEFAULT 0,
  ended_at     TEXT
) STRICT;

CREATE TABLE budget (
  id          TEXT PRIMARY KEY,
  category_id TEXT NOT NULL REFERENCES category(id),
  period      TEXT NOT NULL,         -- '2026-11' (periode budget, bukan bulan kalender)
  amount      INTEGER NOT NULL CHECK (amount >= 0),
  rollover    INTEGER NOT NULL DEFAULT 0,
  UNIQUE (category_id, period)
) STRICT;

-- Capital allocation: tracker read-only, tidak menghasilkan transaksi
CREATE TABLE allocation_bucket (
  id          TEXT PRIMARY KEY,
  parent_id   TEXT REFERENCES allocation_bucket(id),  -- opsional: bucket bertingkat
  name        TEXT NOT NULL,
  target_bp   INTEGER NOT NULL CHECK (target_bp BETWEEN 0 AND 10000),  -- 7000 = 70.00%
  color       TEXT,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  UNIQUE (parent_id, name)
) STRICT;

CREATE TABLE allocation_mapping (
  account_id TEXT NOT NULL REFERENCES account(id),
  bucket_id  TEXT NOT NULL REFERENCES allocation_bucket(id),
  share_bp   INTEGER NOT NULL DEFAULT 10000,   -- porsi saldo akun ke bucket
  PRIMARY KEY (account_id, bucket_id)
) STRICT;

CREATE TABLE fx_rate (
  date TEXT NOT NULL, base TEXT NOT NULL, quote TEXT NOT NULL,
  rate TEXT NOT NULL,
  PRIMARY KEY (date, base, quote)
) STRICT;

CREATE TABLE attachment (
  id TEXT PRIMARY KEY, txn_id TEXT NOT NULL REFERENCES txn(id),
  file_name TEXT NOT NULL, sha256 TEXT NOT NULL, mime TEXT NOT NULL
) STRICT;

CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;

CREATE INDEX idx_txn_date        ON txn(date) WHERE deleted_at IS NULL;
CREATE INDEX idx_posting_account ON posting(account_id);
CREATE INDEX idx_posting_cat     ON posting(category_id);
CREATE INDEX idx_posting_txn     ON posting(txn_id);
```

**Tabel email capture (v2, `EML-01`–`EML-04`)**

Credential email (App Password) **tidak pernah** disimpan di DB, file config, atau log; hanya di OS keychain.

```sql
CREATE TABLE email_sync_cursor (
  mailbox        TEXT PRIMARY KEY,           -- label Gmail, mis. "Finance"
  uid_validity   INTEGER NOT NULL,
  last_uid       INTEGER NOT NULL DEFAULT 0, -- di-update per email, bukan per batch
  last_synced_at TEXT
) STRICT;

CREATE TABLE email_rule (
  id                  TEXT PRIMARY KEY,
  name                TEXT NOT NULL,         -- "BCA Debit"
  sender_pattern      TEXT NOT NULL,         -- "@bca.co.id"
  subject_regex       TEXT,
  amount_regex        TEXT NOT NULL,
  payee_regex         TEXT,
  date_regex          TEXT,
  date_format         TEXT,
  kind                TEXT NOT NULL CHECK (kind IN ('income','expense','transfer')),
  account_id          TEXT REFERENCES account(id),
  default_category_id TEXT REFERENCES category(id),
  priority            INTEGER NOT NULL DEFAULT 0,
  enabled             INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT
) STRICT;

CREATE TABLE email_capture (                 -- review queue
  id          TEXT PRIMARY KEY,
  message_id  TEXT NOT NULL,
  idx         INTEGER NOT NULL DEFAULT 0,    -- urutan transaksi dalam satu email
  rule_id     TEXT REFERENCES email_rule(id),
  date        TEXT NOT NULL,                 -- YYYY-MM-DD dari body email, bukan header Date
  amount      INTEGER NOT NULL,              -- satuan minor, selalu positif
  currency    TEXT NOT NULL,
  payee       TEXT,
  raw_snippet TEXT,
  status      TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending','confirmed','ignored')),
  txn_id      TEXT REFERENCES txn(id),       -- terisi saat confirmed
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  UNIQUE (message_id, idx)                   -- kunci idempotency sync
) STRICT;

CREATE TABLE email_unparsed (
  message_id  TEXT PRIMARY KEY,
  sender      TEXT,
  subject     TEXT,
  received_at TEXT,
  raw_snippet TEXT,
  created_at  TEXT NOT NULL
) STRICT;
```

**Invariant yang dijaga di layer domain (Rust), bukan trigger SQL**

1. `income` / `expense`: setiap posting ke akun wajib punya `category_id` dengan `kind` yang sesuai.
2. `transfer`: tepat 2 posting tanpa kategori (+ opsional posting biaya dengan kategori expense); jika currency sama, jumlah 2 posting utama = 0.
3. `valuation` hanya boleh ke akun `investment`; `adjustment` tidak punya kategori budget.
4. Total `target_bp` bucket dengan `parent_id` sama ≤ 10000 (peringatan di UI bila ≠ 10000); total `share_bp` satu akun ≤ 10000.
5. Semua perubahan satu transaksi (header + postings + tags) dalam satu SQLite transaction.

## 7. Arsitektur teknis

Logika bisnis hidup di crate Rust yang tidak tahu apa-apa soal UI. Dengan begitu pilihan D1 (Tauri vs Slint/egui) hanya mengganti lapisan paling luar, dan seluruh aturan saldo bisa dites tanpa membuka window.

**Struktur Cargo workspace (asumsi D1 = Tauri)**

```
kasku/
├─ crates/
│  ├─ core/       # domain murni: Money, Account, Txn, Posting, validasi invariant, kalkulasi budget & alokasi
│  ├─ storage/    # rusqlite: koneksi, PRAGMA, migrasi, repository, query laporan
│  └─ app/        # use case / service: TransactionService, BudgetService, AllocationService, ReportService, BackupService
├─ src-tauri/     # Tauri commands (tipis), DTO serde, event ke UI
└─ ui/            # Vue 3 + TypeScript + Vite, shadcn-vue + Tailwind v4, Pinia
```

**Aturan dependensi:** `ui → src-tauri → app → (core, storage)`, dan `storage → core`. `core` tidak bergantung pada crate IO apa pun.

**Stack frontend (D1):** Vue 3 (Composition API, `<script setup>`) + TypeScript + Vite, komponen dari [shadcn-vue](https://www.shadcn-vue.com) yang dibangun di atas Reka UI dan Tailwind CSS v4, state dengan Pinia, validasi form dengan Zod. Komponen shadcn-vue di-copy ke `ui/src/components/ui/` lewat CLI (`npx shadcn-vue@latest add ...`), jadi bebas dimodifikasi dan tidak menambah dependensi runtime besar.

**Crate yang disarankan**

| Kebutuhan    | Crate                                                           | Catatan                                                           |
| ------------ | --------------------------------------------------------------- | ----------------------------------------------------------------- |
| SQLite       | `rusqlite` (fitur `bundled`, atau `bundled-sqlcipher` untuk D8) | SQLite ikut di-compile, tidak bergantung versi sistem             |
| Migrasi      | `rusqlite_migration`                                            | Migrasi SQL ber-versi via `user_version`, dijalankan saat startup |
| ID           | `uuid` (v7)                                                     | Terurut waktu, aman untuk sync di masa depan                      |
| Tanggal      | `time` atau `chrono`                                            | Tanggal transaksi sebagai `Date` tanpa timezone                   |
| Desimal kurs | `rust_decimal`                                                  | Hanya untuk kurs & persentase; nominal tetap `i64`                |
| Error        | `thiserror` (lib), `anyhow` (bin)                               | Error domain bertipe, dipetakan ke pesan UI                       |
| Serialisasi  | `serde`, `serde_json`                                           | DTO dan template recurring                                        |
| Tipe TS      | `specta` + `tauri-specta`                                       | Generate tipe TypeScript dari command Rust, satu sumber kebenaran |
| Recurring    | subset RRULE di `nabungo-core::recurrence` (tanpa crate; clamp akhir bulan, lihat plan 7d K1) | Parsing aturan RFC 5545                                           |
| CSV          | `csv`                                                           | Import/export                                                     |
| Lokasi data  | `directories`                                                   | Linux: `~/.local/share/kasku/`, Windows: `%APPDATA%\kasku\`       |
| Logging      | `tracing`, `tracing-appender`                                   | Log lokal berputar, tanpa telemetry                               |
| Email (v2)   | client IMAP + `mail-parser`, `html2text`, `regex`, `keyring`    | Hanya untuk `EML-*`; TLS lewat OpenSSL yang sudah dibundel untuk SQLCipher. Rule engine murni boleh di `nabungo-core`; IMAP & keyring di `nabungo-app` (atau crate terpisah), tidak di core |

**Pola akses database**

- Satu koneksi tulis dilindungi `Mutex` (SQLite single-writer) + pool kecil koneksi read-only untuk laporan, memanfaatkan WAL.
- Setiap use case yang menulis berjalan di satu `rusqlite::Transaction`; repository menerima `&Transaction`, bukan `&Connection`.
- Query laporan berat (net worth over time, tren 12 bulan) dikerjakan di SQL dengan `GROUP BY`, bukan di loop Rust.
- Opsional: tabel cache `account_balance_daily` di-rebuild saat ada perubahan, jika profiling menunjukkan perlu.

**Strategi testing**

- `core`: unit test + `proptest` untuk invariant (saldo, transfer seimbang, periode budget custom).
- `storage` & `app`: integration test dengan SQLite in-memory, migrasi dijalankan penuh setiap test.
- Migrasi: test upgrade dari fixture DB versi lama ke versi terbaru.
- UI: Vitest untuk komponen; Playwright/WebDriver untuk 3–5 alur kritis (input transaksi, transfer, budget).
- CI: `cargo fmt --check`, `clippy -D warnings`, test, build artefak Linux & Windows.

## 8. Kebutuhan non-fungsional

| Area          | Requirement                                                                                                        | Target                                               |
| ------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Offline       | Tidak ada network call saat runtime kecuali fitur opt-in (auto-capture email `EML-*`, cek update); CSP Tauri tetap memblokir semua origin eksternal (koneksi IMAP dari Rust, bukan webview) | 0 request keluar dengan konfigurasi default (diverifikasi dengan firewall test) |
| Performa      | Startup sampai dashboard tampil                                                                                    | < 1,5 detik dengan 50.000 transaksi                  |
| Performa      | Simpan transaksi / buka daftar transaksi (paginasi)                                                                | < 100 ms / < 200 ms                                  |
| Performa      | Laporan 12 bulan & net worth over time                                                                             | < 500 ms dengan 50.000 transaksi                     |
| Ukuran        | Installer                                                                                                          | < 15 MB (Tauri)                                      |
| Integritas    | Semua tulis atomik; `PRAGMA integrity_check` saat startup bila app ditutup tidak normal                            | Tidak ada saldo korup setelah force-kill             |
| Backup        | Backup otomatis harian via SQLite online backup API, retensi 14 file + backup sebelum setiap migrasi               | Restore teruji di CI                                 |
| Keamanan      | Opsi SQLCipher + master password (Argon2id untuk derivasi key); auto-lock idle 5 menit (bisa diatur)               | Tanpa password, file DB tidak terbaca                |
| Keamanan      | Tidak ada password recovery: tampilkan peringatan jelas + opsi recovery key yang dicetak                           | —                                                    |
| Privasi       | Tidak ada telemetry/analytics; crash log hanya lokal                                                               | —                                                    |
| Privasi       | Credential email (`EML-01`) hanya di OS keychain, tidak di DB/config/log; akses mailbox read-only (`BODY.PEEK[]`)  | —                                                    |
| Portabilitas  | Mode portable: folder data di samping executable (opsional)                                                        | Pindah laptop = copy satu folder                     |
| Aksesibilitas | Navigasi penuh dengan keyboard, kontras WCAG AA, ukuran font bisa diubah                                           | —                                                    |
| i18n          | Bahasa ID/EN; format angka & tanggal mengikuti locale (`Rp 1.250.000`, `12 Okt 2026`)                              | —                                                    |
| Update        | Updater Tauri **mati secara default** (offline); update manual dari GitHub Releases, atau opt-in pengecekan update | —                                                    |
| Packaging     | Linux: AppImage, `.rpm`, `.deb`; Windows: `.msi`/NSIS                                                              | Build reproducible di CI                             |

## 9. UX dan daftar layar

Layout dua kolom: sidebar kiri berisi navigasi + daftar akun per group dengan saldo, area kanan untuk konten. Tombol "+ Transaksi" dan shortcut `N` tersedia di semua layar.

| Layar          | Isi utama                                                                                                                    | Shortcut                 |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| Onboarding     | Pilih base currency, bahasa, tanggal awal periode budget, enkripsi (opsional), buat akun pertama, atau import CSV            | —                        |
| Dashboard      | Net worth + perubahan bulan ini, cashflow bulan ini, budget hampir habis, transaksi terjadwal jatuh tempo, ringkasan alokasi | `G D`                    |
| Transaksi      | Tabel virtualized dengan filter & pencarian, grouping per tanggal, saldo berjalan saat difilter per akun                     | `G T`                    |
| Form transaksi | Tab Income / Expense / Transfer; field: tanggal, akun, nominal, kategori, payee, tag, catatan, split, biaya admin, lampiran  | `N`, `Ctrl+Enter` simpan |
| Detail akun    | Saldo, grafik saldo, transaksi akun, tombol Rekonsiliasi & (untuk investment) Update Nilai                                   | —                        |
| Akun & group   | Kelola akun, group, urutan, arsip                                                                                            | —                        |
| Kategori       | Pohon kategori income/expense, merge, arsip                                                                                  | —                        |
| Budget         | Grid kategori × bulan: limit, aktual, sisa, progress; salin bulan lalu                                                       | `G B`                    |
| Alokasi modal  | Donut porsi aktual, tabel target vs aktual vs selisih, riwayat porsi bulanan; kelola bucket dan mapping akun                 | `G A`                    |
| Laporan        | Income vs expense, breakdown kategori, tren, net worth over time; filter periode                                             | `G R`                    |
| Terjadwal      | Daftar recurring rule dan transaksi jatuh tempo (konfirmasi / lewati / edit)                                                 | —                        |
| Inbox (v2)     | Hasil auto-capture email: tab Pending (confirm / edit / ignore, duplicate hint) dan Unparsed (buat rule dari email)          | —                        |
| Pengaturan     | Umum, keamanan, backup & restore, import/export, tampilan, email capture (v2: akun, label, rule, sync)                         | `Ctrl+,`                 |

**Detail interaksi penting**

- Input nominal mendukung ekspresi sederhana (`25000+12500`) dan sufiks `rb`/`jt` (`25rb` → 25.000, `1,5jt` → 1.500.000).
- Setelah memilih payee, kategori & akun terakhir terisi otomatis.
- Halaman Alokasi modal bisa di-toggle: per bucket, per tipe akun, atau per akun.
- Angka negatif/merah hanya untuk expense dan saldo liability; transfer ditampilkan netral dengan ikon panah.

**Pemetaan ke komponen shadcn-vue**

| Kebutuhan UI                                 | Komponen                                                                           |
| -------------------------------------------- | ---------------------------------------------------------------------------------- |
| Sidebar navigasi + daftar akun               | `Sidebar`, `Collapsible`                                                           |
| Daftar transaksi (sort, filter, virtualized) | `Data Table` (TanStack Table) + `@tanstack/vue-virtual`                            |
| Form transaksi cepat                         | `Sheet` atau `Dialog` + `Tabs` (Income/Expense/Transfer), `Form` + VeeValidate/Zod |
| Pilih akun, kategori, payee                  | `Combobox`, `Command`                                                              |
| Quick add & navigasi keyboard                | `Command` (command palette, `Ctrl+K`)                                              |
| Tanggal & periode                            | `Calendar`, `Date Picker`, `Range Calendar`                                        |
| Progress budget                              | `Progress`, `Card`, `Badge`                                                        |
| Laporan & donut alokasi                      | `Chart` (Area, Bar, Line, Donut)                                                   |
| Notifikasi & konfirmasi                      | `Sonner`, `Alert Dialog`                                                           |

Tema terang/gelap memakai CSS variables shadcn; warna semantik (income hijau, expense merah, transfer netral) didefinisikan sebagai token tambahan.

## 10. Roadmap dan risiko

MVP difokuskan agar bisa langsung dipakai harian menggantikan tracker lama; fase berikutnya baru dimulai setelah gate-nya lulus. Estimasi waktu sengaja tidak dicantumkan karena bergantung pada D1 dan kapasitas side project.

&#91;embedded content: roadmap rilis · 3 fase, 2 gate\]

Satu hal yang disarankan: pakai MVP sendiri secara paralel dengan tracker lama selama sebulan (Gate 1) sebelum migrasi penuh.

**Risiko utama**

| Risiko                                               | Dampak                                           | Mitigasi                                                       |
| ---------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------- |
| Skema transaksi salah sejak awal (mis. single-entry) | Migrasi mahal saat menambah split/multi-currency | Model header + postings dari MVP (D3)                          |
| Floating point pada nominal                          | Selisih saldo rupiah yang sulit dilacak          | `i64` minor unit + tipe `Money`; lint melarang `f64` di `core` |
| Kehilangan data (disk rusak, salah hapus)            | Fatal untuk app offline                          | Backup otomatis + backup sebelum migrasi; soft delete + undo   |
| Lupa master password SQLCipher                       | Data tidak bisa dibuka sama sekali               | Recovery key yang dicetak; enkripsi opsional                   |
| Scope creep (holdings, sync, bank import)            | MVP tidak pernah selesai                         | Non-goals eksplisit; gate per fase                             |
| WebView berbeda per OS (WebKitGTK vs WebView2)       | Bug tampilan khusus Linux                        | Uji UI di Fedora & Windows di CI sejak MVP                     |
| Definisi periode budget custom (tgl gajian)          | Angka budget membingungkan                       | Satu fungsi `period_of(date)` di `core` dengan test tabel      |
| Format email bank berubah tanpa pemberitahuan (v2)   | Transaksi tidak ter-capture                      | Log `email_unparsed` + `raw_snippet`, rule bisa diedit di UI   |
| Google membatasi App Password / IMAP (v2)            | Auto-capture berhenti                            | Error login yang jelas; fallback import CSV (DAT-03)           |
| OS keychain tidak tersedia (Linux minimal) (v2)      | Credential tidak bisa disimpan                   | Error jelas; tidak ada fallback simpan ke DB/file              |
| Dependensi IMAP/TLS menambah ukuran installer (v2)   | Melewati batas < 15 MB                           | Ukur di spike; pakai OpenSSL yang sudah dibundel               |
