-- FEATURE: MB14 pg_graphql join microbench setup.
CREATE EXTENSION IF NOT EXISTS pg_graphql;
DROP TABLE IF EXISTS mb14_orders CASCADE;
DROP TABLE IF EXISTS mb14_customers CASCADE;
CREATE TABLE mb14_customers (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE mb14_orders (
    id BIGINT PRIMARY KEY,
    customer_id BIGINT NOT NULL REFERENCES mb14_customers(id),
    amount NUMERIC(12, 2) NOT NULL
);
INSERT INTO mb14_customers
SELECT s, 'customer-' || s FROM generate_series(1, 1000) s;
INSERT INTO mb14_orders
SELECT s, (s % 1000) + 1, (random() * 1000)::NUMERIC(12, 2)
FROM generate_series(1, 10000) s;
