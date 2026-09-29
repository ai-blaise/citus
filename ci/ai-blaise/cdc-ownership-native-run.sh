#!/usr/bin/env bash
set -Eeuo pipefail
# Runs inside an isolated PostgreSQL builder container as its unprivileged user.
readonly source_root=$1
readonly result_root=$2
mkdir -p "$result_root"
cd "$result_root"
export PATH="$(pg_config --bindir):$PATH"
pg_config --version
python3 "$source_root/ci/ai-blaise/cdc-ownership-extract.py" \
  "$source_root/src/backend/distributed/cdc/cdc_decoder.c" cdc-publish-extracted.inc | tee extraction.txt
slots=$(sed -n 's/^pg16_ownership_slots=//p' extraction.txt)
gcc -fPIC -shared -g -O2 -Wall -Wextra -Wno-unused-parameter \
  -Werror=incompatible-pointer-types -Werror=implicit-function-declaration \
  -I"$(pg_config --includedir-server)" -I"$(pg_config --includedir)" -I"$result_root" \
  -DCDC_HAS_PG16_OWNERSHIP_SLOTS="$slots" \
  "$source_root/ci/ai-blaise/cdc-ownership-native.c" -o cdc-ownership-native.so
initdb -D "$result_root/data" --auth-local=trust --auth-host=reject --no-sync > initdb.log
mkdir socket
finish() {
  status=$?
  trap - EXIT
  if pg_ctl -D "$result_root/data" status >/dev/null 2>&1; then
    pg_ctl -D "$result_root/data" -m fast -w stop || status=1
  fi
  printf 'EXIT=%s\nFINISHED_UTC=%s\n' "$status" "$(date -u +%FT%TZ)" > finish.txt
  exit "$status"
}
trap finish EXIT
trap 'exit 143' TERM
trap 'exit 130' INT
pg_ctl -D "$result_root/data" -l "$result_root/postgres.log" \
  -o "-k $result_root/socket -p 55539 -c listen_addresses='' -c shared_buffers=16MB -c max_connections=8" -w start
export PGHOST="$result_root/socket" PGPORT=55539 PGDATABASE=postgres
psql -X -v ON_ERROR_STOP=1 -c "CREATE FUNCTION cdc_ownership_native(text, boolean, integer) RETURNS text AS '$result_root/cdc-ownership-native', 'cdc_ownership_native' LANGUAGE C STRICT;"
passed=0
failed=0
for action in insert update-key update-full delete; do
  for changed in false true; do
    for fault in 0 1 2; do
      if psql -X -v ON_ERROR_STOP=1 -c "SELECT cdc_ownership_native('$action', $changed, $fault);"; then
        passed=$((passed+1))
      else
        failed=$((failed+1))
      fi
    done
  done
done
printf 'CASES_PASSED=%s\nCASES_FAILED=%s\n' "$passed" "$failed" | tee cases.txt
test "$passed" -eq 24
test "$failed" -eq 0
