-- FEATURE: MB8 hll add_agg microbench setup.
CREATE EXTENSION IF NOT EXISTS hll;
DROP TABLE IF EXISTS mb8_events;
CREATE TABLE mb8_events (user_id BIGINT NOT NULL);
INSERT INTO mb8_events
SELECT (random() * 1000000)::BIGINT FROM generate_series(1, 100000);
