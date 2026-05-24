CREATE OR REPLACE FUNCTION pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id bigint)
    RETURNS boolean
    LANGUAGE C
    AS 'MODULE_PATHNAME', $$citus_fast_path_router_can_skip_coordinator$$;
COMMENT ON FUNCTION pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id bigint)
    IS 'returns true when a single-shard route can safely skip the coordinator and dispatch to the local worker path';
GRANT EXECUTE ON FUNCTION pg_catalog.citus_fast_path_router_can_skip_coordinator(shard_id bigint) TO PUBLIC;
