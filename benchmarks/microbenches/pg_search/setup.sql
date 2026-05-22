-- FEATURE: MB13 pg_search BM25 microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS mb13_docs;
CREATE TABLE mb13_docs (
    id BIGINT PRIMARY KEY,
    title TEXT NOT NULL,
    body TEXT NOT NULL
);
