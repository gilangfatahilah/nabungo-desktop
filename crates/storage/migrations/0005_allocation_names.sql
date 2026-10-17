-- 0005: nama bucket alokasi unik tanpa membedakan huruf (root dan sesama parent).
CREATE UNIQUE INDEX allocation_bucket_root_name_nocase ON allocation_bucket (name COLLATE NOCASE)
  WHERE parent_id IS NULL;
CREATE UNIQUE INDEX allocation_bucket_child_name_nocase ON allocation_bucket (parent_id, name COLLATE NOCASE)
  WHERE parent_id IS NOT NULL;
