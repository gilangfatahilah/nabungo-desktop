# Fixture email notifikasi

HTML minimal yang meniru struktur email asli myBCA dan Jago (tabel `Label : Nilai`, nominal,
format tanggal) per Oktober 2026. Nama, nomor rekening/kartu, dan nomor referensi **disamarkan**;
merchant publik dibiarkan. Jangan commit email asli tanpa sensor.

| File | Jenis |
|---|---|
| `bca_qris.eml` | BCA QRIS (expense) |
| `bca_transfer.eml` | BCA transfer ke rekening BCA (expense) |
| `bca_va.eml` | BCA Virtual Account, nominal muncul 3× (expense, 1 item) |
| `bca_tarik_tunai.eml` | BCA tarik tunai tanpa kartu (transfer_out) |
| `bca_fractional.eml` | Nominal berdesimal bukan nol (unparsed) |
| `jago_bayar.eml` | Jago bayar QRIS (expense) |
| `jago_debit.eml` | Jago kartu debit, tanpa tanggal & merchant di body (expense) |
| `jago_dikembalikan.eml` | Jago uang dikembalikan (income) |
| `jago_terima_gopay.eml` | Jago terima dari GoPay (transfer_in) |
| `unknown_sender.eml` | Pengirim tidak dikenal (unparsed) |
