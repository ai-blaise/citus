-- FEATURE: D9
DO $$
DECLARE
    extension_row pg_extension;
    member_count integer;
BEGIN
    SELECT * INTO STRICT extension_row FROM pg_extension WHERE extname = 'ai_blaise_citus';
    IF extension_row.extversion <> '0.1.2' THEN
        RAISE EXCEPTION 'security floor version mismatch';
    END IF;
    SELECT count(*) INTO member_count FROM pg_proc p
    JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid
    WHERE d.refclassid = 'pg_extension'::regclass AND d.refobjid = extension_row.oid AND d.deptype = 'e';
    IF member_count <> 153 THEN
        RAISE EXCEPTION 'unexpected extension routine inventory';
    END IF;
    IF EXISTS (
        SELECT FROM pg_proc p
        JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid
        CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
        WHERE d.refclassid = 'pg_extension'::regclass AND d.refobjid = extension_row.oid
          AND d.deptype = 'e' AND acl.grantee = 0
    ) THEN
        RAISE EXCEPTION 'extension routine remains publicly executable';
    END IF;
    IF cardinality(extension_row.extconfig) <> 68
       OR cardinality(extension_row.extcondition) <> 68
       OR EXISTS (SELECT FROM unnest(extension_row.extcondition) condition WHERE condition <> '') THEN
        RAISE EXCEPTION 'extension dump inventory or filters mismatch';
    END IF;
    SELECT count(*) INTO member_count FROM pg_class c
    JOIN pg_depend d ON d.classid = 'pg_class'::regclass AND d.objid = c.oid
    WHERE d.refclassid = 'pg_extension'::regclass AND d.refobjid = extension_row.oid
      AND d.deptype = 'e' AND c.relkind IN ('r', 'S') AND c.oid = ANY(extension_row.extconfig);
    IF member_count <> 68 OR (
        SELECT count(*) FROM pg_class WHERE oid = ANY(extension_row.extconfig) AND relkind = 'r'
    ) <> 44 THEN
        RAISE EXCEPTION 'backup coverage is not exact extension table and sequence membership';
    END IF;
    IF EXISTS (
        SELECT FROM pg_extension_update_paths('ai_blaise_citus')
        WHERE source = '0.1.2' AND target IN ('0.1.0', '0.1.1') AND path IS NOT NULL
    ) THEN
        RAISE EXCEPTION 'security floor has an unsafe downgrade path';
    END IF;
END
$$;
