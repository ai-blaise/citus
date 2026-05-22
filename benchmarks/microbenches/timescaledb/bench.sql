-- Insert <ROW_COUNT> rows spread across 7 days.
INSERT INTO mb1_metrics
SELECT NOW() - (s % 604800) * INTERVAL '1 second',
       (s % 1024)::INT,
       random()
FROM generate_series(1, :row_count) s;
