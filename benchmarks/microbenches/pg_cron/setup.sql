-- FEATURE: MB4 pg_cron schedule overhead microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_cron;
-- Clean any prior microbench jobs (deterministic prefix).
SELECT cron.unschedule(jobid)
  FROM cron.job
  WHERE jobname LIKE 'mb4-%';
