-- FEATURE: MB7 pgsodium libsodium encrypt microbench setup.
CREATE EXTENSION IF NOT EXISTS pgsodium;
DROP TABLE IF EXISTS mb7_plain;
DROP TABLE IF EXISTS mb7_cipher;
CREATE TABLE mb7_plain (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
CREATE TABLE mb7_cipher (id BIGINT PRIMARY KEY, ciphertext BYTEA NOT NULL);
INSERT INTO mb7_plain (payload)
SELECT md5(s::TEXT) FROM generate_series(1, 1000) s;
