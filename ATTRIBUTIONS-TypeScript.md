# ATTRIBUTIONS - TypeScript

This file documents the TypeScript / npm dependencies of the
ai-blaise/citus front-end tooling. It is the AGPL-contamination guard
for the JavaScript portion of the surface area and is paired with
`ci/ai-blaise/license-check.sh` and `docs/ai-blaise/LICENSE_AUDIT.md`.

## Status: scaffold for upcoming TypeScript work

The TypeScript surface lives under two directories:

- `tools/citus-schema-designer/` -- planned fork of
  [DrawDB](https://github.com/drawdb-io/drawdb) (React + Vite +
  Tailwind schema designer).
- `tools/citus-admin/` -- planned fork of the React frontend portion of
  [WhoDB](https://github.com/clidey/whodb).

As of this commit both directories are Rust contract scaffolds and
**do not contain a `package.json`** yet. There are therefore no live
npm dependencies to enumerate. This file will be regenerated from
`package.json` and `pnpm-lock.yaml` / `package-lock.json` once the
front-end forks land. Until then it documents the expected dependency
set so that the AGPL-contamination posture can be reviewed up front.

## Regenerating (after `package.json` lands)

```sh
for pkg in tools/citus-schema-designer tools/citus-admin; do
  [ -f "${pkg}/package.json" ] && jq -r '
    (.dependencies // {} | to_entries[] | "\(.key)|\(.value)|runtime"),
    (.devDependencies // {} | to_entries[] | "\(.key)|\(.value)|dev")
  ' "${pkg}/package.json"
done | sort -u
```

License strings are taken from each package's `package.json` `license`
field, or from a top-level `LICENSE` / `LICENSE.md` file when the
field is absent.

## License boundary

The `tools/citus-schema-designer` and `tools/citus-admin` crates (and
their eventual npm packages) are distributed under AGPL-3.0
(`Cargo.toml` `[workspace.package]`). AGPL-3.0 is compatible with
permissive transitive npm packages (MIT, Apache-2.0, BSD-2/3-Clause,
ISC, MPL-2.0). Transitive **GPL-2.0** and **GPL-3.0** npm packages
are forbidden.

DrawDB upstream is licensed AGPL-3.0; WhoDB upstream's frontend is
GPL-3.0. Both relicense paths into this repository's AGPL-3.0
distribution are permitted:

- DrawDB -> AGPL-3.0 is a no-op (already AGPL-3.0).
- WhoDB frontend GPL-3.0 -> AGPL-3.0 is one-way compatible per the
  GPL FAQ "Combining work covered by the GNU AGPL".

The full text of each upstream license is preserved in the respective
`tools/citus-*` directory.

## Expected dependency set

The following packages are expected to land once the front-end forks
are in place. They are listed here so that license posture can be
audited before code is added.

### Build, bundling, language tooling

| Package | Expected license | Upstream |
|---|---|---|
| `typescript` | Apache-2.0 | https://github.com/microsoft/TypeScript |
| `vite` | MIT | https://github.com/vitejs/vite |
| `@vitejs/plugin-react` | MIT | https://github.com/vitejs/vite-plugin-react |
| `esbuild` | MIT | https://github.com/evanw/esbuild |
| `rollup` | MIT | https://github.com/rollup/rollup |
| `postcss` | MIT | https://github.com/postcss/postcss |
| `autoprefixer` | MIT | https://github.com/postcss/autoprefixer |
| `tailwindcss` | MIT | https://github.com/tailwindlabs/tailwindcss |

### Linting and formatting

| Package | Expected license | Upstream |
|---|---|---|
| `eslint` | MIT | https://github.com/eslint/eslint |
| `@typescript-eslint/parser` | MIT / BSD-2-Clause | https://github.com/typescript-eslint/typescript-eslint |
| `@typescript-eslint/eslint-plugin` | MIT | https://github.com/typescript-eslint/typescript-eslint |
| `prettier` | MIT | https://github.com/prettier/prettier |
| `eslint-plugin-react` | MIT | https://github.com/jsx-eslint/eslint-plugin-react |
| `eslint-plugin-react-hooks` | MIT | https://github.com/facebook/react |

### React UI runtime

| Package | Expected license | Upstream |
|---|---|---|
| `react` | MIT | https://github.com/facebook/react |
| `react-dom` | MIT | https://github.com/facebook/react |
| `react-router-dom` | MIT | https://github.com/remix-run/react-router |
| `@tanstack/react-query` | MIT | https://github.com/TanStack/query |
| `zustand` | MIT | https://github.com/pmndrs/zustand |

### DrawDB-specific (schema designer)

| Package | Expected license | Upstream |
|---|---|---|
| `reactflow` | MIT | https://github.com/xyflow/xyflow |
| `@xyflow/react` | MIT | https://github.com/xyflow/xyflow |
| `d3` | ISC | https://github.com/d3/d3 |
| `d3-selection` | ISC | https://github.com/d3/d3-selection |
| `d3-zoom` | ISC | https://github.com/d3/d3-zoom |
| `dompurify` | (MPL-2.0 OR Apache-2.0) | https://github.com/cure53/DOMPurify |
| `html-to-image` | MIT | https://github.com/bubkoo/html-to-image |
| `monaco-editor` | MIT | https://github.com/microsoft/monaco-editor |
| `@monaco-editor/react` | MIT | https://github.com/suren-atoyan/monaco-react |

### WhoDB-specific (admin UI)

| Package | Expected license | Upstream |
|---|---|---|
| `@apollo/client` | MIT | https://github.com/apollographql/apollo-client |
| `graphql` | MIT | https://github.com/graphql/graphql-js |
| `@radix-ui/react-dialog` | MIT | https://github.com/radix-ui/primitives |
| `@radix-ui/react-dropdown-menu` | MIT | https://github.com/radix-ui/primitives |
| `@radix-ui/react-tooltip` | MIT | https://github.com/radix-ui/primitives |
| `clsx` | MIT | https://github.com/lukeed/clsx |
| `lucide-react` | ISC | https://github.com/lucide-icons/lucide |
| `tailwind-merge` | MIT | https://github.com/dcastil/tailwind-merge |

### Testing

| Package | Expected license | Upstream |
|---|---|---|
| `vitest` | MIT | https://github.com/vitest-dev/vitest |
| `@vitest/coverage-v8` | MIT | https://github.com/vitest-dev/vitest |
| `@testing-library/react` | MIT | https://github.com/testing-library/react-testing-library |
| `@testing-library/jest-dom` | MIT | https://github.com/testing-library/jest-dom |
| `@testing-library/user-event` | MIT | https://github.com/testing-library/user-event |
| `jsdom` | MIT | https://github.com/jsdom/jsdom |
| `playwright` | Apache-2.0 | https://github.com/microsoft/playwright |

### Type declarations

| Package | Expected license | Upstream |
|---|---|---|
| `@types/node` | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped |
| `@types/react` | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped |
| `@types/react-dom` | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped |

### Forbidden

The front-end packages MUST NOT take a hard dependency on:

- any GPL-2.0-only or GPL-3.0-only npm package other than the
  WhoDB frontend upstream itself (which is relicensed to AGPL-3.0
  inside this repository when the fork lands)
- any "Custom" or "SEE LICENSE IN ..." package whose license cannot
  be programmatically resolved (these show up as `UNKNOWN` and fail
  the audit)
- any unmaintained transitive PRNG / crypto package: the front-end
  must not perform key generation client-side; all crypto operations
  are server-side via the `sidecar/auth` Rust surface

The Rust contract crates that own the schema-designer and admin
surfaces are tracked in [`ATTRIBUTIONS-Rust.md`](./ATTRIBUTIONS-Rust.md);
the Go portion of the admin backend is tracked in
[`ATTRIBUTIONS-Go.md`](./ATTRIBUTIONS-Go.md).
