-- FEATURE: R11
-- pg_warm: ai-blaise replica cold-start cache warming surface.
--
-- There is no upstream "pg_warm" extension on PGDG; cache warming is provided
-- by the core contrib extension pg_prewarm. This local source-built extension
-- exposes a thin, stable surface named pg_warm so the V2 extension catalog and
-- bundled-extension manifest can list pg_warm by name and the operand-image
-- smoke can verify CREATE EXTENSION pg_warm; works against a real PostgreSQL
-- server. Behavior delegates to pg_prewarm, with safe defaults for the
-- prewarm mode and fork.

CREATE FUNCTION pg_warm(
    relation regclass,
    mode text DEFAULT 'buffer',
    fork text DEFAULT 'main',
    first_block bigint DEFAULT NULL,
    last_block bigint DEFAULT NULL
)
RETURNS bigint
LANGUAGE sql
AS $$
    SELECT pg_prewarm(relation, mode, fork, first_block, last_block);
$$;

COMMENT ON FUNCTION pg_warm(regclass, text, text, bigint, bigint) IS
'ai-blaise pg_warm (R11) replica cold-start cache warming, delegates to pg_prewarm.';

CREATE FUNCTION pg_warm_relations(relations regclass[])
RETURNS TABLE(relation regclass, blocks_prewarmed bigint)
LANGUAGE sql
AS $$
    SELECT rel, pg_prewarm(rel)
    FROM unnest(relations) AS rel;
$$;

COMMENT ON FUNCTION pg_warm_relations(regclass[]) IS
'Warm a batch of relations into the shared buffer cache via pg_prewarm.';
