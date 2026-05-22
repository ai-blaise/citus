-- FEATURE: MB25 citext lookup microbench setup.
CREATE EXTENSION IF NOT EXISTS citext;
DROP TABLE IF EXISTS mb25_users;
CREATE TABLE mb25_users (id BIGSERIAL PRIMARY KEY, email CITEXT NOT NULL);
INSERT INTO mb25_users (email)
SELECT md5(s::TEXT) || '@example.com'
FROM generate_series(1, 100000) s;
CREATE INDEX mb25_users_email ON mb25_users (email);
ANALYZE mb25_users;
