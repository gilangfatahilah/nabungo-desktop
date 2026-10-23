-- 0008: target nominal bucket alokasi (CAP-01). Bucket nominal: target_amount terisi, target_bp = 0.
ALTER TABLE allocation_bucket ADD COLUMN target_amount INTEGER
  CHECK (target_amount IS NULL OR target_amount > 0);
