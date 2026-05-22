DO $$
DECLARE
    i INT;
BEGIN
    FOR i IN 1..:row_count LOOP
        PERFORM cron.schedule(
            format('mb4-%s', i),
            '* * * * *',
            format('SELECT %s', i)
        );
    END LOOP;
END $$;
