-- FEATURE: MB26 RUM index FTS microbench setup.
CREATE EXTENSION IF NOT EXISTS rum;
DROP TABLE IF EXISTS mb26_docs;
CREATE TABLE mb26_docs (
    id BIGSERIAL PRIMARY KEY,
    body TEXT NOT NULL,
    body_tsv tsvector NOT NULL
);
INSERT INTO mb26_docs (body, body_tsv)
SELECT md5(s::TEXT) || ' ' || md5((s + 1)::TEXT),
       to_tsvector('english',
                   md5(s::TEXT) || ' ' || md5((s + 1)::TEXT))
FROM generate_series(1, 100000) s;
