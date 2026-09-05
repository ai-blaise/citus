#!/usr/bin/env bash
set -euo pipefail

# FEATURE: Bundle1
# Prove that the release-shaped operand boots through the stock postgres image
# entrypoint without command-line PostgreSQL configuration overrides.

repo_root="$(git rev-parse --show-toplevel)"
image="${BUNDLE1_IMAGE:-ai-blaise-citus-overlay:bundle1-light-pg17}"
pg_major="${BUNDLE1_PG_MAJOR:-17}"
expected_source_git_sha="${BUNDLE1_EXPECTED_SOURCE_GIT_SHA:-}"
expected_source_tree_state="${BUNDLE1_EXPECTED_SOURCE_TREE_STATE:-}"
expected_target="${BUNDLE1_EXPECTED_TARGET:-}"
expected_companion_version="0.1.2"
citus_control="${repo_root}/src/backend/distributed/citus.control"
preload_file="${repo_root}/images/citus-pg-overlay/shared-preload-libraries.conf"
manifest="${repo_root}/images/citus-pg-overlay/extension-manifest.tsv"
lockfile="${repo_root}/images/citus-pg-overlay/bundle1-source-build.lock.tsv"
initdb="${repo_root}/images/citus-pg-overlay/initdb.d/00-ai-blaise-extensions.sql"

for file in "${citus_control}" "${preload_file}" "${manifest}" "${lockfile}" "${initdb}"; do
  if [[ ! -s "${file}" ]]; then
    echo "missing Bundle1 default-boot input: ${file}" >&2
    exit 1
  fi
done
if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for the Bundle1 default-boot smoke" >&2
  exit 1
fi
if ! [[ "${pg_major}" =~ ^[0-9]+$ ]]; then
  echo "BUNDLE1_PG_MAJOR must be numeric: ${pg_major}" >&2
  exit 1
fi
if [[ -z "${expected_source_git_sha}" || -z "${expected_source_tree_state}" ]]; then
  echo "BUNDLE1_EXPECTED_SOURCE_GIT_SHA and BUNDLE1_EXPECTED_SOURCE_TREE_STATE are required" >&2
  exit 1
fi
case "${expected_target}" in
  bundle1-final-light)
    expected_scope="light-required-subset-minus-heavy-and-plrust"
    expected_release_target="false"
    ;;
  bundle1-final-full)
    expected_scope="full-bundle-required-minus-plrust"
    expected_release_target="true"
    ;;
  *)
    echo "BUNDLE1_EXPECTED_TARGET must be bundle1-final-light or bundle1-final-full" >&2
    exit 1
    ;;
esac

expected_citus_version="$(
  sed -n "s/^default_version = '\([^']*\)'$/\1/p" "${citus_control}"
)"
if [[ -z "${expected_citus_version}" ]]; then
  echo "Citus control file does not declare a default_version" >&2
  exit 1
fi

expected_preload="$({
  sed -n "s/^shared_preload_libraries = '\([^']*\)'$/\1/p" "${preload_file}"
} | head -n 1)"
expected_cohabit="$({
  sed -n "s/^citus\.cohabit_extensions = '\([^']*\)'$/\1/p" "${preload_file}"
} | head -n 1)"
if [[ -z "${expected_preload}" || -z "${expected_cohabit}" ]]; then
  echo "canonical Bundle1 preload settings are malformed" >&2
  exit 1
fi

container="ai-blaise-bundle1-default-boot-${RANDOM}-$$"
cleanup() {
  docker rm -fv "${container}" >/dev/null 2>&1 || true
}
trap cleanup EXIT

observed_target="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.target" }}' \
    "${image}"
)"
if [[ "${observed_target}" != "${expected_target}" ]]; then
  echo "Bundle1 target label mismatch: expected ${expected_target}, observed ${observed_target}" >&2
  exit 1
fi
observed_scope="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.evidence-scope" }}' \
    "${image}"
)"
if [[ "${observed_scope}" != "${expected_scope}" ]]; then
  echo "Bundle1 evidence-scope label mismatch: expected ${expected_scope}, observed ${observed_scope}" >&2
  exit 1
fi
observed_release_target="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.release-target" }}' \
    "${image}"
)"
if [[ "${observed_release_target}" != "${expected_release_target}" ]]; then
  echo "Bundle1 release-target label mismatch: expected ${expected_release_target}, observed ${observed_release_target}" >&2
  exit 1
