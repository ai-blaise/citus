SELECT count(*) FROM (
    SELECT uuid_generate_v7()
    FROM generate_series(1, :row_count)
) t;
