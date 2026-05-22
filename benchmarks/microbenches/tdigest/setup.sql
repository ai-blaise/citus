-- FEATURE: MB10 tdigest percentile microbench setup.
CREATE EXTENSION IF NOT EXISTS tdigest;
DROP TABLE IF EXISTS mb10_samples;
CREATE TABLE mb10_samples (value DOUBLE PRECISION NOT NULL);
INSERT INTO mb10_samples
SELECT random() * 1000 FROM generate_series(1, 100000);
