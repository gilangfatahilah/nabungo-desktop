-- Lampiran: isi file di folder data `attachments/<sha256>`; DB menyimpan metadata.
-- (Indeks unik NOCASE nama tag sudah ada sejak 0004.)
ALTER TABLE attachment ADD COLUMN size_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE attachment ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
CREATE INDEX idx_attachment_txn ON attachment (txn_id);
CREATE INDEX idx_attachment_sha ON attachment (sha256);
