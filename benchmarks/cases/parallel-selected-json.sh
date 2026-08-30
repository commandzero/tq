#!/usr/bin/env bash
set -euo pipefail

input=${1:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR]}
tq=${2:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR]}
archive_root=${TQ_BENCHMARK_ARCHIVE_ROOT:-$HOME/Development/commandzero/tq-benchmarks}
output_dir=${3:-$archive_root/.work/parallel-selected-json/$(date +%Y-%m-%d)}
runs=${RUNS:-3}
warmups=${WARMUPS:-2}
workers=${WORKERS:-$(sysctl -n hw.logicalcpu)}
query='[.features[].properties.release] | sort'

mkdir -p "$output_dir"

printf '{"features":[{"properties":{"release":2},"geometry":[1]},{"properties":{"release":1}}]}' |
  RAYON_NUM_THREADS="$workers" "$tq" -i json -o json -c --explain-json "$query" \
    > /dev/null 2> "$output_dir/explain.json"

plan=$(jq -r '.execution.plan' "$output_dir/explain.json")
preparation=$(jq -r '.execution.hybrid_proof.preparation // "none"' "$output_dir/explain.json")
parallel=$(jq -r '.execution.parallel_selected_decode.eligible' "$output_dir/explain.json")
if [[ $plan != hybrid-streaming-blocking || $preparation != stable-sort-runs || $parallel != true ]]; then
  echo "refusing benchmark: plan=$plan preparation=$preparation parallel=$parallel" >&2
  exit 2
fi

RAYON_NUM_THREADS=1 "$tq" -i json -o json -c \
  --max-vm-steps 1000000000 "$query" "$input" > "$output_dir/single.correctness"
RAYON_NUM_THREADS="$workers" "$tq" -i json -o json -c \
  --max-vm-steps 1000000000 --report-file "$output_dir/multi.report.json" \
  "$query" "$input" > "$output_dir/multi.correctness"
cmp "$output_dir/single.correctness" "$output_dir/multi.correctness"

correctness_sha=$(shasum -a 256 "$output_dir/multi.correctness" | awk '{print $1}')
input_sha=$(shasum -a 256 "$input" | awk '{print $1}')
binary_sha=$(shasum -a 256 "$tq" | awk '{print $1}')
input_bytes=$(stat -f %z "$input")

run_mode() {
  local threads=$1
  env RAYON_NUM_THREADS="$threads" "$tq" -i json -o json -c \
    --max-vm-steps 1000000000 "$query" "$input" > /dev/null
}

for ((warmup = 1; warmup <= warmups; warmup++)); do
  echo "warmup $warmup/$warmups" >&2
  run_mode 1
  run_mode "$workers"
done

echo 'mode,workers,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes' \
  > "$output_dir/samples.csv"

measure() {
  local mode=$1 threads=$2 run=$3
  local timing=$output_dir/$mode-$run.time
  echo "measure $run/$runs $mode" >&2
  /usr/bin/time -lp env RAYON_NUM_THREADS="$threads" "$tq" \
    -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" \
    > /dev/null 2> "$timing"
  awk -v mode="$mode" -v workers="$threads" -v run="$run" '
    /^real / { wall=$2 }
    /^user / { user=$2 }
    /^sys / { sys_time=$2 }
    /maximum resident set size/ { rss=$1 }
    END { printf "%s,%s,%s,%.2f,%.2f,%.2f,%.2f,%s\n", mode, workers, run, wall, user, sys_time, user+sys_time, rss }
  ' "$timing" >> "$output_dir/samples.csv"
}

for ((run = 1; run <= runs; run++)); do
  if ((run % 2 == 1)); then
    measure single 1 "$run"
    measure multi "$workers" "$run"
  else
    measure multi "$workers" "$run"
    measure single 1 "$run"
  fi
done

{
  echo "input=$input"
  echo "input_bytes=$input_bytes"
  echo "input_sha256=$input_sha"
  echo "correctness_sha256=$correctness_sha"
  echo "query=$query"
  echo "workers=$workers"
  echo "warmups=$warmups"
  echo "runs=$runs"
  echo "tq=$tq"
  echo "tq_sha256=$binary_sha"
} > "$output_dir/metadata.txt"

echo "$output_dir"
