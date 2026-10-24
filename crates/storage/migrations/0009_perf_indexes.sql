-- Fase 8a: indeks covering untuk saldo dan nominal baris (diukur di harness 50.000 transaksi).

-- Saldo per akun & saldo berjalan (`balances_as_of`, `running_balances`, filter akun di daftar): covering.
CREATE INDEX idx_posting_account_txn ON posting (account_id, txn_id, amount);
-- Nominal baris per transaksi (`ROW_AMOUNT`, sort nominal) dan `category_totals`: covering.
CREATE INDEX idx_posting_txn_cat ON posting (txn_id, category_id, amount);
