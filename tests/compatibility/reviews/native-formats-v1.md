# Native format compatibility review

Reviewed 2026-09-05 against jq 1.8.1 and Mike Farah yq 4.53.2.
The focused campaign contains 33 cases from `native-formats.jsonl` and
`native-delimited.jsonl`. All executed processes exited normally. The report
records executable hashes, arguments, stdout, stderr, results, and exit codes.

Run with `tq-compat run --profile full --case-prefix native. --json REPORT`.
Set `TQ_JQ`, `TQ_YQ`, and `TQ_BIN` to the reviewed executables. The prefix
selects cases before execution and fails if no case matches.

## jq agreement

All 23 jq/tq cases agree on their declared semantic or exact-byte contract,
exit code, and error class. These cover ordinary input cursors, null input,
EOF handling, stream projection, sequence roots, parse recovery, slurp,
query-consumed failures, caught failures, partial event publication, stream
error values, compact and pretty output, empty output, and exact numbers.
The final replay also checks consecutive malformed tokens after a partial
container. The first error retains the failed value path; the next uses the
reset root path, matching jq.

Warnings remain present in both tools for top-level recovery. tq adds source
and recovery-segment context; executable names and diagnostic wrappers are
CLI adaptations, not byte-for-byte stderr promises. Query-caught error strings
match jq in the checked cases. `--stream-errors` suppresses warnings in both.

## Delimited profile observations

Each yq difference below is deliberate. yq is a profile peer, not the definition
of tq's native delimited format.

| Cases | Reviewed observation |
| --- | --- |
| `native.csv-header-rows`, `native.csv-quoted-newline` | Common agreement on row objects, numbers, embedded newlines, and escaped quotes. |
| `native.csv-quoted-scalars`, `native.tsv-quoted-scalars` | tq preserves quoted numeric-looking fields as strings. Quoted empty CSV fields are empty strings, unquoted empty fields are null, and the unquoted literal `null` is a string. yq infers types without retaining quote provenance. |
| `native.csv-missing-fields` | tq fills missing trailing fields with null. yq rejects the row width. |
| `native.csv-excess-fields` | Both reject excess fields. tq uses input-parse exit 5; yq uses exit 1. |
| `native.csv-duplicate-header` | tq rejects duplicate keys. yq emits duplicate object keys, normalized to the last value in the report. |
| `native.csv-output`, `native.tsv-output` | tq quotes numeric-looking and empty strings, writes null as an empty field, and fills missing fields. The raw-byte differences preserve tq's declared profile. |
| `native.strict-missing-key` | tq-only extension. Strict conversion rejects a missing key before the next row commits, retaining the previously written header and row. |

No jq-target divergence is accepted by this review. HCL and Lua remain deferred;
XML, TOML, Properties, and INI remain follow-on native formats.

Full local evidence is retained in the benchmark workspace at
`.work/native-format-compatibility-20260905-final.json`. The earlier checkpoint
remains beside it. The versioned baseline stores
the reviewed stable observations without timings.
