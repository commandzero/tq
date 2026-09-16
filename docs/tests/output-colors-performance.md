---
type: Report
title: Output color performance
description: Regenerable monochrome and colored output test results.
generated: { by: codex, at: 2026-09-13T22:04:09Z }
---

# Output color performance

This test compares the same tq binary with monochrome and forced color.
It covers JSON document output, a multi-result JSON projection, and TOON
identity transcoding with value and sequence framing.

Each colored result must recover the exact plain bytes after stripping
generated SGR before timing begins. Color is presentation, not a data-format
change. Automatic redirected output stays plain.

The correctness gate matches the exact monochrome capture while allowing only
additional complete SGR sequences. Literal ESC bytes and literal SGR content
must remain present; their presence alone is not evidence of generated color.

## Results

<!-- benchmark-results:start -->
Generated from correctness-gated color reports. Lower time and RSS are better.

### Week

Input: 1580876 bytes. Warmups: 2. Measured samples per row: 5. RSS collector: `darwin-wait4`.

Host: Apple M4 Pro / macOS 26.6.2 build 25G83 / arm64; kernel: Darwin 25.6.0; logical CPUs: 14; RAM bytes: 51539607552; build: release, thin LTO, 1 codegen unit. Recorded: 2026-09-13.

| Case | Plain ms | Color ms | Time ratio | Plain RSS MiB | Color RSS MiB | Plain bytes | Color bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| document-json-identity | 37.2 | 69.7 | 1.88× | 20.3 | 25.2 | 2419795 | 8050332 |
| json-projection | 80.5 | 84.4 | 1.05× | 6.8 | 6.8 | 185194 | 748119 |
| identity-transcode | 94.4 | 124.2 | 1.32× | 8.0 | 11.2 | 1872399 | 4737316 |
| sequence-transcode | 86.5 | 117.2 | 1.35× | 8.0 | 11.3 | 1872400 | 4737317 |

### Month

Input: 8013185 bytes. Warmups: 2. Measured samples per row: 5. RSS collector: `darwin-wait4`.

Host: Apple M4 Pro / macOS 26.6.2 build 25G83 / arm64; kernel: Darwin 25.6.0; logical CPUs: 14; RAM bytes: 51539607552; build: release, thin LTO, 1 codegen unit. Recorded: 2026-09-13.

| Case | Plain ms | Color ms | Time ratio | Plain RSS MiB | Color RSS MiB | Plain bytes | Color bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| document-json-identity | 154.1 | 373.8 | 2.43× | 77.3 | 103.9 | 12263577 | 40789404 |
| json-projection | 440.9 | 431.5 | 0.98× | 11.8 | 11.9 | 937004 | 3789326 |
| identity-transcode | 1199.5 | 4156.4 | 3.46× | 14.5 | 14.9 | 9490993 | 24012808 |
| sequence-transcode | 1008.2 | 3828.8 | 3.80× | 14.6 | 15.1 | 9490994 | 24012809 |
<!-- benchmark-results:end -->

## Regenerate

Build the checked-in test command and the native measurement worker:

```console
cargo build --release -p tq-cli -p tq-test-support \
  --bin tq --bin tq-bench-worker --bin tq-color-bench
```

Run outside restricted sandboxes with native child-accounting permissions.
Use cached, unmodified USGS week and month JSON sources, and a new archive
directory in the separate benchmark checkout:

```console
env -u JQ_COLORS -u NO_COLOR \
  target/release/tq-color-bench run \
  target/release/tq "$WEEK_JSON" "$MONTH_JSON" "$COLOR_RUN_DIR" \
  docs/tests/output-colors-performance.md
```

The command checks native RSS availability, runs the plain/color correctness
gates, performs 2 warmups and 5 measured samples per row, saves both raw reports,
and updates this page only after the complete run passes. Failed or incomplete
reports do not replace the published results. Existing raw reports are not
overwritten.

Rebuild the page without rerunning measurements:

```console
cargo run -p tq-test-support --bin tq-color-bench -- \
  --render-only docs/tests/output-colors-performance.md \
  "$COLOR_RUN_DIR/week.json" "$COLOR_RUN_DIR/month.json"
```

The renderer replaces only the marked Results block, following the shared
benchmark report convention. It preserves the authored introduction and method.
Synthetic regression tests check repeatability, changed measurements, invalid
reports, and preservation of authored text.

## Method and interpretation

Each invocation uses strict JSON input by filename. Identity uses `.`;
projection uses:

```jq
.features[] | {id, mag: .properties.mag, place: .properties.place}
```

The native isolated worker measures spawn-to-exit wall time and process peak
RSS while capturing stdout. There is no recurring RSS sampler. Invocations
have a 120-second timeout and a 512 MiB capture limit. Cases run sequentially,
plain before color. Tables show medians, not performance guarantees.

New raw reports record the measured host, release build profile, input and
binary hashes, native worker/collector identity, and `color_present` from the
correctness capture. Render-only requires this evidence to be true for colored
rows and false for monochrome rows. Older reports without color-presence
evidence must be remeasured; do not infer it from the requested flag. The tables
above retain the historical measurements and are not a new run of this gate.
Absent optional host metadata is identified rather than inferred from the
machine rendering this page. Detailed provenance and historical analysis stay
in the separate benchmark archive, not in this generated test page.

JSON retains its existing document serialization buffer, so decorated bytes
can increase its RSS. TOON reuses bounded preparation and spool accounting.
These are same-binary color-overhead tests, not comparisons with a parent
commit, jq, or yq.

The initial measured overhead was accepted for terminal use. The all-month
corpus exceeds typical terminal-friendly output sizes and remains a useful
stress case, not a universal color-overhead limit.

See the [compatibility review](../../tests/compatibility/reviews/output-colors.md)
for the separately reviewed raw/proxy, palette, and framing behavior.
