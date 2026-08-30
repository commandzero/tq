#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 4 ]]; then
  echo "usage: $0 INPUT_DIR BASELINE_TQ CANDIDATE_TQ OUTPUT_DIR" >&2
  exit 2
fi

input_dir=$1
baseline=$2
candidate=$3
output_dir=$4
runs=${RUNS:-7}
read -r -a extra_args <<< "${EXTRA_ARGS:-}"
if [[ -n ${CASES:-} ]]; then
  read -r -a cases <<< "$CASES"
else
  cases=(wide-object nested-object root-array nested-array scalar-array tabular-array)
fi

mkdir -p "$output_dir/raw" "$output_dir/validation"
results="$output_dir/results.csv"
validation="$output_dir/validation.csv"
printf 'case,binary,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes\n' > "$results"
printf 'case,input_bytes,input_sha256,output_bytes,baseline_sha256,candidate_sha256,status\n' > "$validation"

measure() {
  local case_name=$1
  local label=$2
  local binary=$3
  local run=$4
  local timing="$output_dir/raw/$case_name-$label-$run.time"

  { /usr/bin/time -l "$binary" -i json -o toon "${extra_args[@]}" . "$input_dir/$case_name.json" > /dev/null; } 2> "$timing"
  local wall user sys rss
  wall=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "real") { print $(i-1); exit } }' "$timing")
  user=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "user") { print $(i-1); exit } }' "$timing")
  sys=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "sys") { print $(i-1); exit } }' "$timing")
  rss=$(awk '$2 == "maximum" && $3 == "resident" { print $1; exit }' "$timing")
  awk -v c="$case_name" -v b="$label" -v r="$run" -v w="$wall" -v u="$user" -v s="$sys" -v m="$rss" \
    'BEGIN { printf "%s,%s,%d,%.6f,%.6f,%.6f,%.6f,%s\n", c,b,r,w,u,s,u+s,m }' >> "$results"
}

for case_name in "${cases[@]}"; do
  input="$input_dir/$case_name.json"
  baseline_output="$output_dir/validation/$case_name-baseline.toon"
  candidate_output="$output_dir/validation/$case_name-candidate.toon"
  "$baseline" -i json -o toon "${extra_args[@]}" . "$input" > "$baseline_output"
  "$candidate" -i json -o toon "${extra_args[@]}" . "$input" > "$candidate_output"
  input_bytes=$(stat -f '%z' "$input")
  input_sha=$(shasum -a 256 "$input" | awk '{ print $1 }')
  output_bytes=$(stat -f '%z' "$candidate_output")
  baseline_sha=$(shasum -a 256 "$baseline_output" | awk '{ print $1 }')
  candidate_sha=$(shasum -a 256 "$candidate_output" | awk '{ print $1 }')
  status=pass
  [[ $baseline_sha == "$candidate_sha" ]] || status=byte-mismatch
  printf '%s,%s,%s,%s,%s,%s,%s\n' "$case_name" "$input_bytes" "$input_sha" "$output_bytes" "$baseline_sha" "$candidate_sha" "$status" >> "$validation"
  [[ $status == pass ]] || continue

  measure "$case_name" baseline "$baseline" 0
  measure "$case_name" candidate "$candidate" 0
  for ((run = 1; run <= runs; run++)); do
    if ((run % 2 == 0)); then
      measure "$case_name" candidate "$candidate" "$run"
      measure "$case_name" baseline "$baseline" "$run"
    else
      measure "$case_name" baseline "$baseline" "$run"
      measure "$case_name" candidate "$candidate" "$run"
    fi
  done
done

echo "wrote $results"
