\echo Use "CREATE EXTENSION ai_blaise_pgc_probe" to load this file. \quit

-- FEATURE: PGC1
CREATE FUNCTION ai_blaise_pgc_logical_clock_roundtrip(requested timestamptz)
RETURNS timestamptz
AS MODULE_PATHNAME, ai_blaise_pgc_logical_clock_roundtrip
LANGUAGE C STRICT VOLATILE;

-- FEATURE: PGC2
CREATE FUNCTION ai_blaise_pgc_subtrans_override(requested timestamptz, nodeid integer)
RETURNS text
AS MODULE_PATHNAME, ai_blaise_pgc_subtrans_override
LANGUAGE C STRICT VOLATILE;
