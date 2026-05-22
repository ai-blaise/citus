-- FEATURE: MB3 pgvector IVFFlat microbench setup.
CREATE EXTENSION IF NOT EXISTS vector;
DROP TABLE IF EXISTS mb3_vectors;
CREATE TABLE mb3_vectors (
    id BIGINT PRIMARY KEY,
    embedding vector(768) NOT NULL
);
