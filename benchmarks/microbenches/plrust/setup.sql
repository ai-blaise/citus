-- FEATURE: MB17 plrust call-overhead microbench setup.
CREATE EXTENSION IF NOT EXISTS plrust;
CREATE OR REPLACE FUNCTION mb17_plrust_add(a INT, b INT) RETURNS INT
LANGUAGE plrust AS $$
    Ok(Some(a.unwrap_or(0) + b.unwrap_or(0)))
$$;
