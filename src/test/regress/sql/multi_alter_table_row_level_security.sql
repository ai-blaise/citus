--
-- MULTI_ALTER_TABLE_ROW_LEVEL_SECURITY
--
-- Test checks whether row level security can be enabled via
-- ALTER TABLE ... ENABLE | DISABLE ROW LEVEL SECURITY
SET citus.next_shard_id TO 1800000;
SET citus.shard_replication_factor TO 1;

-- Setup user accounts and schema for tests.
CREATE USER rls_table_owner;
CREATE USER rls_tenant_1;
CREATE USER rls_tenant_2;

SET client_min_messages TO WARNING;

CREATE SCHEMA alter_table_rls;

SET search_path TO alter_table_rls;
ALTER ROLE rls_table_owner SET search_path TO alter_table_rls;
ALTER ROLE rls_tenant_1 SET search_path TO alter_table_rls;
ALTER ROLE rls_tenant_2 SET search_path TO alter_table_rls;

GRANT USAGE ON SCHEMA alter_table_rls TO rls_table_owner;
GRANT USAGE ON SCHEMA alter_table_rls TO rls_tenant_1;
GRANT USAGE ON SCHEMA alter_table_rls TO rls_tenant_2;

--
-- The first phase tests enabling Row Level Security only after the table has been
-- turned into a distributed table.
--
-- This demonstrates that enabling Row Level Security on a distributed table correctly
-- enables Row Level Security on all shards that were in the system.
--
CREATE TABLE events (
    tenant_id int,
    id int,
    type text
);

SELECT create_distributed_table('events','tenant_id');

-- running ALTER TABLE ... OWNER TO ... only after distribution, otherwise ownership
-- information is lost.
ALTER TABLE events OWNER TO rls_table_owner;

INSERT INTO events VALUES (1,1,'push');
INSERT INTO events VALUES (2,2,'push');

-- grant access for tenants to table and shards
GRANT SELECT ON TABLE events TO rls_tenant_1;
GRANT SELECT ON TABLE events TO rls_tenant_2;

-- Base line test to verify all rows are visible
SELECT * FROM events ORDER BY 1;

-- Switch user that has been granted rights and read table
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Enable row level security
ALTER TABLE events ENABLE ROW LEVEL SECURITY;

-- Switch user to owner, all rows should be visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch user that has been granted rights, should not be able to see any rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Create policy for tenants to read access their own rows
CREATE POLICY user_mod ON events FOR SELECT TO rls_tenant_1, rls_tenant_2 USING (current_user = 'rls_tenant_' || tenant_id::text);

-- Switch user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch other user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- We will test both renaming and deletion of Policies by renaming the user_mod policy
-- and drop the renamed version. If tenants cannot see rows afterwards both the RENAME and
-- the DROP has worked correctly
ALTER POLICY user_mod ON events RENAME TO user_mod_renamed;
DROP POLICY user_mod_renamed ON events;
-- Switch to tenant user, should not see any rows after above DDL's
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Test Force Row Level Security, should also apply RLS to table owner
ALTER TABLE events FORCE ROW LEVEL SECURITY;

-- Verify all rows are still visible for admin
SELECT * FROM events ORDER BY 1;

-- Switch user to owner, no rows should be visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Allow admins to read all rows
CREATE POLICY owner_mod ON events TO rls_table_owner USING (true) WITH CHECK (true);

-- Verify all rows are visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
-- Verify the owner can insert a record
INSERT INTO events VALUES (3,3,'push');
-- See its in the table
SELECT * FROM events ORDER BY 1;
-- Verify the owner can delete a record
DELETE FROM events WHERE tenant_id = 3;
-- Verify the record is gone
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Alter the policy and verify no rows are visible for admin
ALTER POLICY owner_mod ON events USING (false);

-- Verify no rows are visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Test No Force Row Level Security, owner will not go through RLS anymore
ALTER TABLE events NO FORCE ROW LEVEL SECURITY;

-- Verify all rows are visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Disable row level security
ALTER TABLE events DISABLE ROW LEVEL SECURITY;

-- Switch user that has been granted rights and read table
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Clean up test
DROP TABLE events;

--
-- The second phase tests enables Row Level Security before the table will be turned into
-- a distributed table.
--
-- This demonstrates that tables having Row Level Security enabled before they are
-- distributed to correctly have Row Level Security enabled on the shards after
-- distributing.
--
CREATE TABLE events (
    tenant_id int,
    id int,
    type text
);

