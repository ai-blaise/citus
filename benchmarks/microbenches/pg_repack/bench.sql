-- pg_repack runs out-of-band; this script just registers the
-- target table. The bench.sh wrapper invokes the pg_repack CLI.
SELECT relname FROM pg_class WHERE relname = 'mb20_bloated';
