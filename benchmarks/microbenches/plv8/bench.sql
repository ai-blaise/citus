SELECT count(*) FROM (
    SELECT mb18_plv8_add(s, s + 1)
    FROM generate_series(1, :row_count) s
) t;
