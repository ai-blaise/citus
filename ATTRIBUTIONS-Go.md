# ATTRIBUTIONS - Go

This file documents the Go module dependencies of the ai-blaise/citus
administrative tooling. It is the AGPL-contamination guard for the Go
portion of the surface area and is paired with
`ci/ai-blaise/license-check.sh` and `docs/ai-blaise/LICENSE_AUDIT.md`.

## Status: scaffold for upcoming Go work

The Go surface lives under `tools/citus-admin/`, which is a planned fork
of the [WhoDB](https://github.com/clidey/whodb) database administration
UI. As of this commit `tools/citus-admin/` is still a Rust contract
scaffold (`cargo run -p ai_blaise_citus_admin -- run-canonical`) and
**does not contain a `go.mod`** yet. There are therefore no live Go
dependencies to enumerate.

This file will be regenerated from `go list -m -json all` once the
WhoDB fork lands. Until then it documents the expected dependency set
so that the AGPL-contamination posture can be reviewed up front.

## Regenerating (after `go.mod` lands)

```sh
cd tools/citus-admin
go list -m -json all \
  | jq -r '"\(.Path)|\(.Version)|\(.Info.Module.License // "UNKNOWN")"' \
  | sort -u
```

Then re-bucket by license string and update the tables below.

## License boundary

The `tools/citus-admin` crate (and its eventual Go module) is
distributed under AGPL-3.0 (`Cargo.toml` `[workspace.package]`).
AGPL-3.0 is compatible with permissive transitive Go modules (MIT,
Apache-2.0, BSD-2/3-Clause, ISC, MPL-2.0). Transitive **GPL-2.0** and
**GPL-3.0** Go modules are forbidden and will be rejected by
`ci/ai-blaise/license-check.sh` once the Go scan is wired in.

WhoDB itself is GPL-3.0 upstream. The fork into `tools/citus-admin`
will be relicensed to AGPL-3.0 inside this repository, which is
permitted because the citus repo is itself an AGPL-3.0 distribution
and AGPL-3.0 is one-way compatible with GPL-3.0 (see GPL FAQ
"Combining work covered by the GNU AGPL"). The full text of the
upstream GPL-3.0 license is preserved under `tools/citus-admin/`
when the fork lands.

## Expected dependency set (from WhoDB upstream)

The following modules are expected to land once the WhoDB fork is
in place. They are listed here so that license posture can be
audited before code is added.

### Web framework and routing

| Module | Expected license | Upstream |
|---|---|---|
| `github.com/go-chi/chi/v5` | MIT | https://github.com/go-chi/chi |
| `github.com/labstack/echo/v4` | MIT | https://github.com/labstack/echo |
| `github.com/gofiber/fiber/v2` | MIT | https://github.com/gofiber/fiber |
| `github.com/gorilla/mux` | BSD-3-Clause | https://github.com/gorilla/mux |
| `github.com/gorilla/websocket` | BSD-2-Clause | https://github.com/gorilla/websocket |

WhoDB upstream currently uses Echo; chi and fiber are listed as
permitted fallbacks if the fork swaps to a leaner router. Exactly
one router lands in production.

### PostgreSQL connectivity

| Module | Expected license | Upstream |
|---|---|---|
| `github.com/jackc/pgx/v5` | MIT | https://github.com/jackc/pgx |
| `github.com/jackc/pgconn` | MIT | https://github.com/jackc/pgconn |
| `github.com/jackc/pgxpool` | MIT | https://github.com/jackc/pgx |

`pgx` is the only supported PG driver inside the admin UI. The
upstream `lib/pq` driver is **not** vendored: it has been in
maintenance mode upstream since 2021.

### GraphQL surface

| Module | Expected license | Upstream |
|---|---|---|
| `github.com/99designs/gqlgen` | MIT | https://github.com/99designs/gqlgen |
| `github.com/graph-gophers/graphql-go` | BSD-2-Clause | https://github.com/graph-gophers/graphql-go |
| `github.com/vektah/gqlparser/v2` | MIT | https://github.com/vektah/gqlparser |

WhoDB uses gqlgen upstream. The admin UI keeps the GraphQL surface
optional and **read-only by default** so the surface remains
compatible with the MCP read-only database boundary established by
`sidecar/mcp/`.

### Config, logging, identity

| Module | Expected license | Upstream |
|---|---|---|
| `github.com/spf13/viper` | MIT | https://github.com/spf13/viper |
| `github.com/spf13/cobra` | Apache-2.0 | https://github.com/spf13/cobra |
| `github.com/spf13/pflag` | BSD-3-Clause | https://github.com/spf13/pflag |
| `github.com/rs/zerolog` | MIT | https://github.com/rs/zerolog |
| `github.com/google/uuid` | BSD-3-Clause | https://github.com/google/uuid |
| `golang.org/x/crypto` | BSD-3-Clause | https://github.com/golang/crypto |
| `golang.org/x/oauth2` | BSD-3-Clause | https://github.com/golang/oauth2 |
| `golang.org/x/sync` | BSD-3-Clause | https://github.com/golang/sync |
| `golang.org/x/sys` | BSD-3-Clause | https://github.com/golang/sys |
| `golang.org/x/text` | BSD-3-Clause | https://github.com/golang/text |

### Testing

| Module | Expected license | Upstream |
|---|---|---|
| `github.com/stretchr/testify` | MIT | https://github.com/stretchr/testify |
| `github.com/google/go-cmp` | BSD-3-Clause | https://github.com/google/go-cmp |

### Forbidden

`tools/citus-admin/` MUST NOT take a hard dependency on:

- any GPL-2.0-only Go module (would block AGPL distribution)
- any GPL-3.0-only Go module other than the WhoDB upstream fork
  itself, which is relicensed to AGPL-3.0 inside this repository
- any module whose license cannot be programmatically resolved by
  `go list -m -json all` (these show up as `UNKNOWN` and fail the
  audit)

The TypeScript portion of the WhoDB UI (the React frontend) is
tracked separately in [`ATTRIBUTIONS-TypeScript.md`](./ATTRIBUTIONS-TypeScript.md).
