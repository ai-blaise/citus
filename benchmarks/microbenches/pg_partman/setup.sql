-- FEATURE: MB5 pg_partman partition-create microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_partman;
DROP TABLE IF EXISTS mb5_parent CASCADE;
CREATE TABLE mb5_parent (
    ts TIMESTAMPTZ NOT NULL,
    value DOUBLE PRECISION NOT NULL
) PARTITION BY RANGE (ts);
SELECT partman.create_parent(
    p_parent_table => 'public.mb5_parent',
    p_control => 'ts',
    p_type => 'range',
    p_interval => '1 day'
);
