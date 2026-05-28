# ATTRIBUTIONS - Rust

This file lists every Rust crate dependency of the ai-blaise/citus
Cargo workspace, including all transitive dependencies, with the
license declared in each crate's `Cargo.toml` and the upstream
repository URL. It is generated from `cargo metadata` and kept in
sync by `ci/ai-blaise/license-check.sh`.

## Ported source attributions

The `ai_blaise_citus_pool_wire` crate is a Rust port (not a Cargo
dependency) of upstream PostgreSQL v3 wire-protocol parsing code from
`jackc/pgx` (`pgproto3`), MIT licensed. The upstream copyright
notice is preserved in [pool/wire/THIRD_PARTY_NOTICES.md](pool/wire/THIRD_PARTY_NOTICES.md).
The Rust code itself is original work; only the wire-message shapes,
tag bytes, and encode/decode call surface mirror the upstream Go
implementation. No Cargo dependency on `pgx` exists.

| Ported source | Upstream | License | Local files |
|---|---|---|---|
| `pgproto3` | https://github.com/jackc/pgx | MIT | `pool/wire/src/{lib,codec,envelope,frontend,backend,startup,auth}.rs` + `pool/wire/examples/pipeline_live_smoke.rs` |

## License boundary

The workspace crates published from this repository are licensed
AGPL-3.0 (see `Cargo.toml` `[workspace.package]`). AGPL-3.0 is
compatible with permissive transitive dependencies (MIT,
Apache-2.0, BSD-2/3-Clause, ISC, MPL-2.0, Unlicense, Zlib,
BSL-1.0) and with the weak-copyleft LGPL families. Transitive
**GPL-2.0** and **GPL-3.0** dependencies are forbidden and are
rejected by `ci/ai-blaise/license-check.sh`, because either would
impose viral copyleft constraints the AGPL fork itself cannot
relicense.

Crate totals at generation time:

| License group | Crate count |
|---|---:|
| AGPL-3.0 (workspace crates) | 29 |
| Apache-2.0 | 1 |
| MIT / Apache-2.0 | 93 |
| MIT | 13 |
| Apache-2.0 WITH LLVM-exception / Apache-2.0 / MIT | 16 |
| MIT / Apache-2.0 / Zlib | 4 |
| MIT / Apache-2.0 / Unicode-3.0 | 1 |
| MIT / Apache-2.0 / LGPL-2.1-or-later | 1 |
| MIT / Apache-2.0 / BSL-1.0 | 2 |
| Unlicense / MIT | 2 |
| Zlib | 1 |
| **Total** | **163** |

## Regenerating

```sh
cargo metadata --format-version 1 \
  | jq -r '.packages[] | [.name, .version, (.license // "UNKNOWN"), (.repository // "")] | @tsv' \
  | sort -u
```

Then re-bucket by license string and update the tables below.
`ci/ai-blaise/license-check.sh` fails the build if any `Cargo.lock`
entry resolves to a GPL-2.0 or GPL-3.0 crate.

## Dependencies by license

### AGPL-3.0 (workspace crates)

