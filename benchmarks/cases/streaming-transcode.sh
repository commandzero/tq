#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: $0 INPUT_DIR TQ_BIN OUTPUT_DIR" >&2
  exit 2
fi

input_dir=$1
tq_bin=$2
output_dir=$3
runs=${RUNS:-7}
mkdir -p "$output_dir/raw" "$output_dir/reports" "$output_dir/validation"

synthetic_cases=(wide-object nested-object root-array nested-array scalar-array tabular-array)
natural_cases=(segments recovery)
if [[ -n ${CASES:-} ]]; then
  read -r -a cases <<< "$CASES"
else
  cases=("${synthetic_cases[@]}" "${natural_cases[@]}")
fi
results="$output_dir/results.csv"
validation="$output_dir/validation.csv"

printf 'case,plan,run,wall_seconds,user_seconds,system_seconds,cpu_seconds,peak_rss_bytes,output_bytes,first_byte_seconds,first_payload_seconds,preparation_high_water_bytes,object_index_spills,array_preparations,spool_bytes_written,spool_bytes_replayed,resource_outcome\n' > "$results"
printf 'case,transcode_sha256,document_sha256,status\n' > "$validation"

publication_latency() {
  local input=$1
  local force=$2
  FORCE_DOCUMENT="$force" TQ_BIN="$tq_bin" INPUT_FILE="$input" perl -MTime::HiRes=clock_gettime,CLOCK_MONOTONIC -e '
    my $start = clock_gettime(CLOCK_MONOTONIC);
    my @command = ($ENV{TQ_BIN}, "-i", "json", "-o", "toon", ".", $ENV{INPUT_FILE});
    local $ENV{TQ_BENCH_FORCE_DOCUMENT} = "1" if $ENV{FORCE_DOCUMENT} eq "1";
    open(my $stream, "-|", @command) or die "could not start tq: $!";
    my $byte;
    my $count = read($stream, $byte, 1);
    die "tq produced no first byte" unless $count;
    my $first = clock_gettime(CLOCK_MONOTONIC) - $start;
    $count = read($stream, $byte, 1);
    die "tq produced no payload byte" unless $count;
    my $payload = clock_gettime(CLOCK_MONOTONIC) - $start;
    printf "%.9f,%.9f\n", $first, $payload;
    while (read($stream, my $discard, 65536)) {}
    close($stream) or die "tq failed";
  '
}

measure() {
  local case_name=$1
  local plan=$2
  local run=$3
  local input="$input_dir/$case_name.json"
  local timing="$output_dir/raw/$case_name-$plan-$run.time"
  local report="$output_dir/reports/$case_name-$plan-$run.json"
  local output="$output_dir/raw/$case_name-$plan-$run.toon"
  local force=0
  [[ $plan == document ]] && force=1

  if [[ $force == 1 ]]; then
    { TQ_BENCH_FORCE_DOCUMENT=1 /usr/bin/time -l "$tq_bin" -i json -o toon --report-file "$report" '.' "$input" > "$output"; } 2> "$timing"
  else
    { /usr/bin/time -l "$tq_bin" -i json -o toon --report-file "$report" '.' "$input" > "$output"; } 2> "$timing"
  fi

  local wall user sys rss bytes first payload prep spills arrays written replayed outcome
  wall=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "real") { print $(i-1); exit } }' "$timing")
  user=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "user") { print $(i-1); exit } }' "$timing")
  sys=$(awk '{ for (i = 1; i <= NF; i++) if ($i == "sys") { print $(i-1); exit } }' "$timing")
  rss=$(awk '$2 == "maximum" && $3 == "resident" { print $1; exit }' "$timing")
  bytes=$(stat -f '%z' "$output")
  IFS=, read -r first payload <<< "$(publication_latency "$input" "$force")"
  prep=$(jq -r '.execution.preparation_high_water_bytes // 0' "$report")
  spills=$(jq -r '.execution.object_index_spills // 0' "$report")
  arrays=$(jq -r '.execution.array_preparations // 0' "$report")
  written=$(jq -r '.execution.spool_bytes_written // 0' "$report")
  replayed=$(jq -r '.execution.spool_bytes_replayed // 0' "$report")
  outcome=$(jq -r '.execution.resource_outcome // "success"' "$report")
  awk -v c="$case_name" -v p="$plan" -v r="$run" -v w="$wall" -v u="$user" \
    -v s="$sys" -v m="$rss" -v b="$bytes" -v f="$first" -v fp="$payload" -v h="$prep" \
    -v i="$spills" -v a="$arrays" -v sw="$written" -v sr="$replayed" -v o="$outcome" \
    'BEGIN { printf "%s,%s,%d,%.6f,%.6f,%.6f,%.6f,%s,%s,%.9f,%.9f,%s,%s,%s,%s,%s,%s\n", c,p,r,w,u,s,u+s,m,b,f,fp,h,i,a,sw,sr,o }' \
    >> "$results"
}

for case_name in "${cases[@]}"; do
  input="$input_dir/$case_name.json"
  if [[ ! -f $input ]]; then
    if [[ " ${natural_cases[*]} " == *" $case_name "* ]]; then
      echo "missing accepted natural input: $case_name.json" >&2
      continue
    fi
    echo "missing synthetic input: $case_name.json" >&2
    exit 1
  fi

  transcode="$output_dir/validation/$case_name-transcode.toon"
  document="$output_dir/validation/$case_name-document.toon"
  "$tq_bin" -i json -o toon '.' "$input" > "$transcode"
  TQ_BENCH_FORCE_DOCUMENT=1 "$tq_bin" -i json -o toon '.' "$input" > "$document"
  transcode_sha=$(shasum -a 256 "$transcode" | awk '{ print $1 }')
  document_sha=$(shasum -a 256 "$document" | awk '{ print $1 }')
  status=pass
  [[ $transcode_sha == "$document_sha" ]] || status=byte-mismatch
  printf '%s,%s,%s,%s\n' "$case_name" "$transcode_sha" "$document_sha" "$status" >> "$validation"
  [[ $status == pass ]] || continue

  measure "$case_name" transcode 0
  measure "$case_name" document 0
  for ((run = 1; run <= runs; run++)); do
    if ((run % 2 == 0)); then
      measure "$case_name" document "$run"
      measure "$case_name" transcode "$run"
    else
      measure "$case_name" transcode "$run"
      measure "$case_name" document "$run"
    fi
  done
done

echo "wrote $results"
