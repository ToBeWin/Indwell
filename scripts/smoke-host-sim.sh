#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

host="${INDWELL_SMOKE_HOST:-127.0.0.1:3030}"
base_url="http://${host}"
data_dir="$(mktemp -d)"
log_file="${data_dir}/host-sim.log"

cleanup() {
  if [[ -n "${server_pid:-}" ]]; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi
  rm -rf "${data_dir}"
}
trap cleanup EXIT

INDWELL_DATA_DIR="${data_dir}" INDWELL_HOST_SIM_ADDR="${host}" cargo run -p indwell-host-sim >"${log_file}" 2>&1 &
server_pid="$!"

for _ in $(seq 1 60); do
  if curl -fsS "${base_url}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

curl -fsS "${base_url}/health" >/dev/null

curl -fsS \
  -X POST "${base_url}/v1/channel/input" \
  -H "content-type: application/json" \
  -d '{"channel":"local_pwa","session_id":"smoke-channel","subject_hint":"owner","text":"remember I like quiet mornings"}' \
  >/dev/null

curl -fsS \
  -X POST "${base_url}/v1/voice/mock-turn" \
  -H "content-type: application/json" \
  -d '{"text_hint":"hello indwell","voice":"warm_indwell"}' \
  >/dev/null

echo "host-sim smoke passed at ${base_url}"
