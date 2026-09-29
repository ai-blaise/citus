# ATTRIBUTIONS - Rust

This is a dated snapshot of package-declared license metadata, generated on
2026-09-05 with `cargo metadata --locked --format-version 1`. It includes the
resolved graph, including target-specific and development dependencies; it is
not a list of packages proven present in a particular runtime image.

`Cargo.lock` SHA-256:
`0c42508f54a8e7c79de119e6fdbca7fae8ba2a9fc72d8c3ad602962cd5ed2a5e`.

The graph contains 564 packages and 33 workspace members. Different
versions of a package are separate entries. `ci/ai-blaise/license-check.sh`
checks locked metadata and the existing repository dependency policy; it does
not currently regenerate or compare this table. Regenerate the snapshot after
dependency changes. These declarations do not replace review of exact source
licenses, bundled notices, corresponding source, or distribution conditions.

## Ported source attributions

The `ai_blaise_citus_pool_wire` crate has a separately recorded MIT attribution
for PostgreSQL wire-protocol work derived from `jackc/pgx` (`pgproto3`). It is
not a Cargo dependency. Preserve its copyright notice in
[pool/wire/THIRD_PARTY_NOTICES.md](pool/wire/THIRD_PARTY_NOTICES.md).

## License boundary

The workspace package license is AGPL-3.0. The repository's current Rust
dependency scanner rejects expressions containing GPL-2.0 or GPL-3.0 unless
they also contain AGPL or LGPL. That is the existing repository policy, not a
general compatibility rule or permission to relicense third-party code.
Selecting a license from an alternative expression and satisfying its
conditions remain release-review work. The scan does not qualify the image's
non-Rust components or establish the Timescale licensing choice.

## Package counts by declared expression

| Declared expression | Packages |
| --- | ---: |
| (Apache-2.0 OR MIT) AND BSD-3-Clause | 1 |
| (MIT OR Apache-2.0) AND Apache-2.0 | 1 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 |
| 0BSD OR MIT OR Apache-2.0 | 1 |
| AGPL-3.0 | 33 |
| Apache-2.0 | 61 |
| Apache-2.0 AND ISC | 1 |
| Apache-2.0 AND MIT | 1 |
| Apache-2.0 OR BSL-1.0 | 1 |
| Apache-2.0 OR BSL-1.0 OR MIT | 2 |
| Apache-2.0 OR ISC OR MIT | 3 |
| Apache-2.0 OR MIT | 35 |
| Apache-2.0 WITH LLVM-exception | 1 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 16 |
| BSD-2-Clause | 1 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 2 |
| BSD-3-Clause | 6 |
| BSD-3-Clause AND MIT | 1 |
| BSD-3-Clause/MIT | 1 |
| CC0-1.0 | 1 |
| CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | 1 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |
| CDLA-Permissive-2.0 | 3 |
| ISC | 2 |
| MIT | 79 |
| MIT AND BSD-3-Clause | 1 |
| MIT OR Apache-2.0 | 229 |
| MIT OR Apache-2.0 OR BSD-1-Clause | 1 |
| MIT OR Apache-2.0 OR LGPL-2.1-or-later | 2 |
| MIT OR Apache-2.0 OR Zlib | 2 |
| MIT OR Zlib OR Apache-2.0 | 1 |
| MIT/Apache-2.0 | 36 |
| Unicode-3.0 | 18 |
| Unlicense OR MIT | 7 |
| Unlicense/MIT | 4 |
| Zlib | 3 |
| Zlib OR Apache-2.0 OR MIT | 3 |
| bzip2-1.0.6 | 1 |
| **Total** | **564** |

## Regenerating

Run the locked metadata command from the repository root. Sort packages by
name and version, retain each declared expression verbatim, count packages
per expression, and record the lockfile checksum. Review the actual license
files for the exact candidate before distribution; Cargo declarations alone
are not a completed legal-text inventory.

```sh
cargo metadata --locked --format-version 1 \
  | jq -r '.packages[] | [.name, .version, (.license // "Not declared"), (.repository // "Not declared")] | @tsv' \
  | LC_ALL=C sort
shasum -a 256 Cargo.lock
```

## Resolved packages

