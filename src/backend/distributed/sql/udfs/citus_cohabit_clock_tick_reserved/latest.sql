CREATE OR REPLACE FUNCTION pg_catalog.citus_cohabit_clock_tick_reserved()
    RETURNS boolean
    LANGUAGE C STRICT
    AS 'MODULE_PATHNAME', $$citus_cohabit_clock_tick_reserved$$;
COMMENT ON FUNCTION pg_catalog.citus_cohabit_clock_tick_reserved()
    IS 'returns true when Citus reserved its logical-clock tick slot for an operator-approved pg_cron cohabitant';
GRANT EXECUTE ON FUNCTION pg_catalog.citus_cohabit_clock_tick_reserved() TO PUBLIC;
