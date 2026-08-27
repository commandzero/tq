# Automatic stream planning release evidence, 2026-08-08, verified 2026-08-09

This report records the compatibility and natural-large checks for the
`automatic-stream-planning` change. Commands used the release `tq` binary from
this worktree and the frozen Microsoft US Buildings Georgia GeoJSON snapshot
(`1,119,571,788` bytes). Full machine-readable execution reports were emitted
with `--report-file`; the facts below are the reviewed release summary.

## Compatibility

`make compatibility-full` completed all 158 cases. The campaign retained its
existing `observed-differences` disposition across jq, yq, and tq. The jq/tq
result sequences for the candidate projection, selection, and nested iteration
shapes were identical.

## Natural-large automatic plans

Both commands completed inside the campaign's 600-second timeout. The
projection RSS was sampled from its isolated live process. The selective query
was repeated on 2026-08-09 through default auto-format detection under
`/usr/bin/time -l`, which records the completed process's maximum resident set
size. The source manifest SHA-256 was
`e33434c71f0ed0316e54a56983e1855f57f4a69932a71ca2b4131c62f6f2075d` and
the source JSON SHA-256 was
`2e27cf6160636a5981d3b4f8a8c2488420df3a6611c09ed5305643f107ebf1d6`.

| Query | Plan | Results | Elapsed | Throughput | RSS evidence | Retained high-water |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `.features[].properties.mag` | subtree with static projection | 3,981,792 | 540.01 s | 1.98 MiB/s | ~2.3 MiB | 0 bytes; depth 0 |
| `.features[] \| select(.properties.mag >= 2) \| .id` | auto-detected JSON, subtree | 0 | 421.95 s | 2.53 MiB/s | 4,014,080-byte peak (3.83 MiB) | 16,705 bytes; depth 5 |

The peak-RSS command omitted `--input-format` to test automatic detection:

```console
/usr/bin/time -l target/release/tq --output-format json -c \
  --report-file /tmp/automatic-stream-auto-large.json \
  '.features[] | select(.properties.mag >= 2) | .id' SOURCE.geojson >/dev/null
```

The report selected `subtree`, recorded `3,981,792` completed subtrees, and had
no stream rejection. The measured 4,014,080-byte peak is 2.99% of the
134,217,728-byte manifest-aware gate, leaving 130,203,648 bytes of headroom.

Every source child was accounted for (`3,981,792` completed subtrees in each
report). The projection returned `null` for every missing magnitude; the
selection returned no values, matching jq on this corpus. The results confirm
bounded retention while documenting that decoder/event CPU throughput remains
an optimization opportunity.
