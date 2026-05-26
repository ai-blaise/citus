-- FEATURE: Bundle1 Search1 G1 JS1 PM1 IA1 WF1 F2
-- Canonical extension creation order for the ai-blaise operand image.
--
-- The image build is responsible for making required extension control files
-- available. This SQL keeps cluster initialization deterministic and fails
-- loudly when a required extension is absent.

CREATE EXTENSION IF NOT EXISTS citus;
CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS ai_blaise_citus;
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_cron;
CREATE EXTENSION IF NOT EXISTS pg_partman;
CREATE EXTENSION IF NOT EXISTS pgaudit;
CREATE EXTENSION IF NOT EXISTS pgauditlogtofile;
CREATE EXTENSION IF NOT EXISTS pgsodium;
CREATE EXTENSION IF NOT EXISTS hll;
CREATE EXTENSION IF NOT EXISTS topn;
CREATE EXTENSION IF NOT EXISTS tdigest;
CREATE EXTENSION IF NOT EXISTS pgnodemx;
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pg_graphql;
CREATE EXTENSION IF NOT EXISTS pg_jsonschema;
CREATE EXTENSION IF NOT EXISTS age;
CREATE EXTENSION IF NOT EXISTS pg_uuidv7;
CREATE EXTENSION IF NOT EXISTS pg_repack;
CREATE EXTENSION IF NOT EXISTS pg_prewarm;
CREATE EXTENSION IF NOT EXISTS pg_warm;
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS citext;
CREATE EXTENSION IF NOT EXISTS rum;

-- Heavy bundle extensions: only present in bundle1-final-full. Created
-- conditionally so the same initdb path runs cleanly against both
-- bundle1-final-light and bundle1-final-full.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'pg_search') THEN
    EXECUTE 'CREATE EXTENSION IF NOT EXISTS pg_search;';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_available_extensions WHERE name = 'plv8') THEN
    EXECUTE 'CREATE EXTENSION IF NOT EXISTS plv8;';
  END IF;
END;
$$;

-- pg_failover_slots is shared_preload_libraries-only (no SQL extension).
-- plrust is alpha-deferred upstream and intentionally not created here; see
-- docs/ai-blaise/BUNDLED_EXTENSIONS.md for the EF6 boundary.
