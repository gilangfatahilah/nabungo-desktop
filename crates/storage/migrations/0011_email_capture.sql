-- 0011: auto-capture transaksi dari email notifikasi (EML-01..03, D13). Credential tidak disimpan di DB.
CREATE TABLE email_sync_cursor (
  mailbox        TEXT PRIMARY KEY,
  uid_validity   INTEGER NOT NULL,
  last_uid       INTEGER NOT NULL DEFAULT 0,
  last_synced_at TEXT
) STRICT;

CREATE TABLE email_rule (
  id                  TEXT PRIMARY KEY,
  name                TEXT NOT NULL,
  sender_pattern      TEXT NOT NULL,
  subject_regex       TEXT,
  amount_regex        TEXT NOT NULL,
  payee_regex         TEXT,
  date_regex          TEXT,
  date_format         TEXT,
  kind                TEXT NOT NULL
                        CHECK (kind IN ('income','expense','transfer_out','transfer_in')),
  account_id          TEXT REFERENCES account(id),
  counter_account_id  TEXT REFERENCES account(id),
  default_category_id TEXT REFERENCES category(id),
  priority            INTEGER NOT NULL DEFAULT 0,
  enabled             INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, deleted_at TEXT
) STRICT;

CREATE TABLE email_capture (
  id          TEXT PRIMARY KEY,
  message_id  TEXT NOT NULL,
  idx         INTEGER NOT NULL DEFAULT 0,
  rule_id     TEXT REFERENCES email_rule(id),
  date        TEXT NOT NULL,
  amount      INTEGER NOT NULL,
  currency    TEXT NOT NULL,
  payee       TEXT,
  raw_snippet TEXT,
  status      TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending','confirmed','ignored')),
  txn_id      TEXT REFERENCES txn(id),
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  UNIQUE (message_id, idx)
) STRICT;

CREATE INDEX idx_email_capture_status ON email_capture (status, date);

CREATE TABLE email_unparsed (
  message_id  TEXT PRIMARY KEY,
  sender      TEXT,
  subject     TEXT,
  received_at TEXT,
  raw_snippet TEXT,
  created_at  TEXT NOT NULL
) STRICT;
