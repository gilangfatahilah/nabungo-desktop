-- 0001: skema awal sesuai PRD §6. JANGAN diubah setelah rilis; tambah migrasi baru.

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

-- Seed: v1 hanya IDR (D4); kolom currency disiapkan untuk multi-currency v2.
INSERT INTO currency (code, name, symbol, decimals) VALUES ('IDR', 'Rupiah Indonesia', 'Rp', 0);
