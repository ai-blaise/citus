-- FEATURE: MB6 pgaudit overhead microbench setup.
CREATE EXTENSION IF NOT EXISTS pgaudit;
DROP TABLE IF EXISTS mb6_audited;
CREATE TABLE mb6_audited (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
SET pgaudit.log = 'write';
