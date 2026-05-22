-- FEATURE: MB21 pg_failover_slots WAL-overhead microbench setup.
-- Extension is loaded as a shared_preload_library; CREATE EXTENSION
-- here is harmless if already present and otherwise validates the
-- bundle on every smoke run.
CREATE EXTENSION IF NOT EXISTS pg_failover_slots;
DROP TABLE IF EXISTS mb21_wal_writes;
CREATE TABLE mb21_wal_writes (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
