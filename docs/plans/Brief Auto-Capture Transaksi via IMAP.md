# Brief: Auto-Capture Transaksi via IMAP

Oct 27, 2026 · @Gilang Fatahilah

Requirement: `EML-01`–`EML-04` (PRD §5, rilis v2), keputusan D13 (PRD §3), skema PRD §6. Fitur ini opt-in dan mati default; tanpa konfigurasi email app tidak membuka koneksi jaringan (PRD §8).

## Latar belakang & tujuan

Aplikasi akan menarik email notifikasi transaksi dari Gmail via IMAP, mem-parse-nya secara lokal, lalu menaruh hasilnya di antrian review sebelum masuk ke buku catatan.

Sebelumnya alur ini berjalan di Notion lewat Gmail + Apps Script. Aplikasi desktop (Rust, Tauri, Vue) bersifat offline-first, jadi email diperlakukan sebagai sumber data yang di-sync saat online saja. Parsing, penyimpanan, dan kategorisasi tetap lokal.

Tujuan:

- Mengurangi input manual untuk transaksi bank dan e-money yang punya notifikasi email.
- Tidak ada transaksi dobel walau sync diulang.
- Format email baru bisa ditangani dengan menambah rule, tanpa rebuild aplikasi.

## Scope & non-goals

Scope v1: satu akun Gmail, satu folder/label, parsing berbasis rule, dan review manual sebelum transaksi dicatat.

| Masuk scope | Di luar scope |
| --- | --- |
| Sync IMAP dari label `Finance` | Gmail API + OAuth |
| Rule engine yang disimpan di DB | Parsing berbasis ML/LLM |
| Staging + review queue di UI | Auto-commit tanpa review |
| Dedup via Message-ID | Multi-akun email |
| Duplicate hint vs transaksi manual | Background daemon saat app tertutup |
| Log email yang gagal di-parse | E-money tanpa notifikasi email (butuh companion app Android) |

Import mutasi CSV/PDF dari internet banking direncanakan sebagai fallback, tapi terpisah dari brief ini.

## Setup Gmail & keamanan credential

Login IMAP memakai App Password Gmail, yang disimpan hanya di OS keychain.

1. Aktifkan 2-Step Verification di akun Google.
2. Buat App Password (16 karakter) khusus untuk aplikasi ini.
3. Buat filter Gmail: email dari sender bank/e-money diberi label `Finance`. Di IMAP, label ini muncul sebagai folder.
4. Di aplikasi, user mengisi email + App Password sekali; aplikasi menyimpannya via crate `keyring` (Secret Service di Linux).

Server: `imap.gmail.com`, port 993, TLS.

Catatan keamanan: App Password memberi akses penuh ke mailbox, bukan read-only. Karena itu credential tidak boleh disimpan di SQLite, file config, atau log. Aplikasi hanya melakukan `SELECT` + `FETCH BODY.PEEK[]`, tidak pernah menulis atau menghapus email. User bisa mencabut App Password kapan saja dari akun Google.

## Alur sync IMAP

Sync berjalan saat app dibuka dan secara berkala selama app terbuka; cursor UID memastikan hanya email baru yang diproses.

&#91;embedded content: alur sync IMAP · 3 keputusan, loop per email\]

`last_uid` di-update per email, bukan di akhir batch, sehingga koneksi yang putus di tengah jalan tidak membuat email diproses ulang atau terlewat. `BODY.PEEK[]` dipakai agar email tidak ditandai sudah dibaca.

## Stack Rust

Semua logika sync dan parsing ada di sisi Rust (Tauri command); Vue hanya menampilkan progress dan review queue.

| Kebutuhan | Crate | Catatan |
| --- | --- | --- |
| IMAP client | `imap` (sync) di `spawn_blocking` / thread | TLS lewat OpenSSL yang sudah dibundel untuk SQLCipher (batas installer < 15 MB); `async-imap` + rustls hanya bila ukuran tetap aman |
| Parse MIME | `mail-parser` | Multipart, encoding, charset |
| HTML ke teks | `html2text` | Email bank umumnya tabel HTML |
| Credential | `keyring` | Secret Service di Fedora |
| Database | `rusqlite` | Sesuai D2; migrasi baru via `rusqlite_migration` |
| Pattern matching | `regex` | Rule dikompilasi sekali per sync |

