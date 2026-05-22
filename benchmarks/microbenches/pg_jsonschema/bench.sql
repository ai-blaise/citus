SELECT count(*) FROM mb15_docs
WHERE jsonb_matches_schema(
  '{"type":"object","required":["id","name","value"],"properties":{"id":{"type":"integer"},"name":{"type":"string"},"value":{"type":"number"}}}'::JSON,
  doc
);