INSERT INTO events VALUES (1,1,'push');
INSERT INTO events VALUES (2,2,'push');

-- grant access for tenants to table
GRANT SELECT ON TABLE events TO rls_tenant_1;
GRANT SELECT ON TABLE events TO rls_tenant_2;

-- Base line test to verify all rows are visible
SELECT * FROM events ORDER BY 1;

-- Switch user that has been granted rights and read table
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Enable row level security
ALTER TABLE events ENABLE ROW LEVEL SECURITY;
ALTER TABLE events FORCE ROW LEVEL SECURITY;


-- create all types of policies before distribution
CREATE POLICY owner_read ON events FOR SELECT TO rls_table_owner USING (true);
CREATE POLICY owner_insert ON events FOR INSERT TO rls_table_owner WITH CHECK (false);
CREATE POLICY owner_delete ON events FOR DELETE TO rls_table_owner USING (false);
CREATE POLICY owner_update ON events FOR UPDATE TO rls_table_owner WITH CHECK (false);

-- Distribute table
SELECT create_distributed_table('events','tenant_id');

-- running ALTER TABLE ... OWNER TO ... only after distribution, otherwise ownership
-- information is lost.
ALTER TABLE events OWNER TO rls_table_owner;

SET ROLE rls_table_owner;

-- Verify owner can see all rows
SELECT * FROM events ORDER BY 1;
-- Verify owner cannot insert anything
INSERT INTO events VALUES (3,3,'push');
-- Verify owner cannot delete anything
DELETE FROM events WHERE tenant_id = 1;
-- Verify owner cannot updat anything
UPDATE events SET id = 10 WHERE tenant_id = 2;
-- Double check the table content
SELECT * FROM events ORDER BY 1;

RESET ROLE;

-- Switch user that has been granted rights, should not be able to see any rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Create policy for tenants to read access their own rows
CREATE POLICY user_mod ON events TO PUBLIC USING (current_user = 'rls_tenant_' || tenant_id::text) WITH CHECK (false);

-- Switch user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch other user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

ALTER POLICY user_mod ON events TO rls_tenant_1;

-- Switch user that has been allowed, should be able to see their own rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch other user that has been disallowed
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

ALTER POLICY user_mod ON events TO rls_tenant_1, rls_tenant_2;

-- Switch user that stayed allowed, should be able to see their own rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch other user that got allowed
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Set RLS to NO FORCE
ALTER TABLE events NO FORCE ROW LEVEL SECURITY;

-- Switch to owner to verify all rows are visible
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Disable row level security
ALTER TABLE events DISABLE ROW LEVEL SECURITY;

-- Switch user that has been granted rights and read table
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Clean up test
DROP TABLE events;

--
-- The third phase tests creates policies before distributing the tables. Only after
-- distribution RLS will be enabled.
--
-- This test demonstrates all RLS policy/FORCE settings are configured the same way even
-- if RLS is not enabled at the time of distribution.
--
CREATE TABLE events (
    tenant_id int,
    id int,
    type text
);

INSERT INTO events VALUES
  (1,1,'push'),
  (2,2,'push');

-- grant access for tenants to table
GRANT ALL ON TABLE events TO rls_tenant_1;
GRANT ALL ON TABLE events TO rls_tenant_2;

-- Base line test to verify all rows are visible
SELECT * FROM events ORDER BY 1;

-- Switch user that has been granted rights and read table
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Configure FORCE and some policies before distribution
ALTER TABLE events FORCE ROW LEVEL SECURITY;
CREATE POLICY user_mod ON events TO rls_tenant_1, rls_tenant_2
  USING (current_user = 'rls_tenant_' || tenant_id::text)
  WITH CHECK (current_user = 'rls_tenant_' || tenant_id::text AND id = 2);

-- Distribute table
SELECT create_distributed_table('events','tenant_id');
ALTER TABLE events ENABLE ROW LEVEL SECURITY;

-- running ALTER TABLE ... OWNER TO ... only after distribution, otherwise ownership
-- information is lost.
ALTER TABLE events OWNER TO rls_table_owner;

-- Verify owner cannot see any rows due to FORCE RLS
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- owner mod will only be set after distribution because it can test if FORCE has been
-- propagated during distribution
CREATE POLICY owner_mod ON events TO rls_table_owner USING (true);

-- Verify owner now can see rows
SET ROLE rls_table_owner;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_1;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- Switch other user that has been granted rights, should be able to see their own rows
SET ROLE rls_tenant_2;
SELECT * FROM events ORDER BY 1;
RESET ROLE;

