#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

bash ci/ai-blaise/mcp-stdio-smoke.sh
bash ci/ai-blaise/citus-lsp-smoke.sh
bash ci/ai-blaise/tools-ui-runtime-smoke.sh

echo $'tool_services_runtime_smoke	mcp=ok	lsp=ok	admin=ok	schema_designer=ok	tui=ok	watch=ok'
