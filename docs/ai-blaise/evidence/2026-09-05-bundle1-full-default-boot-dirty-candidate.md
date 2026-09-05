# Bundle1 full default-boot dirty-candidate receipt (2026-09-05)

## Evidence boundary

This receipt records one native `bundle1-final-full` build and stock-entrypoint
boot on the existing `instance-20260415-20260415-235136` VM in
`blaise-478114/asia-south1-b`. It is candidate evidence only. The input was a
content-hashed dirty overlay on a Git base, not a reviewed clean commit, and
several build dependencies remain mutable. It therefore does not qualify a
release, complete W1, or promote any M0/M1/M2 gate. The image label
`ai-blaise.citus.bundle1.release-target=true` identifies the Docker target; it
does not assert release qualification.

## Frozen input identity

- Git base: `e10607031da0ccd2cb3fd948b22902959dcd5f9a`
- Git bundle: `e106-base.bundle`, 19,523,076 bytes,
  SHA-256 `b2e257eef459b972c588d7012656776d98cc2ecbec20e3166b40ffd3e497a9a0`
- Binary tracked overlay: `tracked-overlay.patch`, 751,221 bytes,
  SHA-256 `6775922939b765b1b04a2d3b30abff684b6e251fe3ab54fcb456b4b8a4326362`
- Nonignored untracked overlay: `untracked-overlay.tar`, 656,384 bytes,
  SHA-256 `4d3f77827a48570475005e4dfaeb0ee82f39841e5bbb0ea5e1494f7aa03cf0aa`
- Canonical Docker context: 55,029,760 bytes and 4,428 entries, SHA-256
  `1cf4427f6fd36a795270798bd96304897dbe1c75e55ea4ac54004e2a2e6ae9f0`
- Canonical context entry listing: SHA-256
  `1f18bbad542d30670f624abed09e6818db020605d85fa26364c3e0ffb20bff48`
- Image source-tree label:
  `dirty-sha256-1cf4427f6fd36a795270798bd96304897dbe1c75e55ea4ac54004e2a2e6ae9f0`

The context archive normalized ownership and timestamps while retaining paths,
file modes, bytes, and link targets. It excluded the root Git metadata and the
build-artifact patterns in `.dockerignore`. Re-hashing the materialized source
with the same canonicalization after the smoke produced the same context hash.

Key W1 inputs in that context were:

