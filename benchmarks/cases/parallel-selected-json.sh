#!/usr/bin/env bash
set -euo pipefail

input=${1:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR] [JQ_BIN]}
tq=${2:?usage: parallel-selected-json.sh INPUT TQ [OUTPUT_DIR] [JQ_BIN]}
archive_root=${TQ_BENCHMARK_ARCHIVE_ROOT:-$HOME/Development/commandzero/tq-benchmarks}
sampling=${SAMPLING:-extended}
if [[ -n ${3:-} ]]; then
  output_dir=$3
elif [[ $sampling == quick ]]; then
  output_dir=$(mktemp -d "${TMPDIR:-/tmp}/tq-parallel-selected-quick.XXXXXX")
else
  output_dir="$archive_root/.work/parallel-selected-json/$(date +%Y-%m-%d)"
fi
jq_bin=${4:-${TQ_JQ:-jq}}
resolved_jq=$(command -v "$jq_bin") || {
  printf 'Cannot find jq executable: %s\n' "$jq_bin" >&2
  exit 69
}
jq_bin=$resolved_jq
bench=${TQ_BENCH:-$(dirname "$tq")/tq-bench}

if [[ ! -x $bench ]]; then
  printf 'Build tq-bench beside tq or set TQ_BENCH: %s\n' "$bench" >&2
  exit 69
fi
if [[ -n ${RUNS:-} || -n ${WARMUPS:-} ]]; then
  printf 'RUNS/WARMUPS are replaced by SAMPLING=quick|screen|compare|extended|catalog.\n' >&2
  exit 64
fi
mkdir -p "$output_dir"
export TQ_BIN="$tq" TQ_JQ="$jq_bin"
export RAYON_NUM_THREADS=${WORKERS:-1}

# One native runner owns correctness, timing, deadlines, RSS, and atomic checkpoints.
# Requested threads do not imply parallel decoding; measure the candidate's actual plan.
set -- "$bench" run --suite large-input --profile extended --input "$input" \
  --case benchmark.large-selected-sort \
  --mode "${BENCHMARK_MODE:-fast}" --sampling "$sampling" \
  --output "$output_dir/report.json"
if [[ -n ${CAMPAIGN_BUDGET_SECONDS:-} ]]; then
  set -- "$@" --campaign-budget-seconds "$CAMPAIGN_BUDGET_SECONDS"
fi
if [[ -n ${CASE_BUDGET_SECONDS:-} ]]; then
  set -- "$@" --case-budget-seconds "$CASE_BUDGET_SECONDS"
fi
exec "$@"
