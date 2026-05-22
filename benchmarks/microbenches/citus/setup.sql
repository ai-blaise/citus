-- FEATURE: MB2 citus distributed table insert microbench setup.
CREATE EXTENSION IF NOT EXISTS citus;
DROP TABLE IF EXISTS mb2_orders;
CREATE TABLE mb2_orders (
    order_id BIGINT NOT NULL,
    customer_id INTEGER NOT NULL,
    amount NUMERIC(12, 2) NOT NULL
);
SELECT create_distributed_table('mb2_orders', 'customer_id');