fi
full_initdb_path="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.bundle1.full-initdb-path" }}' \
    "${image}"
)"
if [[ "${full_initdb_path}" != "true" ]]; then
  echo "${image} is not a full-initdb Bundle1 image (label=${full_initdb_path})" >&2
  exit 1
fi
observed_source_git_sha="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.source-git-sha" }}' \
    "${image}"
)"
if [[ "${observed_source_git_sha}" != "${expected_source_git_sha}" ]]; then
  echo "source Git SHA label mismatch: expected ${expected_source_git_sha}, observed ${observed_source_git_sha}" >&2
  exit 1
fi
observed_source_tree_state="$(
  docker image inspect \
    -f '{{ index .Config.Labels "ai-blaise.citus.source-tree-state" }}' \
    "${image}"
)"
if [[ "${observed_source_tree_state}" != "${expected_source_tree_state}" ]]; then
  echo "source tree-state label mismatch: expected ${expected_source_tree_state}, observed ${observed_source_tree_state}" >&2
  exit 1
fi

# Deliberately pass no postgres command or -c override. The image's copied
# postgresql.conf.sample must include the canonical preload file before the
# entrypoint starts its temporary initdb server.
docker run \
  --name "${container}" \
  -e POSTGRES_PASSWORD=postgres \
  -e PGSODIUM_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  -d "${image}" >/dev/null

init_complete=0
for _ in $(seq 1 300); do
  container_logs="$(docker logs "${container}" 2>&1 || true)"
  if [[ "${container_logs}" == *"PostgreSQL init process complete"* ]]; then
    init_complete=1
    break
  fi
  container_running="$(docker inspect -f '{{.State.Running}}' "${container}" 2>/dev/null || true)"
  if [[ "${container_running}" != "true" ]]; then
    docker logs "${container}" >&2 || true
    echo "Bundle1 default-boot container exited during init" >&2
    exit 1
  fi
  sleep 1
done
if [[ "${init_complete}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "Bundle1 default-boot container did not complete initdb" >&2
  exit 1
fi

ready=0
for _ in $(seq 1 120); do
  if docker exec "${container}" psql -U postgres -Atqc 'SELECT 1' >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done
if [[ "${ready}" != "1" ]]; then
  docker logs "${container}" >&2 || true
  echo "Bundle1 default-boot container did not become ready" >&2
  exit 1
fi

observed_pg_major="$(
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT current_setting('server_version_num')::integer / 10000"
)"
if [[ "${observed_pg_major}" != "${pg_major}" ]]; then
  echo "PostgreSQL major mismatch: expected ${pg_major}, observed ${observed_pg_major}" >&2
  exit 1
fi
observed_companion_version="$(
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT extversion FROM pg_extension WHERE extname = 'ai_blaise_citus'"
)"
if [[ "${observed_companion_version}" != "${expected_companion_version}" ]]; then
  echo "ai_blaise_citus version mismatch: expected ${expected_companion_version}, observed ${observed_companion_version}" >&2
  exit 1
fi
observed_citus_version="$(
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT extversion FROM pg_extension WHERE extname = 'citus'"
)"
if [[ "${observed_citus_version}" != "${expected_citus_version}" ]]; then
  echo "Citus version mismatch: expected ${expected_citus_version}, observed ${observed_citus_version}" >&2
  exit 1
fi
observed_preload="$(
  docker exec "${container}" psql -U postgres -Atqc 'SHOW shared_preload_libraries'
)"
if [[ "${observed_preload}" != "${expected_preload}" ]]; then
  echo "default shared_preload_libraries mismatch: expected ${expected_preload}, observed ${observed_preload}" >&2
  exit 1
fi
observed_cohabit="$(
  docker exec "${container}" psql -U postgres -Atqc 'SHOW citus.cohabit_extensions'
)"
if [[ "${observed_cohabit}" != "${expected_cohabit}" ]]; then
  echo "default citus.cohabit_extensions mismatch: expected ${expected_cohabit}, observed ${observed_cohabit}" >&2
  exit 1
fi

# Exercise the installed companion checks against the settings the stock
# entrypoint actually applied. The negative controls ensure this cannot become
# a presence-only call that accepts Citus-first or missing-required-library
# configurations.
docker exec -i "${container}" psql -v ON_ERROR_STOP=1 -U postgres -Atq <<'SQL'
SELECT companion_internal.assert_shared_preload_libraries(
  string_to_array(current_setting('shared_preload_libraries'), ','),
  string_to_array(current_setting('citus.cohabit_extensions'), ',')
);
SELECT companion_internal.assert_citus_cohabit_extension_order();

