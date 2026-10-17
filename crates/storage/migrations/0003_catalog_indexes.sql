-- 0003: keunikan nama case-insensitive untuk group, payee, dan kategori (sesama parent /
-- sesama root per jenis). Nama akun boleh kembar.
CREATE UNIQUE INDEX account_group_name_nocase ON account_group (name COLLATE NOCASE);
CREATE UNIQUE INDEX payee_name_nocase ON payee (name COLLATE NOCASE);
CREATE UNIQUE INDEX category_child_name_nocase ON category (parent_id, name COLLATE NOCASE)
  WHERE parent_id IS NOT NULL;
CREATE UNIQUE INDEX category_root_name_nocase ON category (kind, name COLLATE NOCASE)
  WHERE parent_id IS NULL;
CREATE INDEX idx_txn_payee ON txn (payee_id) WHERE deleted_at IS NULL;
