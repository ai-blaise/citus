#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

registry="${IMAGE_REGISTRY:-ghcr.io/ai-blaise}"
tag="${TAG:-0.1.0}"
dockerfile="${DOCKERFILE:-images/rust-runtime/Dockerfile}"
push="${PUSH:-false}"
digest_file="${DIGEST_FILE:-artifacts/ai-blaise-image-digests.tsv}"

if [[ -n "${digest_file}" ]]; then
  mkdir -p "$(dirname "${digest_file}")"
  printf 'repository\timage\ttag\tdigest\tpackage\tbinary\tpushed\n' >"${digest_file}"
fi

images=(
  "citus-operator|ai_blaise_citus_operator|ai_blaise_citus_operator"
  "citus-pool|ai_blaise_citus_pool|ai_blaise_citus_pool"
  "citus-sidecar-analytical|ai_blaise_citus_sidecar_analytical|ai_blaise_citus_sidecar_analytical"
  "citus-sidecar-auth|ai_blaise_citus_sidecar_auth|ai_blaise_citus_sidecar_auth"
  "citus-sidecar-backup|ai_blaise_citus_sidecar_backup|ai_blaise_citus_sidecar_backup"
  "citus-sidecar-cdc|ai_blaise_citus_sidecar_cdc|ai_blaise_citus_sidecar_cdc"
  "citus-sidecar-coldtier|ai_blaise_citus_sidecar_coldtier|ai_blaise_citus_sidecar_coldtier"
  "citus-sidecar-edge-functions|ai_blaise_citus_sidecar_edge_functions|ai_blaise_citus_sidecar_edge_functions"
  "citus-sidecar-graphql|ai_blaise_citus_sidecar_graphql|ai_blaise_citus_sidecar_graphql"
  "citus-sidecar-hlc|ai_blaise_citus_sidecar_hlc|ai_blaise_citus_sidecar_hlc"
  "citus-sidecar-mcp|ai_blaise_citus_sidecar_mcp|ai_blaise_citus_sidecar_mcp"
  "citus-sidecar-postgrest|ai_blaise_citus_sidecar_postgrest|ai_blaise_citus_sidecar_postgrest"
  "citus-sidecar-raft|ai_blaise_citus_sidecar_raft|ai_blaise_citus_sidecar_raft"
  "citus-sidecar-realtime|ai_blaise_citus_sidecar_realtime|ai_blaise_citus_sidecar_realtime"
  "citus-sidecar-repack|ai_blaise_citus_sidecar_repack|ai_blaise_citus_sidecar_repack"
  "citus-sidecar-schema-job|ai_blaise_citus_sidecar_schema_job|ai_blaise_citus_sidecar_schema_job"
  "citus-sidecar-storage|ai_blaise_citus_sidecar_storage|ai_blaise_citus_sidecar_storage"
  "citus-sidecar-txn-status|ai_blaise_citus_sidecar_txn_status|ai_blaise_citus_sidecar_txn_status"
  "citus-sidecar-vectorizer|ai_blaise_citus_sidecar_vectorizer|ai_blaise_citus_sidecar_vectorizer"
  "citusctl|ai_blaise_citusctl|ai_blaise_citusctl"
)

for image in "${images[@]}"; do
  IFS="|" read -r repository package binary <<< "${image}"
  full_image="${registry}/${repository}:${tag}"
  push_output=""

  docker build \
    --file "${dockerfile}" \
    --build-arg "PACKAGE=${package}" \
    --build-arg "BIN=${binary}" \
    --tag "${full_image}" \
    .

  if [[ "${push}" == "true" ]]; then
    push_output="$(docker push "${full_image}")"
    printf '%s\n' "${push_output}"
  fi

  digest="$(
    printf '%s\n' "${push_output}" |
      awk '/digest: sha256:/ { for (i = 1; i <= NF; i++) if ($i ~ /^sha256:/) { print $i; exit } }'
  )"
  repo_digest="$(
    docker image inspect \
      --format '{{range .RepoDigests}}{{println .}}{{end}}' \
      "${full_image}" 2>/dev/null |
      awk -v prefix="${registry}/${repository}@" 'index($0, prefix) == 1 { print $0; exit }'
  )"
  if [[ -z "${digest}" && -n "${repo_digest}" ]]; then
    digest="${repo_digest#*@}"
  fi

  if [[ "${push}" == "true" && ! "${digest}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
    echo "pushed image ${full_image} did not report an immutable repo digest" >&2
    exit 1
  fi

  if [[ -n "${digest_file}" ]]; then
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "${repository}" \
      "${full_image}" \
      "${tag}" \
      "${digest}" \
      "${package}" \
      "${binary}" \
      "${push}" >>"${digest_file}"
  fi
done
