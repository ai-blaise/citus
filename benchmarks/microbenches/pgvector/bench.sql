INSERT INTO mb3_vectors
SELECT s, (
  SELECT array_agg(random())::vector(768)
  FROM generate_series(1, 768)
)
FROM generate_series(1, :row_count) s;
CREATE INDEX mb3_vectors_ivfflat ON mb3_vectors
  USING ivfflat (embedding vector_l2_ops) WITH (lists = 32);
-- Lookup pass (results discarded; we measure end-to-end time).
SELECT id FROM mb3_vectors
  ORDER BY embedding <-> (SELECT embedding FROM mb3_vectors LIMIT 1)
  LIMIT 10;
