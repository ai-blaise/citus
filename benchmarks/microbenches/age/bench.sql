LOAD 'age';
SET search_path = ag_catalog, public;
SELECT * FROM cypher('mb16_g', $$
    UNWIND range(1, 1000) AS i
    CREATE (n:Node {id: i})
    RETURN count(n)
$$) AS (n agtype);
SELECT * FROM cypher('mb16_g', $$
    MATCH (a:Node)-[*1..2]-(b:Node)
    WHERE a.id < 100 AND b.id < 100
    RETURN count(*)
$$) AS (n agtype);
