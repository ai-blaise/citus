CREATE OR REPLACE FUNCTION pg_catalog.citus_cohabit_extension_role(extension_name text)
    RETURNS text
    LANGUAGE C STRICT
    AS 'MODULE_PATHNAME', $$citus_cohabit_extension_role$$;
COMMENT ON FUNCTION pg_catalog.citus_cohabit_extension_role(extension_name text)
    IS 'classifies a supported Citus cohabiting extension as trusted-hook, clock-worker, partition-manager, or unsupported';
GRANT EXECUTE ON FUNCTION pg_catalog.citus_cohabit_extension_role(extension_name text) TO PUBLIC;
