#!/usr/bin/env bash
set -euo pipefail

input=${1:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR] [JQ_BIN]}
tq=${2:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR] [JQ_BIN]}
archive_root=${TQ_BENCHMARK_ARCHIVE_ROOT:-$HOME/Development/commandzero/tq-benchmarks}
output_dir=${3:-$archive_root/.work/parallel-selected-json/$(date +%Y-%m-%d)}
jq_bin=${4:-${TQ_JQ:-jq}}
runs=${RUNS:-3}
warmups=${WARMUPS:-2}
if [[ $jq_bin != */* ]]; then
  jq_bin_path=$(command -v "$jq_bin" || true)
  jq_bin=$jq_bin_path
fi
if [[ ! -x $jq_bin ]]; then
  echo "parallel benchmark requires an executable jq: $jq_bin" >&2
  exit 69
fi
if [ -n "${WORKERS:-}" ]; then
  workers=$WORKERS
else
  workers=$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)
fi
worker_counts=(1 4 8)
if ((workers != 1 && workers != 4 && workers != 8)); then
  worker_counts+=("$workers")
fi
query='[.features[].properties.release] | sort'

mkdir -p "$output_dir"

printf '{"features":[{"properties":{"release":2},"geometry":[1]},{"properties":{"release":1}}]}' |
  RAYON_NUM_THREADS="$workers" "$tq" -i json -o json -c --explain-json "$query" \
    > /dev/null 2> "$output_dir/explain.json"

plan=$("$jq_bin" -r '.execution.plan' "$output_dir/explain.json")
preparation=$("$jq_bin" -r '.execution.hybrid_proof.preparation // "none"' "$output_dir/explain.json")
parallel=$("$jq_bin" -r '.execution.parallel_selected_decode.eligible' "$output_dir/explain.json")
if [[ $plan != hybrid-streaming-blocking || $preparation != stable-sort-runs || $parallel != true ]]; then
  echo "refusing benchmark: plan=$plan preparation=$preparation parallel=$parallel" >&2
  exit 2
fi

run_correctness() {
  local threads=$1
  RAYON_NUM_THREADS="$threads" "$tq" -i json -o json -c \
    --max-vm-steps 1000000000 --report-file "$output_dir/workers-$threads.report.json" \
    "$query" "$input" > "$output_dir/workers-$threads.correctness"
}

"$jq_bin" -c "$query" "$input" > "$output_dir/jq.correctness"
for threads in "${worker_counts[@]}"; do
  run_correctness "$threads"
  cmp "$output_dir/jq.correctness" "$output_dir/workers-$threads.correctness"
done

jq_correctness_sha=$(shasum -a 256 "$output_dir/jq.correctness" | awk '{print $1}')
input_sha=$(shasum -a 256 "$input" | awk '{print $1}')
tq_sha=$(shasum -a 256 "$tq" | awk '{print $1}')
jq_sha=$(shasum -a 256 "$jq_bin" | awk '{print $1}')
input_bytes=$(stat -f %z "$input")

run_mode() {
  local threads=$1
  env RAYON_NUM_THREADS="$threads" "$tq" -i json -o json -c \
    --max-vm-steps 1000000000 "$query" "$input" > /dev/null
}

mode_name() {
  local threads=$1
  if ((threads == 1)); then
    echo single
  elif ((threads == workers)); then
    echo multi
  else
    echo "parallel-$threads"
  fi
}

for ((warmup = 1; warmup <= warmups; warmup++)); do
  echo "warmup $warmup/$warmups" >&2
  "$jq_bin" -c "$query" "$input" > /dev/null
  for threads in "${worker_counts[@]}"; do
    run_mode "$threads"
  done
done

echo 'tool,workers,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes,output_sha256' \
  > "$output_dir/samples.csv"

measure() {
  local tool=$1 threads=$2 run=$3
  local timing=$output_dir/$tool-$run.time
  echo "measure $run/$runs $tool" >&2
  shift 3
  local status=0
  /usr/bin/time -lp env RAYON_NUM_THREADS="$threads" "$@" \
    > /dev/null 2> "$timing" || status=$?
  if ((status != 0)) && {
    ! grep -q '^real ' "$timing" || ! grep -q 'Operation not permitted' "$timing"
  }; then
    return "$status"
  fi
  awk -v tool="$tool" -v workers="$threads" -v run="$run" -v output_sha="$jq_correctness_sha" '
    /^real / { wall=$2 }
    /^user / { user=$2 }
    /^sys / { sys_time=$2 }
    /maximum resident set size/ { rss=$1 }
    END { printf "%s,%s,%s,%.2f,%.2f,%.2f,%.2f,%s,%s\n", tool, workers, run, wall, user, sys_time, user+sys_time, rss, output_sha }
  ' "$timing" >> "$output_dir/samples.csv"
}

for ((run = 1; run <= runs; run++)); do
  if ((run % 2 == 1)); then
    measure jq 1 "$run" "$jq_bin" -c "$query" "$input"
    for threads in "${worker_counts[@]}"; do
      measure "$(mode_name "$threads")" "$threads" "$run" "$tq" \
        -i json -o json -c --max-vm-steps 1000000000 "$query" "$input"
    done
  else
    for ((index = ${#worker_counts[@]} - 1; index >= 0; index--)); do
      threads=${worker_counts[index]}
      measure "$(mode_name "$threads")" "$threads" "$run" "$tq" \
        -i json -o json -c --max-vm-steps 1000000000 "$query" "$input"
    done
    measure jq 1 "$run" "$jq_bin" -c "$query" "$input"
  fi
done

{
  echo "input=$input"
  echo "input_bytes=$input_bytes"
  echo "input_sha256=$input_sha"
  echo "correctness_sha256=$jq_correctness_sha"
  echo "jq_correctness_sha256=$jq_correctness_sha"
  echo "query=$query"
  echo "workers=$workers"
  echo "worker_counts=${worker_counts[*]}"
  echo "warmups=$warmups"
  echo "runs=$runs"
  echo "tq=$tq"
  echo "tq_sha256=$tq_sha"
  echo "jq=$jq_bin"
  echo "jq_sha256=$jq_sha"
} > "$output_dir/metadata.txt"

echo "$output_dir"
