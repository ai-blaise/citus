-- citus--14.0-1--15.0-1
-- bump version to 15.0-1

#include "udfs/citus_internal_get_next_colocation_id/15.0-1.sql"

-- drop the legacy version that we kept for backward compatibility at Citus 13 and 14
DROP FUNCTION IF EXISTS pg_catalog.worker_adjust_identity_column_seq_ranges(regclass);
#include "udfs/citus_internal_adjust_identity_column_seq_settings/15.0-1.sql"

-- drop the legacy version that we kept for backward compatibility at Citus 13 and 14
DROP FUNCTION IF EXISTS pg_catalog.worker_apply_sequence_command(text, regtype);
#include "udfs/worker_apply_sequence_command/15.0-1.sql"

#include "udfs/citus_internal_lock_colocation_id/15.0-1.sql"

#include "udfs/citus_internal_acquire_placement_colocation_lock/15.0-1.sql"

-- cluster changes block UDFs
#include "udfs/citus_cluster_changes_block/15.0-1.sql"
#include "udfs/citus_cluster_changes_unblock/15.0-1.sql"
#include "udfs/citus_cluster_changes_block_status/15.0-1.sql"

-- placement-generation UDF for pool-side plan-cache invalidation (FEATURE: T2)
#include "udfs/citus_placement_generation/15.0-1.sql"

-- cohabit clock-reservation UDF for pg_cron startup proof (FEATURE: TS19)
#include "udfs/citus_cohabit_clock_tick_reserved/15.0-1.sql"

-- cohabit extension classifier/configuration UDFs (FEATURE: TS20)
#include "udfs/citus_cohabit_extension_role/15.0-1.sql"
#include "udfs/citus_cohabit_extension_configured/15.0-1.sql"
