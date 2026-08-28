#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "usage: $0 INPUT_DIR TQ_BIN TOON_BIN TOON_STREAM_BIN OUTPUT_DIR" >&2
  exit 2
fi

input_dir=$1
tq_bin=$2
toon_bin=$3
toon_stream_bin=$4
output_dir=$5
runs=${RUNS:-7}

mkdir -p "$output_dir/raw" "$output_dir/validation"
results="$output_dir/results.csv"
inputs="$output_dir/inputs.csv"
validation="$output_dir/validation.csv"

cases=(indices mapping nodes_stats segments recovery snapshot)

printf 'case,input_bytes,input_sha256\n' > "$inputs"
printf 'case,tool,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes\n' > "$results"
printf 'case,tool,output_bytes,source_semantic_sha256,self_decoded_semantic_sha256,self_status,cross_status\n' > "$validation"

measure() {
  local case_name=$1
  local tool=$2
  local run=$3
  local input="$input_dir/$case_name.json"
  local timing="$output_dir/raw/${case_name}-${tool}-${run}.time"
  local real user sys rss

  case $tool in
    tq)
      { /usr/bin/time -l "$tq_bin" -i json -o toon --unframed '.' "$input" > /dev/null; } 2> "$timing"
      ;;
    toon)
      { /usr/bin/time -l "$toon_bin" "$input" -o /dev/null > /dev/null; } 2> "$timing"
      ;;
    toon_stream)
      { /usr/bin/time -l "$toon_stream_bin" "$input" -o /dev/null > /dev/null; } 2> "$timing"
      ;;
  esac

  real=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "real") { print $(i-1); exit } }' "$timing")
  user=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "user") { print $(i-1); exit } }' "$timing")
  sys=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "sys") { print $(i-1); exit } }' "$timing")
  rss=$(awk '$2 == "maximum" && $3 == "resident" { print $1; exit }' "$timing")
  awk -v c="$case_name" -v t="$tool" -v r="$run" -v w="$real" -v u="$user" -v s="$sys" -v m="$rss" \
    'BEGIN { printf "%s,%s,%d,%.6f,%.6f,%.6f,%.6f,%s\n", c, t, r, w, u, s, u+s, m }' >> "$results"
}

semantic_hash() {
  jq -S -c . | shasum -a 256 | awk '{ print $1 }'
}

for case_name in "${cases[@]}"; do
  input="$input_dir/$case_name.json"
  if [[ ! -f $input ]]; then
    echo "missing input: $case_name.json" >&2
    exit 1
  fi

  input_bytes=$(stat -f '%z' "$input")
  input_sha=$(shasum -a 256 "$input" | awk '{ print $1 }')
  printf '%s,%s,%s\n' "$case_name" "$input_bytes" "$input_sha" >> "$inputs"

  source_hash=$(jq -S -c . "$input" | semantic_hash)

  tq_output="$output_dir/validation/$case_name.tq.toon"
  "$tq_bin" -i json -o toon --unframed '.' "$input" > "$tq_output"
  tq_status=pass
  if ! tq_decoded_hash=$("$tq_bin" -i toon -o json -c '.' "$tq_output" 2> "$output_dir/raw/$case_name-tq-self-decode.err" | semantic_hash); then
    tq_decoded_hash=error
    tq_status=decode-error
  elif [[ $source_hash != "$tq_decoded_hash" ]]; then
    tq_status=semantic-fail
  fi
  tq_cross=pass
  if ! tq_cross_hash=$("$toon_bin" "$tq_output" --decode 2> "$output_dir/raw/$case_name-tq-cross-decode.err" | semantic_hash); then
    tq_cross=decode-error
  elif [[ $source_hash != "$tq_cross_hash" ]]; then
    tq_cross=semantic-fail
  fi
  printf '%s,tq,%s,%s,%s,%s,%s\n' "$case_name" "$(stat -f '%z' "$tq_output")" "$source_hash" "$tq_decoded_hash" "$tq_status" "$tq_cross" >> "$validation"

  toon_output="$output_dir/validation/$case_name.toon.toon"
  "$toon_bin" "$input" -o "$toon_output"
  toon_status=pass
  if ! toon_decoded_hash=$("$toon_bin" "$toon_output" --decode 2> "$output_dir/raw/$case_name-toon-self-decode.err" | semantic_hash); then
    toon_decoded_hash=error
    toon_status=decode-error
  elif [[ $source_hash != "$toon_decoded_hash" ]]; then
    toon_status=semantic-fail
  fi
  toon_cross=pass
  if ! toon_cross_hash=$("$tq_bin" -i toon -o json -c '.' "$toon_output" 2> "$output_dir/raw/$case_name-toon-cross-decode.err" | semantic_hash); then
    toon_cross=decode-error
  elif [[ $source_hash != "$toon_cross_hash" ]]; then
    toon_cross=semantic-fail
  fi
  printf '%s,toon,%s,%s,%s,%s,%s\n' "$case_name" "$(stat -f '%z' "$toon_output")" "$source_hash" "$toon_decoded_hash" "$toon_status" "$toon_cross" >> "$validation"

  toon_stream_output="$output_dir/validation/$case_name.toon-stream.toon"
  "$toon_stream_bin" "$input" -o "$toon_stream_output"
  toon_stream_status=pass
  if ! toon_stream_decoded_hash=$("$toon_stream_bin" "$toon_stream_output" --decode 2> "$output_dir/raw/$case_name-toon-stream-self-decode.err" | semantic_hash); then
    toon_stream_decoded_hash=error
    toon_stream_status=decode-error
  elif [[ $source_hash != "$toon_stream_decoded_hash" ]]; then
    toon_stream_status=semantic-fail
  fi
  toon_stream_cross=pass
  if ! toon_stream_cross_hash=$("$tq_bin" -i toon -o json -c '.' "$toon_stream_output" 2> "$output_dir/raw/$case_name-toon-stream-cross-decode.err" | semantic_hash); then
    toon_stream_cross=decode-error
  elif [[ $source_hash != "$toon_stream_cross_hash" ]]; then
    toon_stream_cross=semantic-fail
  fi
  printf '%s,toon_stream,%s,%s,%s,%s,%s\n' "$case_name" "$(stat -f '%z' "$toon_stream_output")" "$source_hash" "$toon_stream_decoded_hash" "$toon_stream_status" "$toon_stream_cross" >> "$validation"

  # Use tq's TOON decoder as the common oracle for both encoders. Self-decoder
  # results remain in validation.csv because they expose decoder differences.
  if [[ $tq_status != pass || $toon_cross != pass || $toon_stream_cross != pass ]]; then
    echo "common-decoder validation failed for $case_name; skipping timed runs" >&2
    continue
  fi

  measure "$case_name" tq 0
  measure "$case_name" toon 0
  measure "$case_name" toon_stream 0

  for ((run = 1; run <= runs; run++)); do
    case $((run % 3)) in
      1)
        measure "$case_name" toon "$run"
        measure "$case_name" tq "$run"
        measure "$case_name" toon_stream "$run"
        ;;
      2)
        measure "$case_name" tq "$run"
        measure "$case_name" toon_stream "$run"
        measure "$case_name" toon "$run"
        ;;
      0)
        measure "$case_name" toon_stream "$run"
        measure "$case_name" toon "$run"
        measure "$case_name" tq "$run"
        ;;
    esac
  done
done

echo "wrote $results"