| Input | SHA-256 |
| --- | --- |
| `images/citus-pg-overlay/Dockerfile` | `6125ca677cdce83e56d34e38848163634a297abc06108228de2cc73d0cdf4b0c` |
| `images/citus-pg-overlay/shared-preload-libraries.conf` | `4ace37465396061af27dc30527ca3d299253e4300dc917fd02e119f94486b47b` |
| `ci/ai-blaise/bundle1-default-boot-smoke.sh` | `2fdbf228c2578299a21c506fdecf9a4e4ebc89cdc6c434092cefa02702e592c6` |
| `ci/ai-blaise/bundle1-contract-check.py` | `ef2cbfce4ee91022d58c9cded760960764a177c4531cdaef1af9b766e0c645f0` |
| `ci/ai-blaise/image-check.sh` | `d80a6cf645996135c96604c50e2b30e08274f29ac331c21d5790d693a9909fab` |
| `ci/ai-blaise/sql-extension-smoke.sh` | `46a3804742bb5005b5606b0fe03474f84923f75a5dffb5eb9a03349b9d7e59bc` |
| `.github/workflows/ci-image.yml` | `bd5f9800d9e501997a3f9f82fd3896d2132296a0aca812a93078e03d7a936a22` |
| `images/citus-pg-overlay/extension-manifest.tsv` | `6425183b45ee98f6eafc0f8ef5b600917c2f8ac043d59595e58c187b4b1f949a` |
| `images/citus-pg-overlay/bundle1-source-build.lock.tsv` | `01238ce02a268bff18e99c3034b47ea316abe3f7cb383a9dd373c562482eba14` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus-upgrade-manifest.tsv` | `203bf7ac72148ef3c8d2ad5d01e612e90fc44f8872a2a8a178800f304ea1a87c` |
| `images/citus-pg-overlay/extensions/ai_blaise_citus--0.1.1--0.1.2.sql` | `fe6738bccce024a60296f31e8eddb82d2e31c5445cf23c393cac998448c89722` |
| `images/citus-pg-overlay/upgrades/ai_blaise_citus--0.1.2.sql` | `351031464536f119ec6dac1917d4a8cfde18d524aa6f25b2d2df400c4e31c8aa` |
| `images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql` | `089f39b665259aea808bc917b944dc7b583b6cf96ccb8d8de65d9328e9daec66` |

The Dockerfile also copies and compiles the complete in-tree `src`, `config`,
`vendor`, and `prep_buildtree` inputs and the six root build inputs. Those
paths matched Git base `e1060703` at freeze time; separate overlay inputs kept
the overall context dirty. A narrow overlay-only commit does not turn this
observation into clean-source release evidence; the full context identity
above remains its authority.

## Build identity and command

The build ran on x86-64 Rocky Linux 9 (`a3-ultragpu-8g`, Intel Emerald Rapids)
with Docker client/server `29.5.0`, API `1.54`, and Buildx `v0.34.0`. The exact
base was already present locally and `--pull=false` was used:

`postgres:17-bookworm@sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6`

The Dockerfile frontend resolved to
`docker/dockerfile:1@sha256:ecfaec9ed6d810b56388c508f4121597bfbba70d41a6dfeee4d8cad5f295fc32`.
The command consumed the sealed tar on standard input:

```sh
docker build --pull=false --progress=plain \
  -f images/citus-pg-overlay/Dockerfile \
  --target bundle1-final-full \
  --build-arg PG_MAJOR=17 \
  --build-arg BASE_IMAGE=postgres:17-bookworm@sha256:7bade6d532592ca8ce7ee32def7399dad2607c4ea5583839fc4352a095a11ea6 \
  --build-arg AI_BLAISE_SOURCE_GIT_SHA=e10607031da0ccd2cb3fd948b22902959dcd5f9a \
  --build-arg AI_BLAISE_SOURCE_TREE_STATE=dirty-sha256-1cf4427f6fd36a795270798bd96304897dbe1c75e55ea4ac54004e2a2e6ae9f0 \
  --iidfile /home/spencer/chimera-w1-e106-dirty-20260905/bundle1-full.iid \
  -t ai-blaise-citus-overlay:w1-e106-dirty-1cf4427f6fd3 \
  - < /home/spencer/chimera-w1-e106-dirty-20260905/docker-context.canonical.tar
