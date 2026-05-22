-- FEATURE: MB15 pg_jsonschema validate microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_jsonschema;
DROP TABLE IF EXISTS mb15_docs;
CREATE TABLE mb15_docs (id BIGSERIAL PRIMARY KEY, doc JSONB NOT NULL);
INSERT INTO mb15_docs (doc)
SELECT jsonb_build_object('id', s, 'name', 'doc-' || s, 'value', random())
FROM generate_series(1, 10000) s;