| Package | Version | Declared license | Repository |
| --- | --- | --- | --- |
| `adler2` | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | [upstream](https://github.com/oyvindln/adler2) |
| `ahash` | 0.8.12 | MIT OR Apache-2.0 | [upstream](https://github.com/tkaitchuck/ahash) |
| `aho-corasick` | 1.1.4 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/aho-corasick) |
| `ai_blaise_citus_admin` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_auth_introspection_client` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_companion` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_e2e` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_lsp` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_mcp` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_operator` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_pool` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_pool_wire` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_schema_designer` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_analytical` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_auth` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_backup` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_cdc` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_coldtier` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_edge_functions` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_graphql` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_hlc` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_mcp` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_postgrest` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_raft` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_realtime` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_repack` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_schema_job` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_shared` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_storage` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_txn_status` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_sidecar_vectorizer` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_tool_runtime` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_tui` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citus_watch` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_citusctl` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `ai_blaise_feature_register` | 0.1.0 | AGPL-3.0 | [upstream](https://github.com/ai-blaise/citus) |
| `alloc-no-stdlib` | 2.0.4 | BSD-3-Clause | [upstream](https://github.com/dropbox/rust-alloc-no-stdlib) |
| `alloc-stdlib` | 0.2.2 | BSD-3-Clause | [upstream](https://github.com/dropbox/rust-alloc-no-stdlib) |
| `allocator-api2` | 0.2.21 | MIT OR Apache-2.0 | [upstream](https://github.com/zakarumych/allocator-api2) |
| `android_system_properties` | 0.1.5 | MIT/Apache-2.0 | [upstream](https://github.com/nical/android_system_properties) |
| `annotate-snippets` | 0.12.16 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/annotate-snippets-rs) |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-cli/anstyle.git) |
| `anyhow` | 1.0.104 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/anyhow) |
| `ar_archive_writer` | 0.5.1 | Apache-2.0 WITH LLVM-exception | [upstream](https://github.com/rust-lang/ar_archive_writer) |
| `arraydeque` | 0.5.1 | MIT/Apache-2.0 | [upstream](https://github.com/andylokandy/arraydeque) |
| `arrayref` | 0.3.9 | BSD-2-Clause | [upstream](https://github.com/droundy/arrayref) |
| `arrayvec` | 0.7.6 | MIT OR Apache-2.0 | [upstream](https://github.com/bluss/arrayvec) |
| `arrow` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-arith` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-array` | 59.3.0 | Apache-2.0 AND MIT | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-buffer` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-cast` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-csv` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-data` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-ipc` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-json` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-ord` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-row` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-schema` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-select` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `arrow-string` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `asn1-rs` | 0.6.2 | MIT OR Apache-2.0 | [upstream](https://github.com/rusticata/asn1-rs.git) |
| `asn1-rs-derive` | 0.5.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rusticata/asn1-rs.git) |
| `asn1-rs-impl` | 0.2.0 | MIT/Apache-2.0 | [upstream](https://github.com/rusticata/asn1-rs.git) |
| `async-broadcast` | 0.7.2 | MIT OR Apache-2.0 | [upstream](https://github.com/smol-rs/async-broadcast) |
| `async-compression` | 0.4.44 | MIT OR Apache-2.0 | [upstream](https://github.com/Nullus157/async-compression) |
| `async-lock` | 3.4.2 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/async-lock) |
| `async-nats` | 0.49.0 | Apache-2.0 | [upstream](https://github.com/nats-io/nats.rs) |
| `async-stream` | 0.3.6 | MIT | [upstream](https://github.com/tokio-rs/async-stream) |
| `async-stream-impl` | 0.3.6 | MIT | [upstream](https://github.com/tokio-rs/async-stream) |
| `async-trait` | 0.1.89 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/async-trait) |
| `atoi` | 2.0.0 | MIT | [upstream](https://github.com/pacman82/atoi-rs) |
| `atomic-waker` | 1.1.2 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/atomic-waker) |
| `autocfg` | 1.5.1 | Apache-2.0 OR MIT | [upstream](https://github.com/cuviper/autocfg) |
| `axum` | 0.7.9 | MIT | [upstream](https://github.com/tokio-rs/axum) |
| `axum-core` | 0.4.5 | MIT | [upstream](https://github.com/tokio-rs/axum) |
| `backon` | 1.6.0 | Apache-2.0 | [upstream](https://github.com/Xuanwo/backon) |
| `base64` | 0.22.1 | MIT OR Apache-2.0 | [upstream](https://github.com/marshallpierce/rust-base64) |
| `base64` | 0.23.1 | MIT OR Apache-2.0 | [upstream](https://github.com/marshallpierce/rust-base64) |
| `base64ct` | 1.8.3 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats) |
| `bigdecimal` | 0.4.10 | MIT/Apache-2.0 | [upstream](https://github.com/akubera/bigdecimal-rs) |
| `bitflags` | 1.3.2 | MIT/Apache-2.0 | [upstream](https://github.com/bitflags/bitflags) |
| `bitflags` | 2.11.1 | MIT OR Apache-2.0 | [upstream](https://github.com/bitflags/bitflags) |
| `blake2` | 0.10.6 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/hashes) |
| `blake3` | 1.8.5 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | [upstream](https://github.com/BLAKE3-team/BLAKE3) |
| `block-buffer` | 0.10.4 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/utils) |
| `block-buffer` | 0.12.0 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/utils) |
| `brotli` | 8.0.2 | BSD-3-Clause AND MIT | [upstream](https://github.com/dropbox/rust-brotli) |
| `brotli-decompressor` | 5.0.0 | BSD-3-Clause/MIT | [upstream](https://github.com/dropbox/rust-brotli-decompressor) |
| `bumpalo` | 3.20.2 | MIT OR Apache-2.0 | [upstream](https://github.com/fitzgen/bumpalo) |
| `byteorder` | 1.5.0 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/byteorder) |
| `bytes` | 1.11.1 | MIT | [upstream](https://github.com/tokio-rs/bytes) |
| `bzip2` | 0.6.1 | MIT OR Apache-2.0 | [upstream](https://github.com/trifectatechfoundation/bzip2-rs) |
| `cc` | 1.2.62 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/cc-rs) |
| `cfg-if` | 1.0.4 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/cfg-if) |
| `cfg_aliases` | 0.2.1 | MIT | [upstream](https://github.com/katharostech/cfg_aliases) |
| `chacha20` | 0.10.2 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/stream-ciphers) |
| `chrono` | 0.4.45 | MIT OR Apache-2.0 | [upstream](https://github.com/chronotope/chrono) |
| `chrono-tz` | 0.10.4 | MIT OR Apache-2.0 | [upstream](https://github.com/chronotope/chrono-tz) |
| `cmov` | 0.5.3 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/utils) |
| `combine` | 4.6.8 | MIT | [upstream](https://github.com/Marwes/combine) |
| `comfy-table` | 7.2.2 | MIT | [upstream](https://github.com/nukesor/comfy-table) |
| `compression-codecs` | 0.4.39 | MIT OR Apache-2.0 | [upstream](https://github.com/Nullus157/async-compression) |
| `compression-core` | 0.4.33 | MIT OR Apache-2.0 | [upstream](https://github.com/Nullus157/async-compression) |
| `const-oid` | 0.10.2 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats) |
| `const-oid` | 0.9.6 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats/tree/master/const-oid) |
| `const-random` | 0.1.18 | MIT OR Apache-2.0 | [upstream](https://github.com/tkaitchuck/constrandom) |
| `const-random-macro` | 0.1.16 | MIT OR Apache-2.0 | [upstream](https://github.com/tkaitchuck/constrandom) |
| `constant_time_eq` | 0.4.2 | CC0-1.0 OR MIT-0 OR Apache-2.0 | [upstream](https://github.com/cesarb/constant_time_eq) |
| `core-foundation` | 0.10.1 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/core-foundation-rs) |
| `core-foundation-sys` | 0.8.7 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/core-foundation-rs) |
| `cpufeatures` | 0.2.17 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/utils) |
| `cpufeatures` | 0.3.0 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/utils) |
| `crc32fast` | 1.5.0 | MIT OR Apache-2.0 | [upstream](https://github.com/srijs/rust-crc32fast) |
| `crossbeam-channel` | 0.5.15 | MIT OR Apache-2.0 | [upstream](https://github.com/crossbeam-rs/crossbeam) |
| `crossbeam-epoch` | 0.9.20 | MIT OR Apache-2.0 | [upstream](https://github.com/crossbeam-rs/crossbeam) |
| `crossbeam-utils` | 0.8.21 | MIT OR Apache-2.0 | [upstream](https://github.com/crossbeam-rs/crossbeam) |
| `crunchy` | 0.2.4 | MIT | [upstream](https://github.com/eira-fransham/crunchy) |
| `crypto-common` | 0.1.7 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/traits) |
| `crypto-common` | 0.2.2 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/traits) |
| `csv` | 1.4.0 | Unlicense/MIT | [upstream](https://github.com/BurntSushi/rust-csv) |
| `csv-core` | 0.1.13 | Unlicense/MIT | [upstream](https://github.com/BurntSushi/rust-csv) |
| `ctutils` | 0.4.2 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/utils) |
| `curve25519-dalek` | 4.1.3 | BSD-3-Clause | [upstream](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/curve25519-dalek) |
| `curve25519-dalek-derive` | 0.1.1 | MIT/Apache-2.0 | [upstream](https://github.com/dalek-cryptography/curve25519-dalek) |
| `darling` | 0.23.0 | MIT | [upstream](https://github.com/TedDriggs/darling) |
| `darling_core` | 0.23.0 | MIT | [upstream](https://github.com/TedDriggs/darling) |
| `darling_macro` | 0.23.0 | MIT | [upstream](https://github.com/TedDriggs/darling) |
| `dashmap` | 6.2.1 | MIT | [upstream](https://github.com/xacrimon/dashmap) |
| `data-encoding` | 2.11.0 | MIT | [upstream](https://github.com/ia0/data-encoding) |
| `datafusion` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-catalog` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-catalog-listing` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-common` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-common-runtime` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-datasource` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-datasource-arrow` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-datasource-csv` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-datasource-json` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-datasource-parquet` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-doc` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-execution` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-expr` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-expr-common` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-aggregate` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-aggregate-common` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-nested` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-table` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-window` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-functions-window-common` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-macros` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-optimizer` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-physical-expr` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-physical-expr-adapter` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-physical-expr-common` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-physical-optimizer` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-physical-plan` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-pruning` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-session` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `datafusion-sql` | 55.0.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion) |
| `defmt` | 1.1.1 | MIT OR Apache-2.0 | [upstream](https://github.com/knurling-rs/defmt) |
| `defmt-macros` | 1.1.1 | MIT OR Apache-2.0 | [upstream](https://github.com/knurling-rs/defmt) |
| `defmt-parser` | 1.0.0 | MIT OR Apache-2.0 | [upstream](https://github.com/knurling-rs/defmt) |
| `der` | 0.7.10 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats/tree/master/der) |
| `der-parser` | 9.0.0 | MIT/Apache-2.0 | [upstream](https://github.com/rusticata/der-parser.git) |
| `deranged` | 0.5.8 | MIT OR Apache-2.0 | [upstream](https://github.com/jhpratt/deranged) |
| `derive_more` | 2.1.1 | MIT | [upstream](https://github.com/JelteF/derive_more) |
| `derive_more-impl` | 2.1.1 | MIT | [upstream](https://github.com/JelteF/derive_more) |
| `digest` | 0.10.7 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/traits) |
| `digest` | 0.11.3 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/traits) |
| `displaydoc` | 0.2.5 | MIT OR Apache-2.0 | [upstream](https://github.com/yaahc/displaydoc) |
| `dyn-clone` | 1.0.20 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/dyn-clone) |
| `ed25519` | 2.2.3 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/signatures/tree/master/ed25519) |
| `ed25519-dalek` | 2.2.0 | BSD-3-Clause | [upstream](https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek) |
| `educe` | 0.6.0 | MIT | [upstream](https://github.com/magiclen/educe) |
| `either` | 1.15.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rayon-rs/either) |
| `encoding_rs` | 0.8.35 | (Apache-2.0 OR MIT) AND BSD-3-Clause | [upstream](https://github.com/hsivonen/encoding_rs) |
| `encoding_rs_io` | 0.1.8 | MIT OR Apache-2.0 | [upstream](https://github.com/BurntSushi/encoding_rs_io) |
| `enum-ordinalize` | 4.3.2 | MIT | [upstream](https://github.com/magiclen/enum-ordinalize) |
| `enum-ordinalize-derive` | 4.3.2 | MIT | [upstream](https://github.com/magiclen/enum-ordinalize) |
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT | [upstream](https://github.com/indexmap-rs/equivalent) |
| `errno` | 0.3.14 | MIT OR Apache-2.0 | [upstream](https://github.com/lambda-fairy/rust-errno) |
| `event-listener` | 5.4.2 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/event-listener) |
| `event-listener-strategy` | 0.5.4 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/event-listener-strategy) |
| `fallible-iterator` | 0.2.0 | MIT/Apache-2.0 | [upstream](https://github.com/sfackler/rust-fallible-iterator) |
| `fastrand` | 2.4.1 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/fastrand) |
| `fiat-crypto` | 0.2.9 | MIT OR Apache-2.0 OR BSD-1-Clause | [upstream](https://github.com/mit-plv/fiat-crypto) |
| `find-msvc-tools` | 0.1.9 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/cc-rs) |
| `fixedbitset` | 0.5.7 | MIT OR Apache-2.0 | [upstream](https://github.com/petgraph/fixedbitset) |
| `flatbuffers` | 25.12.19 | Apache-2.0 | [upstream](https://github.com/google/flatbuffers) |
| `flate2` | 1.1.9 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/flate2-rs) |
| `foldhash` | 0.1.5 | Zlib | [upstream](https://github.com/orlp/foldhash) |
| `foldhash` | 0.2.0 | Zlib | [upstream](https://github.com/orlp/foldhash) |
| `foreign-types` | 0.3.2 | MIT/Apache-2.0 | [upstream](https://github.com/sfackler/foreign-types) |
| `foreign-types-shared` | 0.1.1 | MIT/Apache-2.0 | [upstream](https://github.com/sfackler/foreign-types) |
| `form_urlencoded` | 1.2.2 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-url) |
| `futures` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-channel` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-core` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-executor` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-io` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-macro` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-sink` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-task` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `futures-timer` | 3.0.4 | MIT/Apache-2.0 | [upstream](https://github.com/async-rs/futures-timer) |
| `futures-util` | 0.3.32 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/futures-rs) |
| `generic-array` | 0.14.7 | MIT | [upstream](https://github.com/fizyk20/generic-array.git) |
| `getrandom` | 0.2.17 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/getrandom) |
| `getrandom` | 0.3.4 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/getrandom) |
| `getrandom` | 0.4.2 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/getrandom) |
| `glob` | 0.3.3 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/glob) |
| `gloo-timers` | 0.3.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rustwasm/gloo/tree/master/crates/timers) |
| `governor` | 0.7.0 | MIT | [upstream](https://github.com/boinkor-net/governor.git) |
| `granit-parser` | 0.0.7 | MIT OR Apache-2.0 | [upstream](https://github.com/bourumir-wyngs/granit-parser) |
| `half` | 2.7.1 | MIT OR Apache-2.0 | [upstream](https://github.com/VoidStarKat/half-rs) |
| `hashbrown` | 0.14.5 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/hashbrown) |
| `hashbrown` | 0.15.5 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/hashbrown) |
| `hashbrown` | 0.16.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/hashbrown) |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/hashbrown) |
| `heck` | 0.5.0 | MIT OR Apache-2.0 | [upstream](https://github.com/withoutboats/heck) |
| `hex` | 0.4.3 | MIT OR Apache-2.0 | [upstream](https://github.com/KokaKiwi/rust-hex) |
| `hmac` | 0.13.0 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/MACs) |
| `hostname` | 0.4.2 | MIT | [upstream](https://github.com/djc/hostname) |
| `http` | 1.4.0 | MIT OR Apache-2.0 | [upstream](https://github.com/hyperium/http) |
| `http-body` | 1.0.1 | MIT | [upstream](https://github.com/hyperium/http-body) |
| `http-body-util` | 0.1.3 | MIT | [upstream](https://github.com/hyperium/http-body) |
| `httparse` | 1.10.1 | MIT OR Apache-2.0 | [upstream](https://github.com/seanmonstar/httparse) |
| `httpdate` | 1.0.3 | MIT OR Apache-2.0 | [upstream](https://github.com/pyfisch/httpdate) |
| `humantime` | 2.3.0 | MIT OR Apache-2.0 | [upstream](https://github.com/chronotope/humantime) |
| `hybrid-array` | 0.4.12 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/hybrid-array) |
| `hyper` | 1.9.0 | MIT | [upstream](https://github.com/hyperium/hyper) |
| `hyper-rustls` | 0.27.9 | Apache-2.0 OR ISC OR MIT | [upstream](https://github.com/rustls/hyper-rustls) |
| `hyper-timeout` | 0.5.2 | MIT OR Apache-2.0 | [upstream](https://github.com/hjr3/hyper-timeout) |
| `hyper-util` | 0.1.20 | MIT | [upstream](https://github.com/hyperium/hyper-util) |
| `iana-time-zone` | 0.1.65 | MIT OR Apache-2.0 | [upstream](https://github.com/strawlab/iana-time-zone) |
| `iana-time-zone-haiku` | 0.1.2 | MIT OR Apache-2.0 | [upstream](https://github.com/strawlab/iana-time-zone) |
| `icu_collections` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_locale_core` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_normalizer` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_normalizer_data` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_properties` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_properties_data` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `icu_provider` | 2.2.0 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `id-arena` | 2.3.0 | MIT/Apache-2.0 | [upstream](https://github.com/fitzgen/id-arena) |
| `ident_case` | 1.0.1 | MIT/Apache-2.0 | [upstream](https://github.com/TedDriggs/ident_case) |
| `idna` | 1.1.0 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-url/) |
| `idna_adapter` | 1.2.2 | Apache-2.0 OR MIT | [upstream](https://github.com/hsivonen/idna_adapter) |
| `indexmap` | 2.14.0 | Apache-2.0 OR MIT | [upstream](https://github.com/indexmap-rs/indexmap) |
| `ipnet` | 2.12.0 | MIT OR Apache-2.0 | [upstream](https://github.com/krisprice/ipnet) |
| `itertools` | 0.14.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-itertools/itertools) |
| `itertools` | 0.15.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-itertools/itertools) |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/itoa) |
| `jiff` | 0.2.35 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/jiff) |
| `jiff-core` | 0.1.0 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/jiff) |
| `jiff-static` | 0.2.35 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/jiff) |
| `jni` | 0.22.4 | MIT OR Apache-2.0 | [upstream](https://github.com/jni-rs/jni-rs) |
| `jni-macros` | 0.22.4 | MIT OR Apache-2.0 | [upstream](https://github.com/jni-rs/jni-rs) |
| `jni-sys` | 0.4.1 | MIT OR Apache-2.0 | [upstream](https://github.com/jni-rs/jni-sys) |
| `jni-sys-macros` | 0.4.1 | MIT OR Apache-2.0 | [upstream](https://github.com/jni-rs/jni-sys) |
| `jobserver` | 0.1.34 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/jobserver-rs) |
| `js-sys` | 0.3.98 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys) |
| `json-patch` | 4.2.0 | MIT/Apache-2.0 | [upstream](https://github.com/idubrov/json-patch) |
| `jsonpath-rust` | 1.0.10 | MIT | [upstream](https://github.com/besok/jsonpath-rust) |
| `jsonptr` | 0.7.1 | MIT OR Apache-2.0 | [upstream](https://github.com/chanced/jsonptr) |
| `k8s-openapi` | 0.28.0 | Apache-2.0 | [upstream](https://github.com/Arnavion/k8s-openapi) |
| `kube` | 4.2.0 | Apache-2.0 | [upstream](https://github.com/kube-rs/kube) |
| `kube-client` | 4.2.0 | Apache-2.0 | [upstream](https://github.com/kube-rs/kube) |
| `kube-core` | 4.2.0 | Apache-2.0 | [upstream](https://github.com/kube-rs/kube) |
| `kube-derive` | 4.2.0 | Apache-2.0 | [upstream](https://github.com/kube-rs/kube) |
| `kube-runtime` | 4.2.0 | Apache-2.0 | [upstream](https://github.com/kube-rs/kube) |
| `lazy_static` | 1.5.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang-nursery/lazy-static.rs) |
| `leb128fmt` | 0.1.0 | MIT OR Apache-2.0 | [upstream](https://github.com/bluk/leb128fmt) |
| `lexical-core` | 1.0.6 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `lexical-parse-float` | 1.0.6 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `lexical-parse-integer` | 1.0.6 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `lexical-util` | 1.0.7 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `lexical-write-float` | 1.0.6 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `lexical-write-integer` | 1.0.6 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/rust-lexical) |
| `libbz2-rs-sys` | 0.2.5 | bzip2-1.0.6 | [upstream](https://github.com/trifectatechfoundation/libbzip2-rs) |
| `libc` | 0.2.186 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/libc) |
| `liblzma` | 0.4.8 | MIT OR Apache-2.0 | [upstream](https://github.com/portable-network-archive/liblzma-rs) |
| `liblzma-sys` | 0.4.8 | MIT OR Apache-2.0 | [upstream](https://github.com/portable-network-archive/liblzma-rs) |
| `libm` | 0.2.16 | MIT | [upstream](https://github.com/rust-lang/compiler-builtins) |
| `libredox` | 0.1.16 | MIT | [upstream](https://gitlab.redox-os.org/redox-os/libredox.git) |
| `linux-raw-sys` | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/sunfishcode/linux-raw-sys) |
| `litemap` | 0.8.2 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 | [upstream](https://github.com/Amanieu/parking_lot) |
| `log` | 0.4.29 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/log) |
| `lru-slab` | 0.1.2 | MIT OR Apache-2.0 OR Zlib | [upstream](https://github.com/Ralith/lru-slab) |
| `lz4_flex` | 0.14.0 | MIT | [upstream](https://github.com/pseitz/lz4_flex) |
| `matchers` | 0.2.0 | MIT | [upstream](https://github.com/hawkw/matchers) |
| `matchit` | 0.7.3 | MIT AND BSD-3-Clause | [upstream](https://github.com/ibraheemdev/matchit) |
| `md-5` | 0.11.0 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/hashes) |
| `memchr` | 2.8.3 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/memchr) |
| `mime` | 0.3.17 | MIT OR Apache-2.0 | [upstream](https://github.com/hyperium/mime) |
| `minimal-lexical` | 0.2.1 | MIT/Apache-2.0 | [upstream](https://github.com/Alexhuszagh/minimal-lexical) |
| `miniz_oxide` | 0.8.9 | MIT OR Zlib OR Apache-2.0 | [upstream](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) |
| `mio` | 1.2.0 | MIT | [upstream](https://github.com/tokio-rs/mio) |
| `moka` | 0.12.15 | (MIT OR Apache-2.0) AND Apache-2.0 | [upstream](https://github.com/moka-rs/moka) |
| `native-tls` | 0.2.18 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-native-tls/rust-native-tls) |
| `nkeys` | 0.4.5 | Apache-2.0 | [upstream](https://github.com/wasmcloud/nkeys) |
| `no-std-compat` | 0.4.1 | MIT | [upstream](https://gitlab.com/jD91mZM2/no-std-compat) |
| `nohash-hasher` | 0.2.0 | Apache-2.0 OR MIT | [upstream](https://github.com/paritytech/nohash-hasher) |
| `nom` | 7.1.3 | MIT | [upstream](https://github.com/Geal/nom) |
| `nonzero_ext` | 0.3.0 | Apache-2.0 | [upstream](https://github.com/antifuchs/nonzero_ext) |
| `nu-ansi-term` | 0.50.3 | MIT | [upstream](https://github.com/nushell/nu-ansi-term) |
| `nuid` | 0.5.0 | Apache-2.0 | [upstream](https://github.com/casualjim/rs-nuid.git) |
| `num-bigint` | 0.4.6 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-num/num-bigint) |
| `num-bigint` | 0.5.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-num/num-bigint) |
| `num-complex` | 0.4.6 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-num/num-complex) |
| `num-conv` | 0.2.2 | MIT OR Apache-2.0 | [upstream](https://github.com/jhpratt/num-conv) |
| `num-integer` | 0.1.46 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-num/num-integer) |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-num/num-traits) |
| `objc2-core-foundation` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [upstream](https://github.com/madsmtm/objc2) |
| `objc2-system-configuration` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | [upstream](https://github.com/madsmtm/objc2) |
| `object` | 0.37.3 | Apache-2.0 OR MIT | [upstream](https://github.com/gimli-rs/object) |
| `object_store` | 0.13.2 | MIT/Apache-2.0 | [upstream](https://github.com/apache/arrow-rs-object-store) |
| `oid-registry` | 0.7.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rusticata/oid-registry.git) |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | [upstream](https://github.com/matklad/once_cell) |
| `openssl` | 0.10.80 | Apache-2.0 | [upstream](https://github.com/rust-openssl/rust-openssl) |
| `openssl-macros` | 0.1.1 | MIT/Apache-2.0 | Not declared |
| `openssl-probe` | 0.2.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rustls/openssl-probe) |
| `openssl-sys` | 0.9.116 | MIT | [upstream](https://github.com/rust-openssl/rust-openssl) |
| `ordered-float` | 2.10.1 | MIT | [upstream](https://github.com/reem/rust-ordered-float) |
| `parking` | 2.2.1 | Apache-2.0 OR MIT | [upstream](https://github.com/smol-rs/parking) |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 | [upstream](https://github.com/Amanieu/parking_lot) |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 | [upstream](https://github.com/Amanieu/parking_lot) |
| `parquet` | 59.3.0 | Apache-2.0 | [upstream](https://github.com/apache/arrow-rs) |
| `pem` | 3.0.6 | MIT | [upstream](https://github.com/jcreekmore/pem-rs.git) |
| `pem-rfc7468` | 0.7.0 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats/tree/master/pem-rfc7468) |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-url/) |
| `pest` | 2.8.6 | MIT OR Apache-2.0 | [upstream](https://github.com/pest-parser/pest) |
| `pest_derive` | 2.8.6 | MIT OR Apache-2.0 | [upstream](https://github.com/pest-parser/pest) |
| `pest_generator` | 2.8.6 | MIT OR Apache-2.0 | [upstream](https://github.com/pest-parser/pest) |
| `pest_meta` | 2.8.6 | MIT OR Apache-2.0 | [upstream](https://github.com/pest-parser/pest) |
| `petgraph` | 0.8.3 | MIT OR Apache-2.0 | [upstream](https://github.com/petgraph/petgraph) |
| `phf` | 0.12.1 | MIT | [upstream](https://github.com/rust-phf/rust-phf) |
| `phf` | 0.13.1 | MIT | [upstream](https://github.com/rust-phf/rust-phf) |
| `phf_shared` | 0.12.1 | MIT | [upstream](https://github.com/rust-phf/rust-phf) |
| `phf_shared` | 0.13.1 | MIT | [upstream](https://github.com/rust-phf/rust-phf) |
| `pin-project` | 1.1.13 | Apache-2.0 OR MIT | [upstream](https://github.com/taiki-e/pin-project) |
| `pin-project-internal` | 1.1.13 | Apache-2.0 OR MIT | [upstream](https://github.com/taiki-e/pin-project) |
| `pin-project-lite` | 0.2.17 | Apache-2.0 OR MIT | [upstream](https://github.com/taiki-e/pin-project-lite) |
| `pkcs8` | 0.10.2 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats/tree/master/pkcs8) |
| `pkg-config` | 0.3.33 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/pkg-config-rs) |
| `portable-atomic` | 1.13.1 | Apache-2.0 OR MIT | [upstream](https://github.com/taiki-e/portable-atomic) |
| `portable-atomic-util` | 0.2.7 | Apache-2.0 OR MIT | [upstream](https://github.com/taiki-e/portable-atomic-util) |
| `postgres` | 0.19.13 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-postgres/rust-postgres) |
| `postgres-native-tls` | 0.5.3 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-postgres/rust-postgres) |
| `postgres-protocol` | 0.6.12 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-postgres/rust-postgres) |
| `postgres-types` | 0.2.14 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-postgres/rust-postgres) |
| `potential_utf` | 0.1.5 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `powerfmt` | 0.2.0 | MIT OR Apache-2.0 | [upstream](https://github.com/jhpratt/powerfmt) |
| `ppv-lite86` | 0.2.21 | MIT OR Apache-2.0 | [upstream](https://github.com/cryptocorrosion/cryptocorrosion) |
| `prettyplease` | 0.2.37 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/prettyplease) |
| `proc-macro2` | 1.0.106 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/proc-macro2) |
| `psm` | 0.1.31 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/stacker/) |
| `quanta` | 0.12.6 | MIT | [upstream](https://github.com/metrics-rs/quanta) |
| `quinn` | 0.11.9 | MIT OR Apache-2.0 | [upstream](https://github.com/quinn-rs/quinn) |
| `quinn-proto` | 0.11.17 | MIT OR Apache-2.0 | [upstream](https://github.com/quinn-rs/quinn) |
| `quinn-udp` | 0.5.14 | MIT OR Apache-2.0 | [upstream](https://github.com/quinn-rs/quinn) |
| `quote` | 1.0.45 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/quote) |
| `r-efi` | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | [upstream](https://github.com/r-efi/r-efi) |
| `r-efi` | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | [upstream](https://github.com/r-efi/r-efi) |
| `rand` | 0.10.2 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand` | 0.8.6 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand` | 0.9.4 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand_chacha` | 0.3.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand_chacha` | 0.9.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand_core` | 0.10.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand_core) |
| `rand_core` | 0.6.4 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand_core` | 0.9.5 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rand) |
| `rand_pcg` | 0.10.2 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-random/rngs) |
| `raw-cpuid` | 11.6.0 | MIT | [upstream](https://github.com/gz/rust-cpuid) |
| `recursive` | 0.1.1 | MIT | [upstream](https://github.com/orlp/recursive) |
| `recursive-proc-macro-impl` | 0.1.1 | MIT | [upstream](https://github.com/orlp/recursive) |
| `redox_syscall` | 0.5.18 | MIT | [upstream](https://gitlab.redox-os.org/redox-os/syscall) |
| `ref-cast` | 1.0.27 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/ref-cast) |
| `ref-cast-impl` | 1.0.27 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/ref-cast) |
| `regex` | 1.12.3 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/regex) |
| `regex-automata` | 0.4.14 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/regex) |
| `regex-syntax` | 0.8.10 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/regex) |
| `reqwest` | 0.12.28 | MIT OR Apache-2.0 | [upstream](https://github.com/seanmonstar/reqwest) |
| `ring` | 0.17.14 | Apache-2.0 AND ISC | [upstream](https://github.com/briansmith/ring) |
| `rustc-hash` | 2.1.2 | Apache-2.0 OR MIT | [upstream](https://github.com/rust-lang/rustc-hash) |
| `rustc_version` | 0.4.1 | MIT OR Apache-2.0 | [upstream](https://github.com/djc/rustc-version-rs) |
| `rusticata-macros` | 4.1.0 | MIT/Apache-2.0 | [upstream](https://github.com/rusticata/rusticata-macros.git) |
| `rustix` | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/rustix) |
| `rustls` | 0.23.40 | Apache-2.0 OR ISC OR MIT | [upstream](https://github.com/rustls/rustls) |
| `rustls-native-certs` | 0.8.3 | Apache-2.0 OR ISC OR MIT | [upstream](https://github.com/rustls/rustls-native-certs) |
| `rustls-pki-types` | 1.14.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rustls/pki-types) |
| `rustls-platform-verifier` | 0.7.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rustls/rustls-platform-verifier) |
| `rustls-platform-verifier-android` | 0.1.1 | MIT OR Apache-2.0 | [upstream](https://github.com/rustls/rustls-platform-verifier) |
| `rustls-webpki` | 0.103.13 | ISC | [upstream](https://github.com/rustls/webpki) |
| `rustversion` | 1.0.22 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/rustversion) |
| `ryu` | 1.0.23 | Apache-2.0 OR BSL-1.0 | [upstream](https://github.com/dtolnay/ryu) |
| `same-file` | 1.0.6 | Unlicense/MIT | [upstream](https://github.com/BurntSushi/same-file) |
| `schannel` | 0.1.29 | MIT | [upstream](https://github.com/steffengy/schannel-rs) |
| `schemars` | 1.2.2 | MIT | [upstream](https://github.com/GREsau/schemars) |
| `schemars_derive` | 1.2.2 | MIT | [upstream](https://github.com/GREsau/schemars) |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 | [upstream](https://github.com/bluss/scopeguard) |
| `secrecy` | 0.10.3 | Apache-2.0 OR MIT | [upstream](https://github.com/iqlusioninc/crates/tree/main/secrecy) |
| `security-framework` | 3.7.0 | MIT OR Apache-2.0 | [upstream](https://github.com/kornelski/rust-security-framework) |
| `security-framework-sys` | 2.17.0 | MIT OR Apache-2.0 | [upstream](https://github.com/kornelski/rust-security-framework) |
| `semver` | 1.0.28 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/semver) |
| `seq-macro` | 0.3.6 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/seq-macro) |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | [upstream](https://github.com/serde-rs/serde) |
| `serde-saphyr` | 0.0.29 | MIT OR Apache-2.0 | [upstream](https://github.com/bourumir-wyngs/serde-saphyr) |
| `serde-value` | 0.7.0 | MIT | [upstream](https://github.com/arcnmx/serde-value) |
| `serde_core` | 1.0.228 | MIT OR Apache-2.0 | [upstream](https://github.com/serde-rs/serde) |
| `serde_derive` | 1.0.228 | MIT OR Apache-2.0 | [upstream](https://github.com/serde-rs/serde) |
| `serde_derive_internals` | 0.30.0 | MIT OR Apache-2.0 | [upstream](https://github.com/serde-rs/serde) |
| `serde_json` | 1.0.149 | MIT OR Apache-2.0 | [upstream](https://github.com/serde-rs/json) |
| `serde_nanos` | 0.1.4 | MIT OR Apache-2.0 | [upstream](https://github.com/caspervonb/serde_nanos) |
| `serde_path_to_error` | 0.1.20 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/path-to-error) |
| `serde_repr` | 0.1.20 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/serde-repr) |
| `serde_urlencoded` | 0.7.1 | MIT/Apache-2.0 | [upstream](https://github.com/nox/serde_urlencoded) |
| `serde_yaml` | 0.9.34+deprecated | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/serde-yaml) |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/hashes) |
| `sha2` | 0.11.0 | MIT OR Apache-2.0 | [upstream](https://github.com/RustCrypto/hashes) |
| `sharded-slab` | 0.1.7 | MIT | [upstream](https://github.com/hawkw/sharded-slab) |
| `shlex` | 1.3.0 | MIT OR Apache-2.0 | [upstream](https://github.com/comex/rust-shlex) |
| `signal-hook-registry` | 1.4.8 | MIT OR Apache-2.0 | [upstream](https://github.com/vorner/signal-hook) |
| `signatory` | 0.27.1 | Apache-2.0 OR MIT | [upstream](https://github.com/iqlusioninc/crates/tree/main/signatory) |
| `signature` | 2.2.0 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/traits/tree/master/signature) |
| `simd-adler32` | 0.3.9 | MIT | [upstream](https://github.com/mcountryman/simd-adler32) |
| `simd_cesu8` | 1.2.0 | Apache-2.0 OR MIT | [upstream](https://github.com/seancroach/simd_cesu8) |
| `simdutf8` | 0.1.5 | MIT OR Apache-2.0 | [upstream](https://github.com/rusticstuff/simdutf8) |
| `siphasher` | 1.0.3 | MIT/Apache-2.0 | [upstream](https://github.com/jedisct1/rust-siphash) |
| `slab` | 0.4.12 | MIT | [upstream](https://github.com/tokio-rs/slab) |
| `smallvec` | 1.15.1 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-smallvec) |
| `snap` | 1.1.1 | BSD-3-Clause | [upstream](https://github.com/BurntSushi/rust-snappy) |
| `socket2` | 0.6.3 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/socket2) |
| `spinning_top` | 0.3.0 | MIT/Apache-2.0 | [upstream](https://github.com/rust-osdev/spinning_top) |
| `spki` | 0.7.3 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/formats/tree/master/spki) |
| `sqlparser` | 0.62.0 | Apache-2.0 | [upstream](https://github.com/apache/datafusion-sqlparser-rs) |
| `sqlparser_derive` | 0.5.0 | Apache-2.0 | [upstream](https://github.com/sqlparser-rs/sqlparser-rs) |
| `stable_deref_trait` | 1.2.1 | MIT OR Apache-2.0 | [upstream](https://github.com/storyyeller/stable_deref_trait) |
| `stacker` | 0.1.24 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-lang/stacker) |
| `stringprep` | 0.1.5 | MIT/Apache-2.0 | [upstream](https://github.com/sfackler/rust-stringprep) |
| `strsim` | 0.11.1 | MIT | [upstream](https://github.com/rapidfuzz/strsim-rs) |
| `subtle` | 2.6.1 | BSD-3-Clause | [upstream](https://github.com/dalek-cryptography/subtle) |
| `syn` | 2.0.117 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/syn) |
| `syn` | 3.0.4 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/syn) |
| `sync_wrapper` | 1.0.2 | Apache-2.0 | [upstream](https://github.com/Actyx/sync_wrapper) |
| `synstructure` | 0.13.2 | MIT | [upstream](https://github.com/mystor/synstructure) |
| `tagptr` | 0.2.0 | MIT/Apache-2.0 | [upstream](https://github.com/oliver-giersch/tagptr.git) |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | [upstream](https://github.com/Stebalien/tempfile) |
| `thiserror` | 1.0.69 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/thiserror) |
| `thiserror` | 2.0.18 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/thiserror) |
| `thiserror-impl` | 1.0.69 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/thiserror) |
| `thiserror-impl` | 2.0.18 | MIT OR Apache-2.0 | [upstream](https://github.com/dtolnay/thiserror) |
| `thread_local` | 1.1.9 | MIT OR Apache-2.0 | [upstream](https://github.com/Amanieu/thread_local-rs) |
| `time` | 0.3.47 | MIT OR Apache-2.0 | [upstream](https://github.com/time-rs/time) |
| `time-core` | 0.1.8 | MIT OR Apache-2.0 | [upstream](https://github.com/time-rs/time) |
| `time-macros` | 0.2.27 | MIT OR Apache-2.0 | [upstream](https://github.com/time-rs/time) |
| `tiny-keccak` | 2.0.2 | CC0-1.0 | Not declared |
| `tinystr` | 0.8.3 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `tinyvec` | 1.11.0 | Zlib OR Apache-2.0 OR MIT | [upstream](https://github.com/Lokathor/tinyvec) |
| `tinyvec_macros` | 0.1.1 | MIT OR Apache-2.0 OR Zlib | [upstream](https://github.com/Soveu/tinyvec_macros) |
| `tokio` | 1.52.3 | MIT | [upstream](https://github.com/tokio-rs/tokio) |
| `tokio-macros` | 2.7.0 | MIT | [upstream](https://github.com/tokio-rs/tokio) |
| `tokio-native-tls` | 0.3.1 | MIT | [upstream](https://github.com/tokio-rs/tls) |
| `tokio-postgres` | 0.7.18 | MIT OR Apache-2.0 | [upstream](https://github.com/rust-postgres/rust-postgres) |
| `tokio-rustls` | 0.26.4 | MIT OR Apache-2.0 | [upstream](https://github.com/rustls/tokio-rustls) |
| `tokio-stream` | 0.1.18 | MIT | [upstream](https://github.com/tokio-rs/tokio) |
| `tokio-util` | 0.7.18 | MIT | [upstream](https://github.com/tokio-rs/tokio) |
| `tokio-websockets` | 0.10.1 | MIT | [upstream](https://github.com/Gelbpunkt/tokio-websockets/) |
| `tower` | 0.5.3 | MIT | [upstream](https://github.com/tower-rs/tower) |
| `tower-http` | 0.6.11 | MIT | [upstream](https://github.com/tower-rs/tower-http) |
| `tower-layer` | 0.3.3 | MIT | [upstream](https://github.com/tower-rs/tower) |
| `tower-service` | 0.3.3 | MIT | [upstream](https://github.com/tower-rs/tower) |
| `tracing` | 0.1.44 | MIT | [upstream](https://github.com/tokio-rs/tracing) |
| `tracing-attributes` | 0.1.31 | MIT | [upstream](https://github.com/tokio-rs/tracing) |
| `tracing-core` | 0.1.36 | MIT | [upstream](https://github.com/tokio-rs/tracing) |
| `tracing-log` | 0.2.0 | MIT | [upstream](https://github.com/tokio-rs/tracing) |
| `tracing-subscriber` | 0.3.23 | MIT | [upstream](https://github.com/tokio-rs/tracing) |
| `try-lock` | 0.2.5 | MIT | [upstream](https://github.com/seanmonstar/try-lock) |
| `tryhard` | 0.5.2 | MIT OR Apache-2.0 | [upstream](https://github.com/EmbarkStudios/tryhard) |
| `twox-hash` | 2.1.2 | MIT | [upstream](https://github.com/shepmaster/twox-hash) |
| `typenum` | 1.20.0 | MIT OR Apache-2.0 | [upstream](https://github.com/paholg/typenum) |
| `ucd-trie` | 0.1.7 | MIT OR Apache-2.0 | [upstream](https://github.com/BurntSushi/ucd-generate) |
| `unicode-bidi` | 0.3.18 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/unicode-bidi) |
| `unicode-ident` | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | [upstream](https://github.com/dtolnay/unicode-ident) |
| `unicode-normalization` | 0.1.25 | MIT OR Apache-2.0 | [upstream](https://github.com/unicode-rs/unicode-normalization) |
| `unicode-properties` | 0.1.4 | MIT/Apache-2.0 | [upstream](https://github.com/unicode-rs/unicode-properties) |
| `unicode-segmentation` | 1.13.2 | MIT OR Apache-2.0 | [upstream](https://github.com/unicode-rs/unicode-segmentation) |
| `unicode-width` | 0.2.2 | MIT OR Apache-2.0 | [upstream](https://github.com/unicode-rs/unicode-width) |
| `unicode-xid` | 0.2.6 | MIT OR Apache-2.0 | [upstream](https://github.com/unicode-rs/unicode-xid) |
| `unsafe-libyaml` | 0.2.11 | MIT | [upstream](https://github.com/dtolnay/unsafe-libyaml) |
| `untrusted` | 0.9.0 | ISC | [upstream](https://github.com/briansmith/untrusted) |
| `url` | 2.5.8 | MIT OR Apache-2.0 | [upstream](https://github.com/servo/rust-url) |
| `utf8_iter` | 1.0.4 | Apache-2.0 OR MIT | [upstream](https://github.com/hsivonen/utf8_iter) |
| `uuid` | 1.23.1 | Apache-2.0 OR MIT | [upstream](https://github.com/uuid-rs/uuid) |
| `valuable` | 0.1.1 | MIT | [upstream](https://github.com/tokio-rs/valuable) |
| `vcpkg` | 0.2.15 | MIT/Apache-2.0 | [upstream](https://github.com/mcgoo/vcpkg-rs) |
| `version_check` | 0.9.5 | MIT/Apache-2.0 | [upstream](https://github.com/SergioBenitez/version_check) |
| `walkdir` | 2.5.0 | Unlicense/MIT | [upstream](https://github.com/BurntSushi/walkdir) |
| `want` | 0.3.1 | MIT | [upstream](https://github.com/seanmonstar/want) |
| `wasi` | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasi) |
| `wasi` | 0.14.7+wasi-0.2.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasi-rs) |
| `wasip2` | 1.0.3+wasi-0.2.9 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasi-rs) |
| `wasip3` | 0.4.0+wasi-0.3.0-rc-2026-01-06 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasi-rs) |
| `wasite` | 1.0.2 | Apache-2.0 OR BSL-1.0 OR MIT | [upstream](https://github.com/ardaku/wasite) |
| `wasm-bindgen` | 0.2.121 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen) |
| `wasm-bindgen-futures` | 0.4.71 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures) |
| `wasm-bindgen-macro` | 0.2.121 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro) |
| `wasm-bindgen-macro-support` | 0.2.121 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro-support) |
| `wasm-bindgen-shared` | 0.2.121 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared) |
| `wasm-encoder` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder) |
| `wasm-metadata` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-metadata) |
| `wasmparser` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser) |
| `web-sys` | 0.3.98 | MIT OR Apache-2.0 | [upstream](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys) |
| `web-time` | 1.1.0 | MIT OR Apache-2.0 | [upstream](https://github.com/daxpedda/web-time) |
| `webpki-root-certs` | 1.0.9 | CDLA-Permissive-2.0 | [upstream](https://github.com/rustls/webpki-roots) |
| `webpki-roots` | 0.26.11 | CDLA-Permissive-2.0 | [upstream](https://github.com/rustls/webpki-roots) |
| `webpki-roots` | 1.0.7 | CDLA-Permissive-2.0 | [upstream](https://github.com/rustls/webpki-roots) |
| `whoami` | 2.1.2 | Apache-2.0 OR BSL-1.0 OR MIT | [upstream](https://github.com/ardaku/whoami) |
| `winapi` | 0.3.9 | MIT/Apache-2.0 | [upstream](https://github.com/retep998/winapi-rs) |
| `winapi-i686-pc-windows-gnu` | 0.4.0 | MIT/Apache-2.0 | [upstream](https://github.com/retep998/winapi-rs) |
| `winapi-util` | 0.1.11 | Unlicense OR MIT | [upstream](https://github.com/BurntSushi/winapi-util) |
| `winapi-x86_64-pc-windows-gnu` | 0.4.0 | MIT/Apache-2.0 | [upstream](https://github.com/retep998/winapi-rs) |
| `windows-core` | 0.62.2 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-implement` | 0.60.2 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-interface` | 0.59.3 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-result` | 0.4.1 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-strings` | 0.5.1 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-sys` | 0.52.0 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows-targets` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_aarch64_gnullvm` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_aarch64_msvc` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_i686_gnu` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_i686_gnullvm` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_i686_msvc` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_x86_64_gnu` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_x86_64_gnullvm` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `windows_x86_64_msvc` | 0.52.6 | MIT OR Apache-2.0 | [upstream](https://github.com/microsoft/windows-rs) |
| `wit-bindgen` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wit-bindgen) |
| `wit-bindgen` | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wit-bindgen) |
| `wit-bindgen-core` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wit-bindgen) |
| `wit-bindgen-rust` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wit-bindgen) |
| `wit-bindgen-rust-macro` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wit-bindgen) |
| `wit-component` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-component) |
| `wit-parser` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | [upstream](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-parser) |
| `writeable` | 0.6.3 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `x509-parser` | 0.16.0 | MIT OR Apache-2.0 | [upstream](https://github.com/rusticata/x509-parser.git) |
| `yoke` | 0.8.2 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `yoke-derive` | 0.8.2 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zerocopy` | 0.8.48 | BSD-2-Clause OR Apache-2.0 OR MIT | [upstream](https://github.com/google/zerocopy) |
| `zerocopy-derive` | 0.8.48 | BSD-2-Clause OR Apache-2.0 OR MIT | [upstream](https://github.com/google/zerocopy) |
| `zerofrom` | 0.1.8 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zerofrom-derive` | 0.1.7 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zeroize` | 1.8.2 | Apache-2.0 OR MIT | [upstream](https://github.com/RustCrypto/utils) |
| `zerotrie` | 0.2.4 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zerovec` | 0.11.6 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zerovec-derive` | 0.11.3 | Unicode-3.0 | [upstream](https://github.com/unicode-org/icu4x) |
| `zlib-rs` | 0.6.3 | Zlib | [upstream](https://github.com/trifectatechfoundation/zlib-rs) |
| `zmij` | 1.0.21 | MIT | [upstream](https://github.com/dtolnay/zmij) |
| `zstd` | 0.13.3 | MIT | [upstream](https://github.com/gyscos/zstd-rs) |
| `zstd-safe` | 7.2.4 | MIT OR Apache-2.0 | [upstream](https://github.com/gyscos/zstd-rs) |
| `zstd-sys` | 2.0.16+zstd.1.5.7 | MIT/Apache-2.0 | [upstream](https://github.com/gyscos/zstd-rs) |
