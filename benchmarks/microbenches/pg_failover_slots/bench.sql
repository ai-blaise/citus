INSERT INTO mb21_wal_writes (payload)
SELECT md5(s::TEXT) FROM generate_series(1, :row_count) s;
