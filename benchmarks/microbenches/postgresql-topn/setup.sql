-- FEATURE: MB9 topn aggregation microbench setup.
CREATE EXTENSION IF NOT EXISTS topn;
DROP TABLE IF EXISTS mb9_events;
CREATE TABLE mb9_events (key TEXT NOT NULL);
INSERT INTO mb9_events
SELECT 'user-' || (random() * 1000)::INT FROM generate_series(1, 100000);
