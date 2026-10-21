-- 0006: kontak untuk akun piutang/hutang (D10b).
CREATE TABLE contact (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL COLLATE NOCASE UNIQUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
) STRICT;

ALTER TABLE account ADD COLUMN contact_id TEXT REFERENCES contact(id);
CREATE INDEX account_contact ON account (contact_id);
