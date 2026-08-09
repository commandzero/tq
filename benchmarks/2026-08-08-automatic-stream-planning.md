# Automatic stream planning release evidence — 2026-08-08

This artifact records the compatibility and natural-large checks for the
`automatic-stream-planning` change. Commands used the release `tq` binary from
this worktree and the frozen Microsoft US Buildings Georgia GeoJSON snapshot
(`1,119,571,788` bytes). Full machine-readable execution reports were emitted
with `--report-file`; the facts below are the reviewed release summary.

## Compatibility

`make compatibility-full` completed all 158 cases. The campaign retained its
existing `observed-differences` disposition across jq, yq, and tq. The jq/tq
result sequences for the automatic-planning candidate shapes—including
projection, selection, and nested iteration—were identical.

## Natural-large automatic plans

Both commands completed inside the campaign's 600-second timeout. RSS was
sampled from the isolated live process after stale harness children were
removed; both were far below the 128 MiB bounded-stream envelope.

| Query | Plan | Results | Elapsed | Throughput | Observed RSS | Retained high-water |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `.features[].properties.mag` | subtree with static projection | 3,981,792 | 540.01 s | 1.98 MiB/s | ~2.3 MiB | 0 bytes; depth 0 |
| `.features[] \| select(.properties.mag >= 2) \| .id` | subtree | 0 | 451.65 s | 2.36 MiB/s | ~2.7 MiB | 16,705 bytes; depth 5 |

Every source child was accounted for (`3,981,792` completed subtrees in each
report). The projection returned `null` for every missing magnitude; the
selection returned no values, matching jq on this corpus. The results confirm
bounded retention while documenting that decoder/event CPU throughput remains
an optimization opportunity.
