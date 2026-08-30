#!/usr/bin/env bash
set -euo pipefail

input=${1:?usage: fast-discard.sh INPUT BASELINE_TQ CANDIDATE_TQ [OUTPUT_DIR]}
baseline_tq=${2:?usage: fast-discard.sh INPUT BASELINE_TQ CANDIDATE_TQ [OUTPUT_DIR]}
candidate_tq=${3:?usage: fast-discard.sh INPUT BASELINE_TQ CANDIDATE_TQ [OUTPUT_DIR]}
archive_root=${TQ_BENCHMARK_ARCHIVE_ROOT:-$HOME/Development/commandzero/tq-benchmarks}
output_dir=${4:-$archive_root/.work/fast-discard/$(date +%Y-%m-%d)}
runs=${RUNS:-3}
warmups=${WARMUPS:-2}
workers=${WORKERS:-$(sysctl -n hw.logicalcpu)}
query='[.features[].properties.release] | sort'

mkdir -p "$output_dir"

check_plan() {
  local label=$1 binary=$2
  local explain=$output_dir/$label.explain.json
  printf '{"features":[{"properties":{"release":2},"geometry":[1]},{"properties":{"release":1}}]}' | \
    "$binary" -i json -o json -c --explain-json "$query" \
    > /dev/null 2> "$explain"
  local plan preparation rewrites
  plan=$(jq -r '.execution.plan' "$explain")
  preparation=$(jq -r '.execution.hybrid_proof.preparation // "none"' "$explain")
  rewrites=$(jq -r '.execution.optimizer_rewrites | length' "$explain")
  if [[ $plan != hybrid-streaming-blocking || $preparation != stable-sort-runs || $rewrites != 0 ]]; then
    echo "refusing fast-discard benchmark for $label: plan=$plan preparation=$preparation rewrites=$rewrites" >&2
    exit 2
  fi
}

check_plan baseline "$baseline_tq"
check_plan candidate "$candidate_tq"

RAYON_NUM_THREADS=1 "$baseline_tq" -i json -o json -c \
  --max-vm-steps 1000000000 "$query" "$input" > "$output_dir/baseline.correctness"
RAYON_NUM_THREADS=1 "$candidate_tq" -i json -o json -c \
  --max-vm-steps 1000000000 "$query" "$input" > "$output_dir/candidate.correctness"
cmp "$output_dir/baseline.correctness" "$output_dir/candidate.correctness"
correctness_sha=$(shasum -a 256 "$output_dir/candidate.correctness" | awk '{print $1}')
input_sha=$(shasum -a 256 "$input" | awk '{print $1}')
baseline_sha=$(shasum -a 256 "$baseline_tq" | awk '{print $1}')
candidate_sha=$(shasum -a 256 "$candidate_tq" | awk '{print $1}')
input_bytes=$(stat -f %z "$input")

run_mode() {
  local label=$1 threads=$2 binary=$3
  env RAYON_NUM_THREADS="$threads" "$binary" -i json -o json -c \
    --max-vm-steps 1000000000 "$query" "$input" > /dev/null
}

for ((warmup = 1; warmup <= warmups; warmup++)); do
  echo "warmup $warmup/$warmups" >&2
  run_mode baseline-single 1 "$baseline_tq"
  run_mode candidate-single 1 "$candidate_tq"
  run_mode baseline-multi "$workers" "$baseline_tq"
  run_mode candidate-multi "$workers" "$candidate_tq"
done

echo 'tool,workers,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes' \
  > "$output_dir/samples.csv"

measure() {
  local label=$1 threads=$2 run=$3 binary=$4
  local timing=$output_dir/$label-$run.time
  echo "measure $run/$runs $label" >&2
  /usr/bin/time -lp env RAYON_NUM_THREADS="$threads" "$binary" \
    -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" \
    > /dev/null 2> "$timing"
  awk -v tool="$label" -v workers="$threads" -v run="$run" '
    /^real / { wall=$2 }
    /^user / { user=$2 }
    /^sys / { sys_time=$2 }
    /maximum resident set size/ { rss=$1 }
    END { printf "%s,%s,%s,%.2f,%.2f,%.2f,%.2f,%s\n", tool, workers, run, wall, user, sys_time, user+sys_time, rss }
  ' "$timing" >> "$output_dir/samples.csv"
}

for ((run = 1; run <= runs; run++)); do
  if ((run % 2 == 1)); then
    measure baseline-single 1 "$run" "$baseline_tq"
    measure candidate-single 1 "$run" "$candidate_tq"
    measure baseline-multi "$workers" "$run" "$baseline_tq"
    measure candidate-multi "$workers" "$run" "$candidate_tq"
  else
    measure candidate-multi "$workers" "$run" "$candidate_tq"
    measure baseline-multi "$workers" "$run" "$baseline_tq"
    measure candidate-single 1 "$run" "$candidate_tq"
    measure baseline-single 1 "$run" "$baseline_tq"
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
  echo "baseline_tq=$baseline_tq"
  echo "baseline_sha256=$baseline_sha"
  echo "candidate_tq=$candidate_tq"
  echo "candidate_sha256=$candidate_sha"
} > "$output_dir/metadata.txt"

echo "$output_dir"
