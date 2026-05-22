-- FEATURE: MB18 plv8 call-overhead microbench setup.
CREATE EXTENSION IF NOT EXISTS plv8;
CREATE OR REPLACE FUNCTION mb18_plv8_add(a INT, b INT) RETURNS INT
LANGUAGE plv8 AS $$
    return a + b;
$$;
