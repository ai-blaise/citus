CREATE INDEX mb26_docs_rum ON mb26_docs
  USING rum (body_tsv rum_tsvector_ops);
SELECT count(*) FROM mb26_docs
WHERE body_tsv @@ to_tsquery('english', 'aaaaa');