DO $bundle1_negative_controls$
BEGIN
  BEGIN
    PERFORM companion_internal.assert_citus_cohabit_extension_order(
      ARRAY['citus', 'timescaledb']
    );
    RAISE EXCEPTION 'Bundle1 negative order control accepted Citus-first preload';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'citus must be loaded after trusted cohabiting extensions' THEN
        RAISE;
      END IF;
  END;

  BEGIN
    PERFORM companion_internal.assert_shared_preload_libraries(
      string_to_array(current_setting('shared_preload_libraries'), ','),
      string_to_array(current_setting('citus.cohabit_extensions'), ',')
        || ARRAY['bundle1_missing_library_control']
    );
    RAISE EXCEPTION 'Bundle1 negative required-library control accepted a missing library';
  EXCEPTION
    WHEN raise_exception THEN
      IF SQLERRM <> 'required cohabiting extension is not preloaded' THEN
        RAISE;
      END IF;
  END;
END;
$bundle1_negative_controls$;
SQL

observed_preload_source="$(
  docker exec "${container}" psql -U postgres -Atqc \
    "SELECT sourcefile FROM pg_file_settings WHERE name = 'shared_preload_libraries' AND applied ORDER BY seqno DESC LIMIT 1"
)"
if [[ "${observed_preload_source}" != "/etc/postgresql/ai-blaise/shared-preload-libraries.conf" ]]; then
  echo "default preload source mismatch: ${observed_preload_source}" >&2
  exit 1
fi

full_only_extensions="$(
  awk -F '\t' 'NR > 1 && $2 == "full" { print $1 }' "${lockfile}"
)"
if [[ -z "${full_only_extensions}" ]]; then
  echo "Bundle1 lockfile does not define any full-only extensions" >&2
  exit 1
fi

missing_extensions=""
expected_sql_extension_count=0
expected_preload_only_count=0
required_manifest_count="$(
  awk -F '|' '!/^#/ && $2 == "required" { count++ } END { print count + 0 }' "${manifest}"
)"
while IFS= read -r extension; do
  if [[ "${expected_target}" == "bundle1-final-light" ]] \
    && grep -Fxq "${extension}" <<<"${full_only_extensions}"; then
    continue
  fi
  if grep -Fq "CREATE EXTENSION IF NOT EXISTS ${extension};" "${initdb}"; then
    expected_sql_extension_count=$((expected_sql_extension_count + 1))
  elif [[ ",${expected_preload}," == *",${extension},"* ]]; then
    # A required preload-only capability has no pg_extension catalog row. Its
    # presence is proven by the exact SHOW shared_preload_libraries check above.
    expected_preload_only_count=$((expected_preload_only_count + 1))
    continue
  else
    echo "required ${expected_target} extension has neither initdb creation nor preload coverage: ${extension}" >&2
    exit 1
  fi
  present="$(
    docker exec "${container}" psql -U postgres -Atqc \
      "SELECT 1 FROM pg_extension WHERE extname = '${extension}'"
  )"
  if [[ "${present}" != "1" ]]; then
    missing_extensions+=" ${extension}"
  fi
done < <(awk -F'|' '!/^#/ && $2 == "required" { print $1 }' "${manifest}")
if [[ -n "${missing_extensions}" ]]; then
  docker logs "${container}" >&2 || true
  echo "Bundle1 default boot did not install:${missing_extensions}" >&2
  exit 1
fi
if [[ "${expected_target}" == "bundle1-final-full" ]] \
  && [[ "$((expected_sql_extension_count + expected_preload_only_count))" -ne "${required_manifest_count}" ]]; then
  echo "full Bundle1 check did not cover every required manifest entry" >&2
  exit 1
fi

printf 'Bundle1 default-boot smoke passed for %s (target=%s, scope=%s, release_target=%s, PG%s, source_git_sha=%s, source_tree_state=%s, citus=%s, ai_blaise_citus=%s, sql_extensions=%s, preload_only=%s)\n' \
  "${image}" \
  "${observed_target}" \
  "${observed_scope}" \
  "${observed_release_target}" \
  "${pg_major}" \
  "${observed_source_git_sha}" \
  "${observed_source_tree_state}" \
  "${observed_citus_version}" \
  "${observed_companion_version}" \
  "${expected_sql_extension_count}" \
  "${expected_preload_only_count}"
