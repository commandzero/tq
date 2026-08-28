#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 OUTPUT_DIR [ITEMS]" >&2
  exit 2
fi

output_dir=$1
items=${2:-100000}
mkdir -p "$output_dir"

jq -nc --argjson n "$items" \
  'reduce range(0; $n) as $i ({}; .["key_\($i)"] = {id:$i,name:"item-\($i)"})' \
  > "$output_dir/wide-object.json"
jq -nc --argjson n "$items" \
  '{outer:(reduce range(0; $n) as $i ({}; .["key_\($i)"] = [$i, ($i + 1)]))}' \
  > "$output_dir/nested-object.json"
jq -nc --argjson n "$items" '[range(0; $n) | {id:., mixed:(. % 2 == 0)}]' \
  > "$output_dir/root-array.json"
jq -nc --argjson n "$items" '{metadata:{kind:"nested"},items:[range(0; $n)]}' \
  > "$output_dir/nested-array.json"
jq -nc --argjson n "$items" '[range(0; $n)]' \
  > "$output_dir/scalar-array.json"
jq -nc --argjson n "$items" '[range(0; $n) | {id:.,name:"item-\(.)"}]' \
  > "$output_dir/tabular-array.json"

echo "wrote streaming-transcode fixtures to $output_dir"
