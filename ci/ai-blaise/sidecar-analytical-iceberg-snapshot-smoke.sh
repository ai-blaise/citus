#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
if [[ -f "${HOME}/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.cargo/env"
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

commit_dir="${tmp_dir}/iceberg"
output="$(AI_BLAISE_ICEBERG_SNAPSHOT_DIR="${commit_dir}" cargo run -q -p ai_blaise_citus_sidecar_analytical -- run-local-iceberg-snapshot-commit-canonical)"
header="$(printf '%s\n' "${output}" | sed -n '1p')"
row="$(printf '%s\n' "${output}" | sed -n '2p')"

expected_header=$'feature_id\ttransaction_id\tsnapshot_id\tprepare_lsn\tmanifest_uri\tmetadata_path\tmanifest_path\tcurrent_pointer_path\tmetadata_bytes\tmanifest_bytes\tlocal_metadata_written\tlocal_manifest_written\tcurrent_pointer_committed\tprepare_lsn_recorded\tsnapshot_metadata_round_tripped\tatomic_rename_used\tfsync_executed\ticeberg_catalog_commit_exercised\tobject_store_io_attempted\tcitus_prepare_hook_exercised\tmulti_writer_conflict_detection_exercised\twarehouse_federation_exercised\tkubernetes_traffic_exercised\tevidence_boundary'
if [[ "${header}" != "${expected_header}" ]]; then
  echo "unexpected local Iceberg snapshot header" >&2
  printf '%s\n' "${header}" >&2
  exit 1
fi

IFS=$'\t' read -r \
  feature_id transaction_id snapshot_id prepare_lsn manifest_uri metadata_path \
  manifest_path current_pointer_path metadata_bytes manifest_bytes \
  local_metadata_written local_manifest_written current_pointer_committed \
  prepare_lsn_recorded snapshot_metadata_round_tripped atomic_rename_used \
  fsync_executed iceberg_catalog_commit_exercised object_store_io_attempted \
  citus_prepare_hook_exercised multi_writer_conflict_detection_exercised \
  warehouse_federation_exercised kubernetes_traffic_exercised evidence_boundary <<<"${row}"

[[ "${feature_id}" == "L5" ]]
[[ "${transaction_id}" == "tx-1" ]]
[[ "${snapshot_id}" == "snapshot-1" ]]
[[ "${prepare_lsn}" == "16/B374D848" ]]
[[ "${manifest_uri}" == "s3://lake/warehouse/orders/metadata/manifest.avro" ]]
[[ "${metadata_path}" == "${commit_dir}/snapshot-1.metadata.json" ]]
[[ "${manifest_path}" == "${commit_dir}/snapshot-1.manifest.json" ]]
[[ "${current_pointer_path}" == "${commit_dir}/current-snapshot.txt" ]]
[[ "${metadata_bytes}" =~ ^[0-9]+$ ]]
[[ "${manifest_bytes}" =~ ^[0-9]+$ ]]
(( metadata_bytes > 0 ))
(( manifest_bytes > 0 ))
[[ -s "${metadata_path}" ]]
[[ -s "${manifest_path}" ]]
[[ "$(cat "${current_pointer_path}")" == "snapshot-1" ]]
[[ "${local_metadata_written}" == "true" ]]
[[ "${local_manifest_written}" == "true" ]]
[[ "${current_pointer_committed}" == "true" ]]
[[ "${prepare_lsn_recorded}" == "true" ]]
[[ "${snapshot_metadata_round_tripped}" == "true" ]]
[[ "${atomic_rename_used}" == "true" ]]
[[ "${fsync_executed}" == "true" ]]
[[ "${iceberg_catalog_commit_exercised}" == "false" ]]
[[ "${object_store_io_attempted}" == "false" ]]
[[ "${citus_prepare_hook_exercised}" == "false" ]]
[[ "${multi_writer_conflict_detection_exercised}" == "false" ]]
[[ "${warehouse_federation_exercised}" == "false" ]]
[[ "${kubernetes_traffic_exercised}" == "false" ]]
[[ "${evidence_boundary}" == "local-iceberg-snapshot-metadata-commit-only" ]]

python3 - "${metadata_path}" "${manifest_path}" <<'PYVERIFY'
import json
import sys
metadata_path, manifest_path = sys.argv[1:3]
metadata = json.load(open(metadata_path, encoding="utf-8"))
manifest = json.load(open(manifest_path, encoding="utf-8"))
assert metadata["format_version"] == 2
assert metadata["snapshot_id"] == "snapshot-1"
assert metadata["prepare_lsn"] == "16/B374D848"
assert metadata["committed_at_boundary"] == "prepare-lsn-local-metadata"
assert manifest["format_version"] == 1
assert manifest["data_files"][0]["file_format"] == "parquet"
assert manifest["data_files"][0]["record_count"] == 4
PYVERIFY

printf 'iceberg_snapshot_commit_live=passed\n'
printf 'l5_local_metadata_written=%s\n' "${local_metadata_written}"
printf 'l5_local_manifest_written=%s\n' "${local_manifest_written}"
printf 'l5_current_pointer_committed=%s\n' "${current_pointer_committed}"
printf 'l5_prepare_lsn_recorded=%s\n' "${prepare_lsn_recorded}"
printf 'l5_snapshot_metadata_round_tripped=%s\n' "${snapshot_metadata_round_tripped}"
printf 'atomic_rename_used=%s\n' "${atomic_rename_used}"
printf 'fsync_executed=%s\n' "${fsync_executed}"
printf 'iceberg_catalog_commit_exercised=%s\n' "${iceberg_catalog_commit_exercised}"
printf 'object_store_io_attempted=%s\n' "${object_store_io_attempted}"
printf 'citus_prepare_hook_exercised=%s\n' "${citus_prepare_hook_exercised}"
printf 'multi_writer_conflict_detection_exercised=%s\n' "${multi_writer_conflict_detection_exercised}"
printf 'warehouse_federation_exercised=%s\n' "${warehouse_federation_exercised}"
printf 'kubernetes_traffic_exercised=%s\n' "${kubernetes_traffic_exercised}"
printf 'sidecar_analytical_iceberg_snapshot_smoke\tfeature_id=%s\tsnapshot_id=%s\tprepare_lsn=%s\tevidence_boundary=%s\n' "${feature_id}" "${snapshot_id}" "${prepare_lsn}" "${evidence_boundary}"
