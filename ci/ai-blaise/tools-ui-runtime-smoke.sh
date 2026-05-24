#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

snapshot="${tmp_dir}/snapshot.tsv"
cat >"${snapshot}" <<'TSV'
meta	cluster_name	prod-east
meta	generated_at	2026-05-23T22:00:00Z
worker	worker-1	10.0.0.11	primary	ready
worker	worker-2	10.0.0.12	replica	ready
table	public.events	tenant_id	32	tenant	created_at	1 day	1	2
shard	public.events	102008	worker-1	active	1048576
shard	public.events	102009	worker-2	active	2097152
tenant	tenant-a	active	worker-1	2
tenant	tenant-b	moving	worker-2	2
vectorizer	documents-body	tenant-a	128	250000	ok
search_index	public.events	events_search	ready	bm25
branch	branch-main	active	0/16B6C50
backup	backup-20260523	completed	2026-05-23T21:55:00Z
realtime	tenant-a	public.events	3	0/16B6C50
pool	ready	42	2	0
TSV

require_contains() {
  local file="$1"
  local expected="$2"

  if ! grep -Fq "${expected}" "${file}"; then
    echo "missing expected output: ${expected}" >&2
    echo "--- ${file} ---" >&2
    cat "${file}" >&2
    exit 1
  fi
}

require_fails_with() {
  local expected="$1"
  shift

  local stdout="${tmp_dir}/fail.stdout"
  local stderr="${tmp_dir}/fail.stderr"
  if "$@" >"${stdout}" 2>"${stderr}"; then
    echo "command unexpectedly succeeded: $*" >&2
    cat "${stdout}" >&2
    exit 1
  fi
  if ! grep -Fq "${expected}" "${stderr}"; then
    echo "failing command did not report expected error: ${expected}" >&2
    echo "--- stderr ---" >&2
    cat "${stderr}" >&2
    exit 1
  fi
}

admin_html="${tmp_dir}/admin.html"
admin_html_repeat="${tmp_dir}/admin-repeat.html"
cargo run -q -p ai_blaise_citus_admin -- \
  render --snapshot "${snapshot}" --route /cluster/shards >"${admin_html}"
cargo run -q -p ai_blaise_citus_admin -- \
  render --snapshot "${snapshot}" --route /cluster/shards >"${admin_html_repeat}"
cmp "${admin_html}" "${admin_html_repeat}"
require_contains "${admin_html}" 'data-tool="citus-admin"'
require_contains "${admin_html}" 'data-route="/cluster/shards"'
require_contains "${admin_html}" '102008'
require_contains "${admin_html}" 'worker-1'

require_fails_with \
  'unknown admin route: /not-a-route' \
  cargo run -q -p ai_blaise_citus_admin -- \
    render --snapshot "${snapshot}" --route /not-a-route

require_fails_with \
  'rebalance-shard requires CONFIRM' \
  cargo run -q -p ai_blaise_citus_admin -- \
    action --snapshot "${snapshot}" --kind rebalance-shard --shard-id 102008

admin_action="${tmp_dir}/admin-action.tsv"
cargo run -q -p ai_blaise_citus_admin -- \
  action --snapshot "${snapshot}" --kind rebalance-shard --shard-id 102008 \
  --confirm CONFIRM >"${admin_action}"
require_contains "${admin_action}" $'rebalance-shard	accepted	validated dry-run for shard 102008'

schema_svg="${tmp_dir}/schema.svg"
schema_svg_repeat="${tmp_dir}/schema-repeat.svg"
cargo run -q -p ai_blaise_citus_schema_designer -- \
  render-svg --snapshot "${snapshot}" >"${schema_svg}"
cargo run -q -p ai_blaise_citus_schema_designer -- \
  render-svg --snapshot "${snapshot}" >"${schema_svg_repeat}"
cmp "${schema_svg}" "${schema_svg_repeat}"
require_contains "${schema_svg}" '<svg'
require_contains "${schema_svg}" 'data-feature="D6 M9"'
require_contains "${schema_svg}" 'public.events'
require_contains "${schema_svg}" 'shard 102008 on worker-1'

bad_snapshot="${tmp_dir}/bad-snapshot.tsv"
cat >"${bad_snapshot}" <<'TSV'
meta	generated_at	2026-05-23T22:00:00Z
worker	worker-1	10.0.0.11	primary	ready
table	public.events	tenant_id	32	tenant	created_at	1 day	1	2
shard	public.events	102008	worker-x	active	1048576
TSV
require_fails_with \
  'references unknown value worker-x' \
  cargo run -q -p ai_blaise_citus_schema_designer -- \
    render-svg --snapshot "${bad_snapshot}"

tui_frame="${tmp_dir}/tui-frame.txt"
cargo run -q -p ai_blaise_citus_tui -- \
  render-frame --snapshot "${snapshot}" --panel shards >"${tui_frame}"
require_contains "${tui_frame}" 'citus-tui | cluster=prod-east | panel=shards'
require_contains "${tui_frame}" '102008'
require_contains "${tui_frame}" 'worker-1'

require_fails_with \
  'unknown panel not-a-panel' \
  cargo run -q -p ai_blaise_citus_tui -- \
    render-frame --snapshot "${snapshot}" --panel not-a-panel

require_fails_with \
  'safe_mode blocks tenant-move' \
  cargo run -q -p ai_blaise_citus_tui -- \
    action --snapshot "${snapshot}" --kind tenant-move \
    --tenant tenant-a --target-worker worker-2

tui_action="${tmp_dir}/tui-action.tsv"
cargo run -q -p ai_blaise_citus_tui -- \
  action --snapshot "${snapshot}" --kind tenant-move \
  --tenant tenant-a --target-worker worker-2 \
  --unsafe-allow-mutation --confirm CONFIRM >"${tui_action}"
require_contains "${tui_action}" $'tenant-move	accepted	validated preview'

watch_frame="${tmp_dir}/watch-frame.txt"
watch_frame_repeat="${tmp_dir}/watch-frame-repeat.txt"
cargo run -q -p ai_blaise_citus_watch -- \
  render-frame --snapshot "${snapshot}" >"${watch_frame}"
cargo run -q -p ai_blaise_citus_watch -- \
  render-frame --snapshot "${snapshot}" >"${watch_frame_repeat}"
cmp "${watch_frame}" "${watch_frame_repeat}"
require_contains "${watch_frame}" 'citus-watch | cluster=prod-east | refresh=5s'
require_contains "${watch_frame}" 'vectorizer-backlog'
require_contains "${watch_frame}" 'companion.shard_placements'

watch_bad_snapshot="${tmp_dir}/watch-bad-snapshot.tsv"
cat >"${watch_bad_snapshot}" <<'TSV'
meta	cluster_name	prod-east
meta	generated_at	2026-05-23T22:00:00Z
worker	worker-1	10.0.0.11	primary	ready
table	public.events	tenant_id	nope	tenant	created_at	1 day	1	2
shard	public.events	102008	worker-1	active	1048576
TSV
require_fails_with \
  'table.shard_count has invalid numeric value nope' \
  cargo run -q -p ai_blaise_citus_watch -- \
    render-frame --snapshot "${watch_bad_snapshot}"

echo $'tools_ui_runtime_smoke	admin=ok	schema_designer=ok	tui=ok	watch=ok'