-- We will test 2 different updates to the databse.
-- tenant 1 should not be able to update its record because its id is not set to 2 as
-- required by with check, tenant 2 should be able to update its record.
SET ROLE rls_tenant_1;
UPDATE events SET type = 'pull';
RESET ROLE;
SET ROLE rls_tenant_2;
UPDATE events SET type = 'pull';
RESET ROLE;
-- only items for tenant 2 should read pull in the result.
SELECT * FROM events ORDER BY 1;

-- allow all users to update their own values
ALTER POLICY user_mod ON events WITH CHECK (true);
SET ROLE rls_tenant_1;
UPDATE events SET type = 'pull tenant 1';
RESET ROLE;
SET ROLE rls_tenant_2;
UPDATE events SET type = 'pull tenant 2';
RESET ROLE;

-- records should read their own tenant pull items
SELECT * FROM events ORDER BY 1;

-- Clean up test
DROP TABLE events;

--
-- The forth phase tests some disallowed policies for distributed tables
-- slight based on example https://www.postgresql.org/docs/9.5/static/ddl-rowsecurity.html
--
CREATE TABLE groups (
  group_id int PRIMARY KEY,
  group_name text NOT NULL
);

INSERT INTO groups VALUES
  (1, 'low'),
  (2, 'medium'),
  (5, 'high');

-- definition of users' privilege levels
CREATE TABLE users (
  user_name text PRIMARY KEY,
  group_id int NOT NULL
);

INSERT INTO users VALUES
  ('alice', 5),
  ('bob', 2),
  ('mallory', 2);

-- table holding the information to be protected
CREATE TABLE information (
  info text,
  group_id int NOT NULL
);

INSERT INTO information VALUES
  ('barely secret', 1),
  ('slightly secret', 2),
  ('very secret', 5);

ALTER TABLE information ENABLE ROW LEVEL SECURITY;

-- this policy is disallowed because it has a subquery in it
CREATE POLICY fp_s ON information FOR SELECT
  USING (group_id <= (SELECT group_id FROM users WHERE user_name = current_user));

-- this attempt for distribution fails because the table has a disallowed expression
SELECT create_distributed_table('information', 'group_id');

-- DROP the expression so we can distribute the table
DROP POLICY fp_s ON information;
SELECT create_distributed_table('information', 'group_id');

-- Try and create the expression on a distributed table, this should also fail
CREATE POLICY fp_s ON information FOR SELECT
  USING (group_id <= (SELECT group_id FROM users WHERE user_name = current_user));

-- Clean up test
DROP TABLE information, groups, users;

SET citus.next_shard_id TO 1810000;
CREATE TABLE test(x int, y int);
SELECT create_distributed_table('test','x');

GRANT SELECT ON TABLE test TO rls_tenant_2;

ALTER TABLE test ENABLE ROW LEVEL SECURITY;
CREATE POLICY id_2_only
ON test
FOR SELECT TO rls_tenant_2
USING (x = 2);

INSERT INTO test SELECT i,i FROM generate_series(0,100)i;
SELECT count(*) FROM test ;

SET ROLE rls_tenant_2;
SELECT count(*) FROM test ;
RESET ROLE;

SELECT master_move_shard_placement(get_shard_id_for_distribution_column('test', 2),
    'localhost', :worker_2_port, 'localhost', :worker_1_port, shard_transfer_mode:='block_writes');

SET ROLE rls_tenant_2;
SELECT count(*) FROM test ;
RESET ROLE;

-- Show that having nondistributed table via policy is checked
BEGIN;
  CREATE TABLE table_1_check_policy (
      tenant_id int,
      id int
  );

  CREATE TABLE table_2_check_policy (
      tenant_id int,
      id int
  );

  ALTER TABLE table_1_check_policy ENABLE ROW LEVEL SECURITY;
  ALTER TABLE table_1_check_policy FORCE ROW LEVEL SECURITY;

  CREATE OR REPLACE FUNCTION func_in_transaction(param_1 int, param_2 table_2_check_policy)
  RETURNS boolean
  LANGUAGE plpgsql AS
  $$
  BEGIN
      return param_1 > 5;
  END;
  $$;

  CREATE POLICY owner_read ON table_1_check_policy FOR SELECT USING (func_in_transaction(id, NULL::table_2_check_policy));
  CREATE POLICY owner_insert ON table_1_check_policy FOR INSERT WITH CHECK (func_in_transaction(id, NULL::table_2_check_policy));

  -- It should error out
  SELECT create_distributed_table('table_1_check_policy', 'tenant_id');
