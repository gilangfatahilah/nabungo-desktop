-- Fase 8a2: `reconciled_through` di `list_accounts` (dipanggil beberapa kali per startup dasbor).
-- Tanpa indeks ini setiap posting akun dibaca untuk mengecek `reconciled` (±55 ms di 50.000 transaksi).
CREATE INDEX idx_posting_reconciled ON posting (account_id, txn_id) WHERE reconciled = 1;
