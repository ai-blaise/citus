SELECT count(*) FROM (
    SELECT pgnodemx.cpu()
    FROM generate_series(1, :row_count)
) t;
