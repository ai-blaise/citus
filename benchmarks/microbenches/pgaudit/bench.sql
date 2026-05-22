INSERT INTO mb6_audited (payload)
SELECT md5(s::TEXT) FROM generate_series(1, :row_count) s;
