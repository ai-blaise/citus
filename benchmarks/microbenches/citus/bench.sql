INSERT INTO mb2_orders
SELECT s, (s % 4096)::INT, (random() * 1000)::NUMERIC(12, 2)
FROM generate_series(1, :row_count) s;
