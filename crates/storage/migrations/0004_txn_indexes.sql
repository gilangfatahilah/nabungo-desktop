-- Fase 4: tag unik tanpa membedakan huruf, urutan daftar transaksi, lookup tag.

CREATE UNIQUE INDEX tag_name_nocase ON tag (name COLLATE NOCASE);
CREATE INDEX idx_txn_date_id ON txn (date DESC, id DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_txn_tag_tag ON txn_tag (tag_id);
