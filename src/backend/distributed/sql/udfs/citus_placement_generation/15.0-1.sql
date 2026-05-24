CREATE OR REPLACE FUNCTION pg_catalog.citus_placement_generation()
    RETURNS bigint
    LANGUAGE C STRICT
    AS 'MODULE_PATHNAME', $$citus_placement_generation$$;
COMMENT ON FUNCTION pg_catalog.citus_placement_generation()
    IS 'returns the current backend-local Citus placement generation counter for plan-cache invalidation';
GRANT EXECUTE ON FUNCTION pg_catalog.citus_placement_generation() TO PUBLIC;
