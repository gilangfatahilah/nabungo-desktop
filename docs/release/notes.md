Nabungo adalah aplikasi keuangan pribadi desktop yang berjalan sepenuhnya offline: transaksi, saldo akun, budget, dan alokasi modal tersimpan di satu file SQLite milik Anda. Tidak ada akun, tidak ada telemetry, tidak ada koneksi internet saat aplikasi berjalan.

Baru: **Email capture** (opsional, mati secara default). Nabungo membaca email notifikasi bank dari satu label Gmail secara read-only lewat App Password yang disimpan di keychain sistem, lalu menyiapkan transaksi di Inbox untuk ditinjau. Rule pembacaan bisa diubah, dan rule baru bisa dibuat dari email yang gagal dibaca. Hanya fitur ini yang memakai internet, dan hanya setelah Anda mengaktifkannya.

## Unduhan

| Sistem | File |
|---|---|
| Fedora / openSUSE | `.rpm` |
| Ubuntu / Debian | `.deb` |
| Linux lain | `.AppImage` (lebih besar karena membawa WebKitGTK sendiri) |
| Windows 10/11 | `*-setup.exe` (disarankan, tanpa hak admin) atau `.msi` |

**Windows:** installer belum ditandatangani, jadi SmartScreen dapat menampilkan "Windows protected your PC". Pilih **More info → Run anyway**. Bila WebView2 belum terpasang, installer mengunduhnya saat instalasi (hanya sekali; aplikasi sendiri tidak memakai internet).

**Verifikasi file:** unduh `SHA256SUMS` ke folder yang sama lalu jalankan `sha256sum -c --ignore-missing SHA256SUMS` (Linux) atau `Get-FileHash <file>` (PowerShell) dan bandingkan.

## Data Anda

- Linux: `~/.local/share/nabungo/` — Windows: `%APPDATA%\nabungo\`
- Backup otomatis harian di subfolder `backups/` (14 terakhir). Backup manual dan ekspor CSV/JSON ada di Pengaturan.
- Bila enkripsi diaktifkan, simpan recovery key: tanpa password atau recovery key data tidak bisa dibuka.

Update bisa lewat **Pengaturan → Update aplikasi**: Periksa update, lalu unduh & pasang (installer diverifikasi dengan `SHA256SUMS`). Cek otomatis saat aplikasi dibuka bersifat opsional dan mati secara default. Update manual tetap bisa: unduh versi baru dari halaman Releases dan instal di atas versi lama; data tetap.