ROLLBACK;

-- Restrictive policy mode must survive both immediate DDL and reconstruction.
-- These invoker helpers inspect every live placement, including empty shards.
-- Empty/missing/error results cannot satisfy either aggregate.
CREATE FUNCTION policy_mode_catalog_matches(table_name regclass)
RETURNS boolean LANGUAGE sql STRICT SECURITY INVOKER AS $function$
    SELECT COALESCE(
        (SELECT count(*) = 3
                AND count(*) FILTER (WHERE polname IN ('tenant_default', 'tenant_explicit', 'eligible')) = 3
                AND bool_and(polpermissive = (polname <> 'eligible'))
         FROM pg_policy WHERE polrelid = table_name)
        AND count(*) > 0
        AND count(*) = (SELECT count(*) FROM pg_dist_shard s
                        JOIN pg_dist_shard_placement p USING (shardid)
                        WHERE s.logicalrelid = table_name AND p.shardstate = 1)
        AND bool_and(COALESCE(success AND result = 't', false)), false)
    FROM run_command_on_placements(table_name, $command$
        SELECT count(*) = 3
               AND count(*) FILTER (WHERE polname IN ('tenant_default', 'tenant_explicit', 'eligible')) = 3
               AND bool_and(polpermissive = (polname <> 'eligible'))
        FROM pg_policy WHERE polrelid = '%s'::regclass
    $command$);
$function$;

-- master_run_on_worker opens a fresh connection as the invoking user. Check
-- that actual worker identity and privileges as well as the visible row set.
CREATE FUNCTION policy_mode_rows_match(table_name regclass, tenant integer)
RETURNS boolean LANGUAGE sql STRICT SECURITY INVOKER AS $function$
    SELECT COALESCE(count(*) > 0
        AND count(*) = (SELECT count(*) FROM pg_dist_shard s
                        JOIN pg_dist_shard_placement p USING (shardid)
                        WHERE s.logicalrelid = table_name AND p.shardstate = 1)
        AND bool_and(COALESCE(success AND result IN ('0', '1'), false))
        AND sum(CASE WHEN success AND result IN ('0', '1') THEN result::integer ELSE -1 END) = 1,
        false)
    FROM run_command_on_placements(table_name, format($command$
        SELECT CASE WHEN current_user = %1$L
            AND NOT (SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user)
            AND (SELECT relowner <> current_user::regrole FROM pg_class WHERE oid = '%%1$s'::regclass)
            AND count(*) FILTER (WHERE tenant_id <> %2$s OR eligible IS NOT TRUE) = 0
            THEN count(*) ELSE -1 END
        FROM %%1$s
    $command$, current_user, tenant));
$function$;

CREATE TABLE policy_mode_after (tenant_id int NOT NULL, id int NOT NULL, eligible boolean NOT NULL);
SELECT create_distributed_table('policy_mode_after', 'tenant_id', colocate_with => 'none');
INSERT INTO policy_mode_after VALUES (1, 11, true), (1, 12, false), (2, 21, true), (2, 22, false);
GRANT SELECT ON policy_mode_after TO rls_tenant_1, rls_tenant_2;
ALTER TABLE policy_mode_after ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_default ON policy_mode_after FOR SELECT TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY tenant_explicit ON policy_mode_after AS PERMISSIVE FOR SELECT TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY eligible ON policy_mode_after AS RESTRICTIVE FOR SELECT TO PUBLIC USING (eligible);

