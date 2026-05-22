INSERT INTO mb13_docs
SELECT s,
       'title-' || s,
       md5(s::TEXT) || ' ' || md5((s + 1)::TEXT)
FROM generate_series(1, :row_count) s;
CREATE INDEX mb13_docs_bm25 ON mb13_docs
  USING bm25 (id, title, body)
  WITH (key_field='id');
SELECT id FROM mb13_docs
  WHERE body @@@ 'title-1'
  LIMIT 10;
