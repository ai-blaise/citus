-- FEATURE: D9
-- Only relation names, counts and hashes leave the fixture database.
SET timezone = 'UTC';
SET datestyle = 'ISO, YMD';
CREATE TEMP TABLE backup_state (relation text PRIMARY KEY, rows bigint, checksum text);
DO $$
DECLARE
    relation record;
BEGIN
    FOR relation IN
        SELECT c.oid, c.relkind, n.nspname, c.relname FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_depend d ON d.classid = 'pg_class'::regclass AND d.objid = c.oid
        JOIN pg_extension e ON d.refclassid = 'pg_extension'::regclass AND d.refobjid = e.oid
        WHERE e.extname = 'ai_blaise_citus' AND d.deptype = 'e' AND c.relkind IN ('r', 'S')
    LOOP
        IF relation.relkind = 'S' THEN
            EXECUTE format(
                'INSERT INTO backup_state SELECT %L, 1, md5(jsonb_build_object(''last_value'', last_value, ''is_called'', is_called)::text) FROM %I.%I',
                relation.nspname || '.' || relation.relname, relation.nspname, relation.relname);
        ELSE
            EXECUTE format(
                'INSERT INTO backup_state SELECT %L, count(*), md5(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text)::text) FROM %I.%I t',
                relation.nspname || '.' || relation.relname, relation.nspname, relation.relname);
        END IF;
    END LOOP;
    IF (SELECT count(*) FROM backup_state) <> 68
       OR EXISTS (SELECT FROM backup_state WHERE rows <> 1 OR checksum IS NULL) THEN
        RAISE EXCEPTION 'expected one populated row in each of 44 tables and 24 sequences';
    END IF;
END
$$;
SELECT relation, rows, checksum FROM backup_state ORDER BY relation;