| Crate | Version | License | Repository |
|---|---|---|---|
| `ai_blaise_citus_admin` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_companion` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_e2e` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_lsp` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_mcp` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_operator` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_pool` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_schema_designer` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_analytical` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_auth` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_backup` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_cdc` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_coldtier` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_edge_functions` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_graphql` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_hlc` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_mcp` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_postgrest` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_raft` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_realtime` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_repack` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_schema_job` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_shared` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_storage` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_txn_status` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_sidecar_vectorizer` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_tui` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citus_watch` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |
| `ai_blaise_citusctl` | 0.1.0 | AGPL-3.0 | https://github.com/ai-blaise/citus |

### Apache-2.0

| Crate | Version | License | Repository |
|---|---|---|---|
| `openssl` | 0.10.80 | Apache-2.0 | https://github.com/rust-openssl/rust-openssl |

### MIT / Apache-2.0

| Crate | Version | License | Repository |
|---|---|---|---|
| `anyhow` | 1.0.102 | MIT OR Apache-2.0 | https://github.com/dtolnay/anyhow |
| `async-trait` | 0.1.89 | MIT OR Apache-2.0 | https://github.com/dtolnay/async-trait |
| `base64` | 0.22.1 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 |
| `bitflags` | 2.11.1 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags |
| `block-buffer` | 0.12.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| `bumpalo` | 3.20.2 | MIT OR Apache-2.0 | https://github.com/fitzgen/bumpalo |
| `cc` | 1.2.62 | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| `cfg-if` | 1.0.4 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| `chacha20` | 0.10.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/stream-ciphers |
| `cmov` | 0.5.3 | Apache-2.0 OR MIT | https://github.com/RustCrypto/utils |
| `const-oid` | 0.10.2 | Apache-2.0 OR MIT | https://github.com/RustCrypto/formats |
| `core-foundation` | 0.10.1 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `core-foundation-sys` | 0.8.7 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `cpufeatures` | 0.3.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils |
| `crypto-common` | 0.2.2 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| `ctutils` | 0.4.2 | Apache-2.0 OR MIT | https://github.com/RustCrypto/utils |
| `digest` | 0.11.3 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits |
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| `errno` | 0.3.14 | MIT OR Apache-2.0 | https://github.com/lambda-fairy/rust-errno |
| `fallible-iterator` | 0.2.0 | MIT/Apache-2.0 | https://github.com/sfackler/rust-fallible-iterator |
| `fastrand` | 2.4.1 | Apache-2.0 OR MIT | https://github.com/smol-rs/fastrand |
| `find-msvc-tools` | 0.1.9 | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| `foreign-types` | 0.3.2 | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `foreign-types-shared` | 0.1.1 | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `futures-channel` | 0.3.32 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-core` | 0.3.32 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-sink` | 0.3.32 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-task` | 0.3.32 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-util` | 0.3.32 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `getrandom` | 0.4.2 | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom |
| `hashbrown` | 0.15.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `heck` | 0.5.0 | MIT OR Apache-2.0 | https://github.com/withoutboats/heck |
| `hmac` | 0.13.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/MACs |
| `hybrid-array` | 0.4.12 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hybrid-array |
| `id-arena` | 2.3.0 | MIT/Apache-2.0 | https://github.com/fitzgen/id-arena |
| `indexmap` | 2.14.0 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| `js-sys` | 0.3.98 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys |
| `leb128fmt` | 0.1.0 | MIT OR Apache-2.0 | https://github.com/bluk/leb128fmt |
| `libc` | 0.2.186 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `log` | 0.4.29 | MIT OR Apache-2.0 | https://github.com/rust-lang/log |
| `md-5` | 0.11.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `native-tls` | 0.2.18 | MIT OR Apache-2.0 | https://github.com/rust-native-tls/rust-native-tls |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | https://github.com/matklad/once_cell |
| `openssl-macros` | 0.1.1 | MIT/Apache-2.0 | _(not declared)_ |
| `openssl-probe` | 0.2.1 | MIT OR Apache-2.0 | https://github.com/rustls/openssl-probe |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ |
| `pin-project-lite` | 0.2.17 | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project-lite |
| `pkg-config` | 0.3.33 | MIT OR Apache-2.0 | https://github.com/rust-lang/pkg-config-rs |
| `postgres` | 0.19.13 | MIT OR Apache-2.0 | https://github.com/rust-postgres/rust-postgres |
| `postgres-native-tls` | 0.5.3 | MIT OR Apache-2.0 | https://github.com/rust-postgres/rust-postgres |
| `postgres-protocol` | 0.6.11 | MIT OR Apache-2.0 | https://github.com/rust-postgres/rust-postgres |
| `postgres-types` | 0.2.13 | MIT OR Apache-2.0 | https://github.com/rust-postgres/rust-postgres |
| `prettyplease` | 0.2.37 | MIT OR Apache-2.0 | https://github.com/dtolnay/prettyplease |
| `proc-macro2` | 1.0.106 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| `quote` | 1.0.45 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| `rand` | 0.10.1 | MIT OR Apache-2.0 | https://github.com/rust-random/rand |
| `rand_core` | 0.10.1 | MIT OR Apache-2.0 | https://github.com/rust-random/rand_core |
| `rustversion` | 1.0.22 | MIT OR Apache-2.0 | https://github.com/dtolnay/rustversion |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 | https://github.com/bluss/scopeguard |
| `security-framework` | 3.7.0 | MIT OR Apache-2.0 | https://github.com/kornelski/rust-security-framework |
| `security-framework-sys` | 2.17.0 | MIT OR Apache-2.0 | https://github.com/kornelski/rust-security-framework |
| `semver` | 1.0.28 | MIT OR Apache-2.0 | https://github.com/dtolnay/semver |
| `serde` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_core` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_derive` | 1.0.228 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_json` | 1.0.149 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| `sha2` | 0.11.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `shlex` | 1.3.0 | MIT OR Apache-2.0 | https://github.com/comex/rust-shlex |
| `siphasher` | 1.0.3 | MIT/Apache-2.0 | https://github.com/jedisct1/rust-siphash |
| `smallvec` | 1.15.1 | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec |
| `socket2` | 0.6.3 | MIT OR Apache-2.0 | https://github.com/rust-lang/socket2 |
| `stringprep` | 0.1.5 | MIT/Apache-2.0 | https://github.com/sfackler/rust-stringprep |
| `syn` | 2.0.117 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | https://github.com/Stebalien/tempfile |
| `tokio-postgres` | 0.7.17 | MIT OR Apache-2.0 | https://github.com/rust-postgres/rust-postgres |
| `typenum` | 1.20.0 | MIT OR Apache-2.0 | https://github.com/paholg/typenum |
| `unicode-bidi` | 0.3.18 | MIT OR Apache-2.0 | https://github.com/servo/unicode-bidi |
| `unicode-normalization` | 0.1.25 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-normalization |
| `unicode-properties` | 0.1.4 | MIT/Apache-2.0 | https://github.com/unicode-rs/unicode-properties |
| `unicode-xid` | 0.2.6 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-xid |
| `vcpkg` | 0.2.15 | MIT/Apache-2.0 | https://github.com/mcgoo/vcpkg-rs |
| `wasm-bindgen` | 0.2.121 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen |
| `wasm-bindgen-macro` | 0.2.121 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro |
| `wasm-bindgen-macro-support` | 0.2.121 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro-support |
| `wasm-bindgen-shared` | 0.2.121 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared |
| `web-sys` | 0.3.98 | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |

