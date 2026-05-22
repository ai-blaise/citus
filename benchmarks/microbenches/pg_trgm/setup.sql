-- FEATURE: MB24 pg_trgm similarity microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
DROP TABLE IF EXISTS mb24_names;
CREATE TABLE mb24_names (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL);
INSERT INTO mb24_names (name)
SELECT md5(s::TEXT) FROM generate_series(1, 100000) s;
CREATE INDEX mb24_names_trgm ON mb24_names USING GIN (name gin_trgm_ops);
ANALYZE mb24_names;
