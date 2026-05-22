-- Citus-aware TPC-C schema distribution.
--
-- Applied after the benchbase TPC-C loader has created the base tables. Each
-- distributed table is co-located on `w_id` so warehouse-keyed transactions
-- (NewOrder, Payment, Delivery) execute on a single shard. The customer,
-- stock, order, order_line, and history tables join on the same warehouse key
-- so colocation makes the joins shard-local. The item table is a reference
-- table because all warehouses share the catalog.

SET citus.shard_count = 32;
SET citus.shard_replication_factor = 1;

SELECT create_distributed_table('warehouse', 'w_id');
SELECT create_distributed_table('district', 'd_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('customer', 'c_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('stock', 's_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('orders', 'o_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('new_order', 'no_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('order_line', 'ol_w_id', colocate_with => 'warehouse');
SELECT create_distributed_table('history', 'h_w_id', colocate_with => 'warehouse');

SELECT create_reference_table('item');
