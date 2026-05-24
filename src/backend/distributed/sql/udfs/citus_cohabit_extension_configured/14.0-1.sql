CREATE OR REPLACE FUNCTION pg_catalog.citus_cohabit_extension_configured(extension_name text)
    RETURNS boolean
    LANGUAGE C STRICT
    AS 'MODULE_PATHNAME', $$citus_cohabit_extension_configured$$;
COMMENT ON FUNCTION pg_catalog.citus_cohabit_extension_configured(extension_name text)
    IS 'returns true when citus.cohabit_extensions contains a supported cohabiting extension name';
GRANT EXECUTE ON FUNCTION pg_catalog.citus_cohabit_extension_configured(extension_name text) TO PUBLIC;