### MIT

| Crate | Version | License | Repository |
|---|---|---|---|
| `bytes` | 1.11.1 | MIT | https://github.com/tokio-rs/bytes |
| `libredox` | 0.1.16 | MIT | https://gitlab.redox-os.org/redox-os/libredox.git |
| `mio` | 1.2.0 | MIT | https://github.com/tokio-rs/mio |
| `openssl-sys` | 0.9.116 | MIT | https://github.com/rust-openssl/rust-openssl |
| `phf` | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf |
| `phf_shared` | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf |
| `redox_syscall` | 0.5.18 | MIT | https://gitlab.redox-os.org/redox-os/syscall |
| `schannel` | 0.1.29 | MIT | https://github.com/steffengy/schannel-rs |
| `slab` | 0.4.12 | MIT | https://github.com/tokio-rs/slab |
| `tokio` | 1.52.3 | MIT | https://github.com/tokio-rs/tokio |
| `tokio-native-tls` | 0.3.1 | MIT | https://github.com/tokio-rs/tls |
| `tokio-util` | 0.7.18 | MIT | https://github.com/tokio-rs/tokio |
| `zmij` | 1.0.21 | MIT | https://github.com/dtolnay/zmij |

### Apache-2.0 WITH LLVM-exception / Apache-2.0 / MIT

| Crate | Version | License | Repository |
|---|---|---|---|
| `linux-raw-sys` | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/sunfishcode/linux-raw-sys |
| `rustix` | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/rustix |
| `wasi` | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi |
| `wasi` | 0.14.7+wasi-0.2.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi-rs |
| `wasip2` | 1.0.3+wasi-0.2.9 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi-rs |
| `wasip3` | 0.4.0+wasi-0.3.0-rc-2026-01-06 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi-rs |
| `wasm-encoder` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder |
| `wasm-metadata` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-metadata |
| `wasmparser` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser |
| `wit-bindgen` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wit-bindgen |
| `wit-bindgen` | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wit-bindgen |
| `wit-bindgen-core` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wit-bindgen |
| `wit-bindgen-rust` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wit-bindgen |
| `wit-bindgen-rust-macro` | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wit-bindgen |
| `wit-component` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-component |
| `wit-parser` | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wit-parser |

### MIT / Apache-2.0 / Zlib

| Crate | Version | License | Repository |
|---|---|---|---|
| `objc2-core-foundation` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 |
| `objc2-system-configuration` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 |
| `tinyvec` | 1.11.0 | Zlib OR Apache-2.0 OR MIT | https://github.com/Lokathor/tinyvec |
| `tinyvec_macros` | 0.1.1 | MIT OR Apache-2.0 OR Zlib | https://github.com/Soveu/tinyvec_macros |

### MIT / Apache-2.0 / Unicode-3.0

| Crate | Version | License | Repository |
|---|---|---|---|
| `unicode-ident` | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |

### MIT / Apache-2.0 / LGPL-2.1-or-later

| Crate | Version | License | Repository |
|---|---|---|---|
| `r-efi` | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | https://github.com/r-efi/r-efi |

### MIT / Apache-2.0 / BSL-1.0

| Crate | Version | License | Repository |
|---|---|---|---|
| `wasite` | 1.0.2 | Apache-2.0 OR BSL-1.0 OR MIT | https://github.com/ardaku/wasite |
| `whoami` | 2.1.2 | Apache-2.0 OR BSL-1.0 OR MIT | https://github.com/ardaku/whoami |

### Unlicense / MIT

| Crate | Version | License | Repository |
|---|---|---|---|
| `byteorder` | 1.5.0 | Unlicense OR MIT | https://github.com/BurntSushi/byteorder |
| `memchr` | 2.8.0 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |

### Zlib

| Crate | Version | License | Repository |
|---|---|---|---|
| `foldhash` | 0.1.5 | Zlib | https://github.com/orlp/foldhash |

