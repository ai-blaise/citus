-- FEATURE: MB23 pgcrypto pgp_sym_encrypt microbench setup.
CREATE EXTENSION IF NOT EXISTS pgcrypto;
DROP TABLE IF EXISTS mb23_plain;
DROP TABLE IF EXISTS mb23_cipher;
CREATE TABLE mb23_plain (id BIGSERIAL PRIMARY KEY, payload TEXT NOT NULL);
CREATE TABLE mb23_cipher (id BIGINT PRIMARY KEY, ciphertext BYTEA NOT NULL);
INSERT INTO mb23_plain (payload)
SELECT md5(s::TEXT) FROM generate_series(1, 10000) s;