Kontrak ke frontend: command `sync_email` mengembalikan ringkasan (jumlah baru, gagal parse, error), dan event `sync:progress` / `sync:done` di-emit selama proses.

## Data model & rule engine

Rule parsing disimpan sebagai data di SQLite, sehingga perubahan format email cukup diatasi dengan mengedit rule.

Skema lengkap ada di PRD §6 (`email_sync_cursor`, `email_rule`, `email_capture`, `email_unparsed`) dan mengikuti konvensi repo: ID UUIDv7 TEXT, FK TEXT ke `account`/`category`, tanggal `YYYY-MM-DD`, nominal `INTEGER` satuan minor + `currency`, timestamp audit ISO-8601 UTC. Kunci idempotency `UNIQUE(message_id, idx)` supaya satu email bisa berisi beberapa transaksi.

Cara kerja rule engine:

1. Ambil rule `enabled` yang `sender_pattern`-nya cocok, urutkan berdasarkan `priority`.
2. Rule pertama yang `subject_regex` dan `amount_regex`-nya cocok dipakai; sisanya dilewati.
3. Tidak ada rule yang cocok → simpan ke `unparsed_emails`, jangan dibuang diam-diam.
4. Insert ke `email_capture` dengan `ON CONFLICT(message_id, idx) DO NOTHING`.

Normalisasi angka: format Indonesia memakai titik untuk ribuan dan koma untuk desimal (`Rp 1.250.000,00`). Pakai parser nominal yang sudah ada di `crates/core/src/amount.rs`, jangan menulis parser baru; hasilnya integer satuan minor.

Waktu transaksi diambil dari body email, bukan header `Date`, karena email bisa terkirim terlambat.

## Review flow di UI

Hasil parsing tampil sebagai inbox; tidak ada transaksi yang tercatat tanpa konfirmasi user.

- **Confirm**: bentuk `TxnForm` lalu catat lewat use case transaksi yang ada (postings dibuat `form_postings` di core); status jadi `confirmed` dan `txn_id` terisi, dalam satu SQLite transaction.
- **Edit lalu confirm**: ubah nominal, payee, kategori, atau akun sebelum disimpan.
- **Ignore**: status jadi `ignored`; email tidak akan muncul lagi walau di-sync ulang.
- **Duplicate hint**: jika ada transaksi manual dengan nominal sama dan selisih waktu ±1 hari, tawarkan *merge* alih-alih membuat transaksi baru.
- **Unparsed**: tab terpisah berisi email yang gagal di-parse, dengan tombol untuk membuat rule baru dari email tersebut.

Implementasi 9c (`docs/design/design.md` §8.10):

- **Catat** hanya tampil untuk draft *siap* (akun aktif, kategori aktif sejenis atau akun tujuan untuk transfer, tanpa kandidat duplikat); selain itu aksi utama adalah **Tinjau**.
- **Tinjau** membuka sheet berisi form transaksi biasa yang terisi dari draft, plus "Email asli" (snippet) yang tertutup default. Isian tidak tertimpa bila sync berjalan di latar; capture yang sudah diproses di tempat lain memunculkan error di form.
- **Duplicate hint** berupa daftar kandidat dengan tombol **Hubungkan**: capture ditandai `confirmed` dengan `txn_id` transaksi yang ada, tanpa membuat transaksi baru.
- **Abaikan** (satu atau massal sampai tanggal, untuk backlog sync pertama) selalu bisa di-undo lewat toast.
- **Gagal dibaca** hanya menampilkan subjek, pengirim, waktu terima, dan snippet; tombol buat rule menyusul di 9d.

Auto-kategori: pakai `Ledger::payee_suggestion` (`crates/app/src/payee.rs`), yang sudah mengisi kategori & akun dari transaksi terakhir payee yang sama.

