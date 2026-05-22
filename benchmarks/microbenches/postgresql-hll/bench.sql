SELECT hll_cardinality(hll_add_agg(hll_hash_bigint(user_id)))
FROM mb8_events;
