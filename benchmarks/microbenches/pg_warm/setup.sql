-- FEATURE: MB22 pg_prewarm microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_prewarm;
DROP TABLE IF EXISTS mb22_warm_target;
CREATE TABLE mb22_warm_target (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
INSERT INTO mb22_warm_target (payload)
SELECT md5(s::TEXT) FROM generate_series(1, 100000) s;
