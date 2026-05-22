-- FEATURE: MB20 pg_repack microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_repack;
DROP TABLE IF EXISTS mb20_bloated;
CREATE TABLE mb20_bloated (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
INSERT INTO mb20_bloated (payload)
SELECT md5(s::TEXT) FROM generate_series(1, 100000) s;
DELETE FROM mb20_bloated WHERE id % 2 = 0;
