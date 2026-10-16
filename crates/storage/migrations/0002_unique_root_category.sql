-- UNIQUE (parent_id, name) tidak berlaku untuk kategori root karena SQLite menganggap
-- setiap NULL berbeda. Nama kategori root harus unik per jenis (income/expense).
CREATE UNIQUE INDEX category_root_name_unique ON category (kind, name) WHERE parent_id IS NULL;
