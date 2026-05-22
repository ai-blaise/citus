-- FEATURE: MB1 timescaledb hypertable insert microbench setup.
CREATE EXTENSION IF NOT EXISTS timescaledb;
DROP TABLE IF EXISTS mb1_metrics;
CREATE TABLE mb1_metrics (
    ts TIMESTAMPTZ NOT NULL,
    device_id INTEGER NOT NULL,
    value DOUBLE PRECISION NOT NULL
);
SELECT create_hypertable('mb1_metrics', 'ts', chunk_time_interval => INTERVAL '1 day');
