-- FEATURE: D9
-- Run with psql -X -v ON_ERROR_STOP=1 as a superuser during the upgrade window.
\set ON_ERROR_STOP on
BEGIN;
SET LOCAL search_path = pg_catalog, pg_temp;

DO $$
BEGIN
    IF current_user <> session_user OR NOT (SELECT rolsuper FROM pg_roles WHERE rolname = current_user) THEN
        RAISE EXCEPTION 'privilege-preserving upgrade requires an unassumed superuser session';
    END IF;
    IF (SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus') IS DISTINCT FROM '0.1.1' THEN
        RAISE EXCEPTION 'privilege-preserving upgrade requires companion version 0.1.1';
    END IF;
END
$$;

CREATE TEMP TABLE ai_blaise_upgrade_routines ON COMMIT DROP AS
SELECT p.oid, p.proowner, p.prokind, n.nspname, p.proname,
       pg_get_function_identity_arguments(p.oid) AS arguments
FROM pg_proc p
JOIN pg_namespace n ON n.oid = p.pronamespace
JOIN pg_depend d ON d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e'
JOIN pg_extension e ON d.refclassid = 'pg_extension'::regclass AND d.refobjid = e.oid
WHERE e.extname = 'ai_blaise_citus';

CREATE TEMP TABLE ai_blaise_upgrade_grants ON COMMIT DROP AS
SELECT r.oid, acl.* FROM ai_blaise_upgrade_routines r
JOIN pg_proc p ON p.oid = r.oid
CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
WHERE acl.grantee <> 0;

DO $$
DECLARE
    grant_row record;
BEGIN
    IF (SELECT count(*) FROM ai_blaise_upgrade_routines) <> 153
       OR EXISTS (SELECT FROM ai_blaise_upgrade_routines WHERE prokind <> 'f')
       OR EXISTS (
           SELECT FROM ai_blaise_upgrade_grants g
           JOIN ai_blaise_upgrade_routines r USING (oid)
           WHERE g.grantor <> r.proowner OR g.privilege_type <> 'EXECUTE'
              OR NOT EXISTS (SELECT FROM pg_roles WHERE oid = g.grantee)
       ) THEN
        RAISE EXCEPTION 'routine ownership or delegated grants require manual upgrade review';
    END IF;
    FOR grant_row IN
        SELECT r.*, roles.rolname FROM ai_blaise_upgrade_grants g
        JOIN ai_blaise_upgrade_routines r USING (oid)
        JOIN pg_roles roles ON roles.oid = g.grantee
        WHERE g.grantee <> r.proowner
        ORDER BY r.oid, g.grantee
    LOOP
        EXECUTE format('REVOKE ALL ON ROUTINE %I.%I(%s) FROM %I',
                       grant_row.nspname, grant_row.proname, grant_row.arguments, grant_row.rolname);
    END LOOP;
END
$$;

ALTER EXTENSION ai_blaise_citus UPDATE TO '0.1.2';

DO $$
DECLARE
    grant_row record;
BEGIN
    IF EXISTS (
        SELECT FROM ai_blaise_upgrade_routines r LEFT JOIN pg_proc p ON p.oid = r.oid
        WHERE p.oid IS NULL OR p.proowner <> r.proowner OR p.prokind <> r.prokind
           OR p.proname <> r.proname
           OR (SELECT nspname FROM pg_namespace WHERE oid = p.pronamespace) <> r.nspname
           OR pg_get_function_identity_arguments(p.oid) <> r.arguments
    ) THEN
        RAISE EXCEPTION 'routine identity changed during privilege-preserving upgrade';
    END IF;
    FOR grant_row IN
        SELECT r.*, roles.rolname, owner_role.rolname AS owner_name, g.is_grantable
        FROM ai_blaise_upgrade_grants g
        JOIN ai_blaise_upgrade_routines r USING (oid)
        JOIN pg_roles roles ON roles.oid = g.grantee
        JOIN pg_roles owner_role ON owner_role.oid = r.proowner
        WHERE g.grantee <> r.proowner
        ORDER BY r.oid, g.grantee
    LOOP
        EXECUTE format('GRANT EXECUTE ON ROUTINE %I.%I(%s) TO %I%s GRANTED BY %I',
                       grant_row.nspname, grant_row.proname, grant_row.arguments, grant_row.rolname,
                       CASE WHEN grant_row.is_grantable THEN ' WITH GRANT OPTION' ELSE '' END,
                       grant_row.owner_name);
    END LOOP;
    IF EXISTS (
        (SELECT * FROM ai_blaise_upgrade_grants
         EXCEPT
         SELECT r.oid, acl.* FROM ai_blaise_upgrade_routines r JOIN pg_proc p ON p.oid = r.oid
         CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl)
        UNION ALL
        (SELECT r.oid, acl.* FROM ai_blaise_upgrade_routines r JOIN pg_proc p ON p.oid = r.oid
         CROSS JOIN LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
         EXCEPT SELECT * FROM ai_blaise_upgrade_grants)
    ) THEN
        RAISE EXCEPTION 'routine grants changed during privilege-preserving upgrade';
    END IF;
END
$$;
COMMIT;