## Risiko & edge case

Risiko terbesar ada pada perubahan format email bank yang terjadi tanpa pemberitahuan; mitigasinya adalah log unparsed dan rule yang bisa diedit.

| Risiko | Dampak | Mitigasi |
| --- | --- | --- |
| Format email bank berubah | Transaksi tidak tercatat | `unparsed_emails` + rule editable + simpan `raw_snippet` |
| UIDVALIDITY berubah | Cursor UID tidak valid | Reset cursor, rescan; dedup tetap aman lewat Message-ID |
| App Password bocor | Akses penuh ke mailbox | Simpan di keychain saja, jangan pernah di log |
| Google membatasi App Password | Sync berhenti | Tampilkan error login yang jelas; fallback import CSV |
| Email terlambat datang | Tanggal salah | Ambil waktu dari body email |
| E-money tanpa email per transaksi | Data tidak lengkap | Dicatat manual atau via import; companion app di luar scope |
| Satu email berisi beberapa transaksi | Hanya satu yang tercatat | Rule mendukung banyak match; dedup key = Message-ID + index |
| Koneksi putus saat sync | Sync setengah jalan | Update `last_uid` per email, bukan di akhir batch |

## Milestone

Dikerjakan bertahap, dan tiap tahap bisa dipakai sendiri sebelum lanjut ke tahap berikutnya. Tanggal belum ditentukan.

**Status (2026-10-27):** milestone 1–3 selesai di Fase 9a (`docs/plans/2026-10-27-fase-9a-email-capture-backend.md`): crate `nabungo-mail`, rule bawaan BCA + Jago (diuji pada 348 email asli: 0 unparsed), tabel `email_*`, review queue di `nabungo-app`, CLI `capture_sync`. Milestone 4 → 9b, 5–6 → 9c.

**Status (2026-10-28):** milestone 4 selesai di Fase 9b (`docs/plans/2026-10-28-fase-9b-email-capture-tauri.md`): App Password di OS keychain, `connect_email`/`disconnect_email`/`sync_email`, sync saat app dibuka lalu tiap 15 menit (thread `email-sync`), kartu Email di Settings. Inbox review tetap di 9c.

**Status (2026-10-28):** milestone 5 dan sebagian 6 selesai di Fase 9c (`docs/plans/2026-10-28-fase-9c-email-inbox.md`): Inbox (`/inbox`) dengan Catat cepat, Tinjau (edit lalu confirm), Abaikan + Urungkan, abaikan massal sampai tanggal, hint duplikat dengan `Hubungkan` (link ke transaksi yang ada, bukan merge), dan tab Gagal dibaca (hanya baca). Rule editor + buat rule dari email unparsed → 9d.

1. **Spike IMAP**: CLI kecil di Rust yang login, `SELECT "Finance"`, dan mencetak subject 10 email terakhir.
2. **Parser satu bank**: satu rule hardcoded untuk bank yang paling sering dipakai, diuji dengan contoh email asli yang sudah disensor.
3. **Schema + rule engine**: tabel di atas, rule dibaca dari DB, dedup via Message-ID, log unparsed.
4. **Integrasi Tauri**: command `sync_email`, credential di keychain, sync saat app dibuka.
5. **Review UI**: inbox staging, confirm/edit/ignore, tab unparsed.
6. **Polish**: duplicate hint, auto-kategori, sync berkala (pola `spawn_backup_scheduler` di `src-tauri/src/lifecycle.rs`: lewati tick saat terkunci/mode pemulihan), rule editor di UI.

### Open questions

- [x] Bank dan e-money mana saja yang dipakai dan mengirim email per transaksi? → BCA dan Jago (9a).
- [x] Rule editor perlu UI di v1, atau cukup seed SQL dulu? → seed dulu, editor nanti (EML-04).
- [x] Interval sync berkala berapa menit saat app terbuka? → 15 menit, plus sekali saat app dibuka/di-unlock (9b).