```

The successful build produced:

- immutable image ID / manifest-list digest:
  `sha256:f2fe63ade65ddc9fcc2e67398166dcc9e159600a88dcfce2222d3b0e34c666a4`
- platform manifest: `sha256:5792bf165e96fb15c76787795110002e53fe912c135caac66e5a635c5bad4494`
- config: `sha256:464900eaba6f310800bae9857d38680aeea9d42a65d946089a5474f0db0124e3`
- attestation manifest:
  `sha256:8c70e5e3a3c388bdc272b4500e0a37769fec5f35ad7f30b772a7e70647e82fad`
- unpacked image size: 581,367,117 bytes
- build log SHA-256:
  `d5698a33a0ccf8648182579a0f79f7dd4331d9d7b7e58e3c307f2036618a4e96`

The native in-tree Citus `./configure PG_CONFIG=/usr/bin/pg_config` completed
without a PostgreSQL-version bypass, and `make install-all` installed the
downgrade SQL. The build also completed the pinned light and full source stages.

## Toolchain and package observation

- PostgreSQL: `17.11 (Debian 17.11-1.pgdg12+2)`
- builder default Rust: `rustc 1.98.1 (48a229cea 2026-09-01)`
- builder Cargo: `1.98.1 (797e8a9bc 2026-08-05)`
- `cargo-pgrx`: `0.16.1`
- pg_search project toolchain installed by its pinned source: Rust `1.90.0`
- C compiler: Debian GCC `12.2.0-14+deb12u1`
- GNU Make `4.3`; CMake `3.25.1`
- runtime glibc: `2.36-9+deb12u14`

The complete 256-row runtime package list has SHA-256
`3be9fcd857aa3396da1641ec462a4dad6352fb91cd0c6a51ea099f2f8d81c94e`.
The complete builder package list has SHA-256
`4d69eb80d05da1e9af2b5ffdbb9d68b6151c83c7b130fdf64241d107b1c37d86`.
Notable runtime packages were TimescaleDB `2.29.2~debian12-1711`, pgvector
`0.8.6-1.pgdg12+1`, pgaudit `17.1-2.pgdg12+1`, pg_cron
`1.6.7-3.pgdg12+1`, AGE `1.7.0~rc0-1.pgdg12+1`, and PostGIS
`3.6.4+dfsg-2.pgdg12+1`.

## Default-boot proof

The smoke used the IID file, passed no PostgreSQL command or `-c` override, and
bound the expected target and source labels:

```sh
BUNDLE1_IMAGE=sha256:f2fe63ade65ddc9fcc2e67398166dcc9e159600a88dcfce2222d3b0e34c666a4 \
BUNDLE1_PG_MAJOR=17 \
BUNDLE1_EXPECTED_TARGET=bundle1-final-full \
BUNDLE1_EXPECTED_SOURCE_GIT_SHA=e10607031da0ccd2cb3fd948b22902959dcd5f9a \
BUNDLE1_EXPECTED_SOURCE_TREE_STATE=dirty-sha256-1cf4427f6fd36a795270798bd96304897dbe1c75e55ea4ac54004e2a2e6ae9f0 \
  bash ci/ai-blaise/bundle1-default-boot-smoke.sh
```

The smoke passed with target `bundle1-final-full`, scope
`full-bundle-required-minus-plrust`, PostgreSQL 17, Citus `15.0-1`, companion
`0.1.2`, 26 required SQL extensions, and one required preload-only capability.
It called the installed zero-argument preload-order assertion and the
required-library assertion against the applied GUCs. The deliberate Citus-first
and missing-required-library inputs both took the expected error paths under
`ON_ERROR_STOP`. The smoke log SHA-256 is
`8b976a5f43a6b32bb0ee79887f19b52ff79019539fac1b04de8314328eadc23a`.

## Preserved artifacts and remaining gates

The task-owned records remain under
`/home/spencer/chimera-w1-e106-dirty-20260905/` on the existing VM. In addition
to the hashes above, image inspect, image history, runtime toolchain, runtime
package, builder toolchain, builder package, and environment records were
captured and hashed there. That VM path is an operational receipt location,
not a durable release evidence store.

This observation is not reproducible release evidence yet:

- the input is a dirty overlay over `e1060703`, and its native result depends on
  the complete context rather than only the W1 files;
- Debian, PGDG, and Timescale package versions are selected from mutable
  repositories instead of a content-pinned package snapshot;
- the rustup installer and `stable` toolchain selector are mutable, although
  the versions observed in this run are recorded;
- the Dockerfile frontend is named by a mutable tag in source, even though its
  resolved digest is recorded here;
- the exact-ref Git checks prevent silent tag movement, but remote source and
  registry availability remain external build dependencies;
- plrust remains explicitly deferred for PG17 and is outside this target;
- no second clean-input rebuild, registry publication/signature, SBOM and
  vulnerability disposition, multi-architecture build, PG18/PG19 full operand,
  clustered upgrade/rollback, performance, or M0/M1/M2 native proof was run.
