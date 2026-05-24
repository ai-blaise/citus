#!/usr/bin/env bash
set -euo pipefail

# FEATURE: D13

registry="${IMAGE_REGISTRY:-ghcr.io/ai-blaise}"
tag="${TAG:-0.1.0}"
dockerfile="${DOCKERFILE:-images/rust-runtime/Dockerfile}"
push="${PUSH:-false}"
digest_file="${DIGEST_FILE:-artifacts/ai-blaise-image-digests.tsv}"
source_revision="${SOURCE_REVISION:-}"

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
  "citusctl|ai_blaise_citusctl|ai_blaise_citusctl|plan inspect cluster"
)

if [[ "${LIST_IMAGES:-false}" == "true" ]]; then
  printf 'repository\tpackage\tbinary\tdefault_args\n'
  for image in "${images[@]}"; do
    IFS="|" read -r repository package binary default_args <<< "${image}"
    printf '%s\t%s\t%s\t%s\n' \
      "${repository}" \
      "${package}" \
      "${binary}" \
      "${default_args:-serve}"
  done
  exit 0
fi

if [[ "${push}" != "true" && "${push}" != "false" ]]; then
  echo "PUSH must be either true or false" >&2
  exit 1
fi

if [[ -z "${registry}" ]]; then
  echo "IMAGE_REGISTRY must not be empty" >&2
  exit 1
fi

if [[ -z "${tag}" ]]; then
  echo "TAG must not be empty" >&2
  exit 1
fi

if [[ "${tag}" =~ [[:space:]/@] ]]; then
  echo "release image tag contains invalid characters: ${tag}" >&2
  exit 1
fi

if [[ "${push}" == "true" ]]; then
  if [[ "${tag}" =~ ^(latest|main|master|dev|test|local)$ ]]; then
    echo "release image tag must not be mutable: ${tag}" >&2
    exit 1
  fi
  if [[ -z "${IMAGE_REGISTRY+x}" || -z "${IMAGE_REGISTRY}" ]]; then
    echo "PUSH=true requires IMAGE_REGISTRY to be set explicitly" >&2
    exit 1
  fi
  if [[ -z "${TAG+x}" || -z "${TAG}" ]]; then
    echo "PUSH=true requires TAG to be set explicitly" >&2
    exit 1
  fi
  if [[ -z "${digest_file}" ]]; then
    echo "PUSH=true requires DIGEST_FILE so the immutable image handoff is durable" >&2
    exit 1
  fi
  if [[ "${registry}" =~ ^(localhost|127\.0\.0\.1)([:/]|$) && "${ALLOW_LOCAL_IMAGE_REGISTRY:-false}" != "true" ]]; then
    echo "local IMAGE_REGISTRY requires ALLOW_LOCAL_IMAGE_REGISTRY=true and is not release evidence" >&2
    exit 1
  fi
fi

if [[ -z "${source_revision}" ]]; then
  source_revision="$(git rev-parse --verify HEAD 2>/dev/null || printf unknown)"
fi

if [[ -n "${digest_file}" ]]; then
  mkdir -p "$(dirname "${digest_file}")"
  printf 'source_revision\trepository\timage\ttag\tdigest\tpackage\tbinary\tpushed\n' >"${digest_file}"
fi

for image in "${images[@]}"; do
  IFS="|" read -r repository package binary default_args <<< "${image}"
  default_args="${default_args:-serve}"
  full_image="${registry}/${repository}:${tag}"
  push_output=""

  docker build \
    --file "${dockerfile}" \
    --build-arg "PACKAGE=${package}" \
    --build-arg "BIN=${binary}" \
    --build-arg "DEFAULT_ARGS=${default_args}" \
    --label "org.opencontainers.image.source=https://github.com/ai-blaise/citus" \
    --label "org.opencontainers.image.revision=${source_revision}" \
    --label "org.opencontainers.image.version=${tag}" \
    --label "org.opencontainers.image.title=ai-blaise/citus ${repository}" \
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
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "${source_revision}" \
      "${repository}" \
      "${full_image}" \
      "${tag}" \
      "${digest}" \
      "${package}" \
      "${binary}" \
      "${push}" >>"${digest_file}"
  fi
done
