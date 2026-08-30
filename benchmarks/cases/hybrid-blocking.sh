#!/usr/bin/env bash
set -euo pipefail

input=${1:?usage: hybrid-blocking.sh INPUT TQ_BIN [OUTPUT_DIR] [JQ_BIN]}
tq_bin=${2:?usage: hybrid-blocking.sh INPUT TQ_BIN [OUTPUT_DIR] [JQ_BIN]}
archive_root=${TQ_BENCHMARK_ARCHIVE_ROOT:-$HOME/Development/commandzero/tq-benchmarks}
output_dir=${3:-$archive_root/.work/hybrid-blocking/$(date +%Y-%m-%d)}
jq_bin=${4:-jq}
runs=${RUNS:-3}
warmups=${WARMUPS:-2}
workers=${WORKERS:-$(sysctl -n hw.logicalcpu)}
query='[.features[].properties.release] | sort'

mkdir -p "$output_dir"

printf '{"features":[{"properties":{"release":2}},{"properties":{"release":1}}]}' | \
  "$tq_bin" -i json -o json -c --explain-json "$query" \
  > /dev/null 2> "$output_dir/explain.json"
plan=$($jq_bin -r '.execution.plan' "$output_dir/explain.json")
preparation=$($jq_bin -r '.execution.hybrid_proof.preparation // "none"' "$output_dir/explain.json")
rewrites=$($jq_bin -r '.execution.optimizer_rewrites | length' "$output_dir/explain.json")
if [[ $plan != hybrid-streaming-blocking || $preparation != stable-sort-runs || $rewrites != 0 ]]; then
  echo "refusing blocking benchmark: plan=$plan preparation=$preparation rewrites=$rewrites" >&2
  exit 2
fi

"$jq_bin" -c "$query" "$input" > "$output_dir/jq.correctness"
TQ_BENCH_FORCE_DOCUMENT=1 RAYON_NUM_THREADS=1 \
  "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" \
  > "$output_dir/tq-document.correctness"
RAYON_NUM_THREADS=1 \
  "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" \
  > "$output_dir/tq-hybrid-single.correctness"
RAYON_NUM_THREADS="$workers" \
  "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" \
  > "$output_dir/tq-hybrid-multi.correctness"
for candidate in tq-document tq-hybrid-single tq-hybrid-multi; do
  cmp "$output_dir/jq.correctness" "$output_dir/$candidate.correctness"
done
jq_correctness_sha=$(shasum -a 256 "$output_dir/jq.correctness" | awk '{print $1}')
document_correctness_sha=$(shasum -a 256 "$output_dir/tq-document.correctness" | awk '{print $1}')
single_correctness_sha=$(shasum -a 256 "$output_dir/tq-hybrid-single.correctness" | awk '{print $1}')
multi_correctness_sha=$(shasum -a 256 "$output_dir/tq-hybrid-multi.correctness" | awk '{print $1}')
input_sha=$(shasum -a 256 "$input" | awk '{print $1}')
input_bytes=$(stat -f %z "$input")

echo 'tool,workers,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes' \
  > "$output_dir/samples.csv"

for ((warmup = 1; warmup <= warmups; warmup++)); do
  "$jq_bin" -c "$query" "$input" > /dev/null
  TQ_BENCH_FORCE_DOCUMENT=1 RAYON_NUM_THREADS=1 \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" > /dev/null
  RAYON_NUM_THREADS=1 \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" > /dev/null
  RAYON_NUM_THREADS="$workers" \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input" > /dev/null
done

measure() {
  local tool=$1 thread_count=$2 run=$3
  local timing=$output_dir/$tool-$run.time
  shift 3
  /usr/bin/time -lp env RAYON_NUM_THREADS="$thread_count" "$@" \
    > /dev/null 2> "$timing"
  awk -v tool="$tool" -v workers="$thread_count" -v run="$run" '
    /^real / { wall=$2 }
    /^user / { user=$2 }
    /^sys / { sys_time=$2 }
    /maximum resident set size/ { rss=$1 }
    END { printf "%s,%s,%s,%.2f,%.2f,%.2f,%.2f,%s\n", tool, workers, run, wall, user, sys_time, user+sys_time, rss }
  ' "$timing" >> "$output_dir/samples.csv"
}

for ((run = 1; run <= runs; run++)); do
  measure jq 1 "$run" "$jq_bin" -c "$query" "$input"
  measure tq-document 1 "$run" env TQ_BENCH_FORCE_DOCUMENT=1 \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input"
  measure tq-hybrid-single 1 "$run" \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input"
  measure tq-hybrid-multi "$workers" "$run" \
    "$tq_bin" -i json -o json -c --max-vm-steps 1000000000 "$query" "$input"
done

{
  echo "input=$input"
  echo "input_bytes=$input_bytes"
  echo "input_sha256=$input_sha"
  echo "jq_correctness_sha256=$jq_correctness_sha"
  echo "tq_document_correctness_sha256=$document_correctness_sha"
  echo "tq_hybrid_single_correctness_sha256=$single_correctness_sha"
  echo "tq_hybrid_multi_correctness_sha256=$multi_correctness_sha"
  echo "query=$query"
  echo "plan=$plan"
  echo "preparation=$preparation"
  echo "workers=$workers"
  echo "warmups=$warmups"
  echo "runs=$runs"
  echo "tq_bin=$tq_bin"
  echo "jq_bin=$jq_bin"
} > "$output_dir/metadata.txt"

echo "$output_dir"
