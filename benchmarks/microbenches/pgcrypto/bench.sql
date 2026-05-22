INSERT INTO mb23_cipher (id, ciphertext)
SELECT id, pgp_sym_encrypt(payload, 'mb23-key')
FROM mb23_plain;