-- Existing restrictive policies are reconstructed with RLS enabled already.
CREATE TABLE policy_mode_before_enabled (LIKE policy_mode_after);
INSERT INTO policy_mode_before_enabled SELECT * FROM policy_mode_after;
GRANT SELECT, INSERT ON policy_mode_before_enabled TO rls_tenant_1, rls_tenant_2;
ALTER TABLE policy_mode_before_enabled ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_default ON policy_mode_before_enabled TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text)
    WITH CHECK (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY tenant_explicit ON policy_mode_before_enabled AS PERMISSIVE FOR SELECT TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY eligible ON policy_mode_before_enabled AS RESTRICTIVE TO rls_tenant_1, rls_tenant_2
    USING (eligible) WITH CHECK (eligible);
SELECT create_distributed_table('policy_mode_before_enabled', 'tenant_id', colocate_with => 'none');

-- The same mode must be retained when RLS is disabled during distribution.
CREATE TABLE policy_mode_before_disabled (LIKE policy_mode_after);
INSERT INTO policy_mode_before_disabled SELECT * FROM policy_mode_after;
GRANT SELECT, INSERT ON policy_mode_before_disabled TO rls_tenant_1, rls_tenant_2;
CREATE POLICY tenant_default ON policy_mode_before_disabled TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text)
    WITH CHECK (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY tenant_explicit ON policy_mode_before_disabled AS PERMISSIVE FOR SELECT TO rls_tenant_1, rls_tenant_2
    USING (current_user = 'rls_tenant_' || tenant_id::text);
CREATE POLICY eligible ON policy_mode_before_disabled AS RESTRICTIVE TO rls_tenant_1, rls_tenant_2
    USING (eligible) WITH CHECK (eligible);
SELECT create_distributed_table('policy_mode_before_disabled', 'tenant_id', colocate_with => 'none');
ALTER TABLE policy_mode_before_disabled ENABLE ROW LEVEL SECURITY;

SELECT policy_mode_catalog_matches('policy_mode_after'),
       policy_mode_catalog_matches('policy_mode_before_enabled'),
       policy_mode_catalog_matches('policy_mode_before_disabled');

SET ROLE rls_tenant_1;
SELECT id FROM policy_mode_after ORDER BY id;
SELECT id FROM policy_mode_before_enabled ORDER BY id;
SELECT id FROM policy_mode_before_disabled ORDER BY id;
SELECT policy_mode_rows_match('policy_mode_after', 1),
       policy_mode_rows_match('policy_mode_before_enabled', 1),
       policy_mode_rows_match('policy_mode_before_disabled', 1);
-- FOR ALL / WITH CHECK survives reconstruction; neither insert is admissible.
INSERT INTO policy_mode_before_enabled VALUES (1, 13, false);
INSERT INTO policy_mode_before_disabled VALUES (2, 23, true);
RESET ROLE;
SELECT (SELECT count(*) = 4 FROM policy_mode_before_enabled)
   AND (SELECT count(*) = 4 FROM policy_mode_before_disabled) AS rejected_inserts_unchanged;

SET ROLE rls_tenant_2;
SELECT id FROM policy_mode_after ORDER BY id;
SELECT id FROM policy_mode_before_enabled ORDER BY id;
SELECT id FROM policy_mode_before_disabled ORDER BY id;
SELECT policy_mode_rows_match('policy_mode_after', 2),
       policy_mode_rows_match('policy_mode_before_enabled', 2),
       policy_mode_rows_match('policy_mode_before_disabled', 2);
RESET ROLE;

-- Recreate a real placement, selecting its actual source instead of assuming
-- a placement order. Replication factor is one and this table is not colocated.
SELECT p.nodename AS policy_move_host, p.nodeport AS policy_move_port,
       CASE WHEN p.nodeport = :worker_1_port THEN :worker_2_port ELSE :worker_1_port END AS policy_move_target
FROM pg_dist_shard_placement p
WHERE p.shardid = get_shard_id_for_distribution_column('policy_mode_before_enabled', 1)
  AND p.shardstate = 1
\gset
SELECT master_move_shard_placement(get_shard_id_for_distribution_column('policy_mode_before_enabled', 1),
    :'policy_move_host', :policy_move_port, 'localhost', :policy_move_target,
    shard_transfer_mode := 'block_writes');
SELECT count(*) = 1 AND bool_and(nodeport = :policy_move_target) AS moved_placement_present
FROM pg_dist_shard_placement
WHERE shardid = get_shard_id_for_distribution_column('policy_mode_before_enabled', 1)
  AND shardstate = 1;
SELECT policy_mode_catalog_matches('policy_mode_before_enabled');
SET ROLE rls_tenant_1;
SELECT id FROM policy_mode_before_enabled ORDER BY id;
SELECT policy_mode_rows_match('policy_mode_before_enabled', 1);
RESET ROLE;
SET ROLE rls_tenant_2;
SELECT id FROM policy_mode_before_enabled ORDER BY id;
SELECT policy_mode_rows_match('policy_mode_before_enabled', 2);
RESET ROLE;

-- Clean up test suite
DROP SCHEMA alter_table_rls CASCADE;

DROP USER rls_table_owner;
DROP USER rls_tenant_1;
DROP USER rls_tenant_2;
