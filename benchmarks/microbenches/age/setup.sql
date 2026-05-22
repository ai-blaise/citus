-- FEATURE: MB16 Apache AGE Cypher microbench setup.
CREATE EXTENSION IF NOT EXISTS age;
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT create_graph('mb16_g');
