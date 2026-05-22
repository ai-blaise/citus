INSERT INTO mb7_cipher (id, ciphertext)
SELECT id,
       pgsodium.crypto_secretbox(
         payload::BYTEA,
         pgsodium.randombytes_buf(24),
         pgsodium.crypto_secretbox_keygen()
       )
FROM mb7_plain;
