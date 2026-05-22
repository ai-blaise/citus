#!/usr/bin/env bash
# FEATURE: O16
#
# Conversion-webhook smoke. Runs the operator's run-conversion-canonical mode
# and asserts the TSV output matches the contract published in
# operator/src/main.rs and the CRD bundle in
# command-center/helm/charts/citus-cluster/crds/. The smoke does not require a
# live API server -- it exercises the typed conversion path the HTTPS adapter
# delegates to. The kind-cluster apply path is covered by
# ci/ai-blaise/kind-production-smoke.sh; this script keeps the contract gated
# inside the operator workflow itself.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for the conversion-webhook smoke" >&2
  exit 1
fi

actual_canonical="$(cargo run -q -p ai_blaise_citus_operator -- run-conversion-canonical 2>&1)"

expected_canonical=$'kinds\tserved_versions\tstorage_version\tround_trips_passed\twebhook_path\twebhook_port\n17\t2\tv1alpha1\t17\t/convert\t8443'

if [[ "${actual_canonical}" != "${expected_canonical}" ]]; then
  echo "conversion canonical output mismatch:" >&2
  diff <(printf '%s\n' "${expected_canonical}") <(printf '%s\n' "${actual_canonical}") >&2 || true
  exit 1
fi

echo "conversion-webhook canonical contract verified"

# Re-run the round-trip tests to catch a future divergence between v1beta1
# and v1alpha1 immediately rather than at deploy time. Use a narrow test
# filter so the smoke runs in a few seconds even on a cold cache.
cargo test -q -p ai_blaise_citus_operator conversion:: -- --test-threads=1 \
  >/dev/null

echo "conversion-webhook round-trip tests verified"

# Sanity-check the CRD module catalog: every entry has both versions on disk.
missing=0
for module in \
  backup branch citus_cluster conflict_policy federation function hypertable \
  migration region scheduled_repack search_index shard_group sidecar \
  survival_goal tenant vectorizer webhook; do
  for file in mod.rs v1alpha1.rs v1beta1.rs; do
    path="operator/src/crds/${module}/${file}"
    if [[ ! -s "${path}" ]]; then
      echo "missing ${path}" >&2
      missing=1
    fi
  done
  handler="operator/src/conversion/${module}.rs"
  if [[ ! -s "${handler}" ]]; then
    echo "missing ${handler}" >&2
    missing=1
  fi
done

if [[ "${missing}" -ne 0 ]]; then
  exit 1
fi

echo "conversion-webhook artifact layout verified"
