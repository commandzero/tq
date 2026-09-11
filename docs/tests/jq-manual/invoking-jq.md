---
type: Report
title: "Invoking jq coverage audit"
description: "Recorded review of Invoking jq coverage audit."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# Invoking jq coverage audit

Source: jq manual, [Invoking jq](https://jqlang.org/manual/#invoking-jq). See the [machine-readable ledger](../../../tests/compatibility/reviews/jq-manual/invoking-jq.toon).

48 entries. 44 new, 4 adapted. Cases use supplied examples or documented representative fixtures.

PowerShell/cmd examples test the resulting argument vector, not shell tokenization. The binary flag test runs on this host; Windows CRLF translation remains unverified. Forced color is tested with NO_COLOR, but terminal auto-detection and flush timing require a different capture setup.

| Source line | Example | Cases | Disposition |
| ---: | --- | --- | --- |
| 16 | invoking.input-stream-prose | `manual.invoking.whitespace-stream` | new |
| 18 | invoking.identity-filter | `manual.invoking.identity` | new |
| 21 | invoking.fence-shell-fragment | `manual.invoking.stdin-default-filter` | new |
| 25 | invoking.unquoted-foo | `manual.invoking.unquoted-filter` | new |
| 26 | invoking.foo-diagnostic | `manual.invoking.unquoted-filter` | new |
| 32 | invoking.shell-unix | `manual.invoking.shell-unix` | new |
| 33 | invoking.shell-powershell | `manual.invoking.shell-unix` | adapted |
| 34 | invoking.shell-cmd | `manual.invoking.shell-unix` | adapted |
| 36 | invoking.user-function-prose | `manual.invoking.user-function` | new |
| 40 | invoking.null-input-option | `manual.invoking.null-input` | new |
| 44 | invoking.raw-input-option | `manual.invoking.raw-input` | new |
| 48 | invoking.slurp-option | `manual.invoking.slurp` | new |
| 52 | invoking.compact-output-option | `manual.invoking.compact-output` | new |
| 56 | invoking.raw-output-option | `manual.invoking.raw-output` | new |
| 60 | invoking.raw-output0-option | `manual.invoking.raw-output0` | new |
| 64 | invoking.join-output-option | `manual.invoking.join-output` | new |
| 70 | invoking.ascii-output | `manual.invoking.ascii-escape` | new |
| 72 | invoking.sort-keys-option | `manual.invoking.sort-keys` | new |
| 76 | invoking.color-option | `manual.invoking.monochrome-output`, `manual.invoking.color-output` | new |
| 78 | invoking.no-color | `manual.invoking.no-color-forced` | adapted |
| 80 | invoking.jq-colors | `manual.invoking.jq-colors` | new |
| 82 | invoking.tab-option | `manual.invoking.tab-output` | new |
| 86 | invoking.indent-option | `manual.invoking.indent-output` | new |
| 90 | invoking.unbuffered-option | `manual.invoking.unbuffered` | new |
| 96 | invoking.stream-scalar | `manual.invoking.stream-scalar` | new |
| 96 | invoking.stream-nested | `manual.invoking.stream-nested` | new |
| 98 | invoking.stream-reduce-foreach-prose | `manual.invoking.stream-reduce` | new |
| 102 | invoking.stream-errors | `manual.invoking.stream-errors` | new |
| 103 | invoking.stream-errors-fence | `manual.invoking.stream-errors` | new |
| 111 | invoking.seq-option | `manual.invoking.seq` | new |
| 115 | invoking.from-file-option | `manual.invoking.from-file` | new |
| 119 | invoking.library-path-option | `manual.invoking.library-path` | new |
| 125 | invoking.arg-string | `manual.invoking.arg-string` | new |
| 125 | invoking.arg-string-number | `manual.invoking.arg-string-number` | new |
| 127 | invoking.args-named-prose | `manual.invoking.args-named` | new |
| 131 | invoking.argjson | `manual.invoking.argjson` | new |
| 135 | invoking.slurpfile | `manual.invoking.slurpfile` | new |
| 139 | invoking.rawfile | `manual.invoking.rawfile` | new |
| 141 | invoking.args-option | `manual.invoking.args` | new |
| 145 | invoking.jsonargs-option | `manual.invoking.jsonargs` | new |
| 149 | invoking.exit-status-option | `manual.invoking.exit-false`, `manual.invoking.exit-null`, `manual.invoking.exit-empty`, `manual.invoking.exit-true` | new |
| 153 | invoking.halt-error-prose | `manual.invoking.halt-error` | new |
| 155 | invoking.binary-option | `manual.invoking.binary-portable` | adapted |
| 159 | invoking.version-option | `manual.invoking.version` | new |
| 163 | invoking.build-configuration-option | `manual.invoking.build-configuration` | new |
| 167 | invoking.help-option | `manual.invoking.help` | new |
| 171 | invoking.argument-terminator | `manual.invoking.argument-terminator` | new |
| 175 | invoking.run-tests-option | `manual.invoking.run-tests` | new |

The aggregate manual report records execution results. A mapped case is test coverage, not evidence of jq/tq parity.

<!-- tq-manual-compare:begin section=invoking-jq -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/invoking-jq.toon)


| Verdict | Cases |
| --- | ---: |
| match | 48 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 25 | 25 |
| toon | 25 | 25 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

24 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 227 | 217 | -10 | -4.41% |
| `cl100k_base` | 227 | 217 | -10 | -4.41% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### manual.invoking.arg-string

```
# input
jq --arg foo bar '$foo'

# jq
"bar"

# tq -o json
"bar"

# tq
bar
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual.invoking.arg-string-number

```
# input
jq --arg foo 123 '$foo'

# jq
"123"

# tq -o json
"123"

# tq
"123"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### manual.invoking.argjson

```
# input
jq --argjson foo 123 '$foo'

# jq
123

# tq -o json
123

# tq
123
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.args

```
# input
jq --args '$ARGS.positional[]' foo bar

# jq
"foo"
"bar"

# tq -o json
"foo"
"bar"

# tq
foo
bar
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 6 | 6 | 4 | -2 | -33.33% |

#### manual.invoking.args-named

```
# input
jq --arg foo-bar value '$ARGS.named["foo-bar"]'

# jq
"value"

# tq -o json
"value"

# tq
value
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.argument-terminator

```
# input
jq -- .

# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.ascii-escape

```
# input
jq -a .

# jq
"\u03bc"

# tq -o json
"\u03bc"

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | n/a | n/a | n/a |
| `cl100k_base` | 5 | 5 | n/a | n/a | n/a |

#### manual.invoking.binary-portable

```
# input
jq -b -c .

# jq
"line\nnext"

# tq -o json
"line\nnext"

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | n/a | n/a | n/a |
| `cl100k_base` | 5 | 5 | n/a | n/a | n/a |

#### manual.invoking.build-configuration

```
# input
jq --build-configuration

# jq
--host=arm64-apple-darwin23.6.0 --disable-docs --with-oniguruma=builtin --disable-shared --enable-static --enable-all-static 'CFLAGS=-O2 -pthread -fstack-protector-all' host_alias=arm64-apple-darwin23.6.0 'CC=clang -target arm64-apple-darwin23.6.0' LDFLAGS=-dead_strip

# tq -o json
target=macos binary-stdio=native formats=toon,yaml,json,jsonl jq-target=1.8.x

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 95 | 25 | n/a | n/a | n/a |
| `cl100k_base` | 95 | 26 | n/a | n/a | n/a |

#### manual.invoking.color-output

```
# input
jq -C .

# jq
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m2\x1b[0m
\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m\x1b[1;39m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m2\x1b[0m
\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 112 | 112 | n/a | n/a | n/a |
| `cl100k_base` | 94 | 94 | n/a | n/a | n/a |

#### manual.invoking.compact-output

```
# input
jq -c .

# jq
{"a":1,"b":[2]}

# tq -o json
{"a":1,"b":[2]}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | n/a | n/a | n/a |
| `cl100k_base` | 9 | 9 | n/a | n/a | n/a |

#### manual.invoking.exit-empty

```
# input
jq -e empty

# jq

# tq -o json

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a | n/a |

#### manual.invoking.exit-false

```
# input
jq -e .

# jq
false

# tq -o json
false

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a | n/a |

#### manual.invoking.exit-null

```
# input
jq -e .

# jq
null

# tq -o json
null

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a | n/a |

#### manual.invoking.exit-true

```
# input
jq -e .

# jq
true

# tq -o json
true

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a | n/a |

#### manual.invoking.from-file

```
# input
jq -f tests/fixtures/manual-invoking/program.jq

# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.halt-error

```
# input
jq 'halt_error(7)'

# jq


[stderr]
failure

# tq -o json


[stderr]
failure

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a | n/a |

#### manual.invoking.help

```
# input
jq --help

# jq
jq - commandline JSON processor [version 1.8.1]

Usage:	jq [options] <jq filter> [file...]
	jq [options] --args <jq filter> [strings...]
	jq [options] --jsonargs <jq filter> [JSON_TEXTS...]

jq is a tool for processing JSON inputs, applying the given filter to
its JSON text inputs and producing the filter's results as JSON on
standard output.

The simplest filter is ., which copies jq's input to its output
unmodified except for formatting. For more advanced filters see
the jq(1) manpage ("man jq") and/or https://jqlang.org/.

Example:

	$ echo '{"foo": 0}' | jq .
	{
	  "foo": 0
	}

Command options:
  -n, --null-input          use `null` as the single input value;
  -R, --raw-input           read each line as string instead of JSON;
  -s, --slurp               read all inputs into an array and use it as
                            the single input value;
  -c, --compact-output      compact instead of pretty-printed output;
  -r, --raw-output          output strings without escapes and quotes;
      --raw-output0         implies -r and output NUL after each output;
  -j, --join-output         implies -r and output without newline after
                            each output;
  -a, --ascii-output        output strings by only ASCII characters
                            using escape sequences;
  -S, --sort-keys           sort keys of each object on output;
  -C, --color-output        colorize JSON output;
  -M, --monochrome-output   disable colored output;
      --tab                 use tabs for indentation;
      --indent n            use n spaces for indentation (max 7 spaces);
      --unbuffered          flush output stream after each output;
      --stream              parse the input value in streaming fashion;
      --stream-errors       implies --stream and report parse error as
                            an array;
      --seq                 parse input/output as application/json-seq;
  -f, --from-file           load the filter from a file;
  -L, --library-path dir    search modules from the directory;
      --arg name value      set $name to the string value;
      --argjson name value  set $name to the JSON value;
      --slurpfile name file set $name to an array of JSON values read
                            from the file;
      --rawfile name file   set $name to string contents of file;
      --args                consume remaining arguments as positional
                            string values;
      --jsonargs            consume remaining arguments as positional
                            JSON values;
  -e, --exit-status         set exit status code based on the output;
  -V, --version             show the version;
  --build-configuration     show jq's build configuration;
  -h, --help                show the help;
  --                        terminates argument processing;

Named arguments are also available as $ARGS.named[], while
positional arguments are available as $ARGS.positional[].

# tq -o json
tq - jq-compatible queries over TOON, YAML, JSON, JSON5, and JSON Lines

Usage: tq [OPTIONS] [FILTER [FILE...]]
       tq [OPTIONS] -f FILE [INPUT...]
       tq compatibility

Options:
  -i, --input-format FORMAT       select auto, TOON, YAML, JSON, JSON5, JSON Lines, or TOON sequence input
  -o, --output-format FORMAT      select TOON, YAML, JSON, or JSON Lines output
  -n, --null-input                run once with null input
  -R, --raw-input                 read physical lines as strings
  -s, --slurp                     collect ordered inputs and run once
  -c, --compact-output            emit compact JSON
  -r, --raw-output                emit strings without JSON quotes
  --raw-output0                   emit NUL-separated raw outputs
  -j, --join-output               emit raw output without separators
  -a, --ascii-output              escape non-ASCII JSON output
  -S, --sort-keys                 sort object keys recursively
  -C, --color-output              force stable ANSI JSON color
  -M, --monochrome-output         disable ANSI color
  --tab                           indent JSON with tabs
  --indent N                      indent structured output
  --unbuffered                    flush after every output
  --allow-environment             permit env to inspect a redaction-safe process snapshot
  --allow-platform                permit clock, timezone, and input metadata built-ins
  --stream                        read path/value events
  --stream-errors                 report stream parse errors as values
  -x, --proxy-on-error            pass through sources rejected by structured parsing
  --seq                           use sequence framing for the selected structured output
  -f, --from-file FILE            load filter from a file
  -L, --library-path DIR          add an explicit jq module search path
  --arg NAME VALUE                bind a string variable
  --argjson NAME JSON             bind a JSON variable
  --argtoon NAME TOON             bind a TOON variable
  --slurpfile NAME FILE           bind JSON texts as an array
  --rawfile NAME FILE             bind complete UTF-8 file text
  --args                          bind remaining argv strings
  --jsonargs                      bind remaining argv JSON texts
  -e, --exit-status               derive status from the last result
  -b, --binary                    request binary-safe platform output
  -V, --version                   print version targets
  --build-configuration           print stable build capabilities
  --run-tests [FILE]              run a jq-compatible test file
  -h, --help                      print this generated help

Formats: -i, --input-format auto|toon|yaml|json|json5|jsonl|toon-seq|json-seq
-o, --output-format toon|yaml|json|jsonl, --toon-sequence-input, --unframed
TOON:    --delimiter comma|tab|pipe, --fold-keys, --flatten-depth N, --non-strict
Reports: --explain, --explain-json, --trace, --trace-limit N, --report-file FILE
Limits:  --max-input-bytes N, --max-depth N, --max-token-bytes N,
--max-line-bytes N, --max-lookahead-bytes N, --max-vm-steps N,
--max-results N, --max-output-bytes N, --prepare-memory-bytes N,
--hybrid-batch-values N, --hybrid-in-flight-batches N,
--hybrid-in-flight-bytes N, --decode-batch-values N,
--decode-batch-bytes N, --decode-in-flight-batches N,
--decode-in-flight-bytes N, --max-spool-bytes N

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 652 | 801 | n/a | n/a | n/a |
| `cl100k_base` | 652 | 804 | n/a | n/a | n/a |

#### manual.invoking.identity

```
# input
jq .

# jq
{
  "foo": "bar"
}

# tq -o json
{
  "foo": "bar"
}

# tq
foo: bar
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 4 | -5 | -55.56% |
| `cl100k_base` | 9 | 9 | 4 | -5 | -55.56% |

#### manual.invoking.indent-output

```
# input
jq -n --indent 4 '{a:{b:1}}'

# jq
{
    "a": {
        "b": 1
    }
}

# tq -o json
{
    "a": {
        "b": 1
    }
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a | n/a |

#### manual.invoking.join-output

```
# input
jq -j '.[]'

# jq
ab

# tq -o json
ab

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | n/a | n/a | n/a |
| `cl100k_base` | 1 | 1 | n/a | n/a | n/a |

#### manual.invoking.jq-colors

```
# input
jq -C .

# jq
\x1b[1;37m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m1\x1b[0m\x1b[1;37m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m2\x1b[0m
\x1b[1;37m}\x1b[0m

# tq -o json
\x1b[1;37m{\x1b[0m
  \x1b[1;34m"z"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m1\x1b[0m\x1b[1;37m,\x1b[0m
  \x1b[1;34m"a"\x1b[0m\x1b[1;37m:\x1b[0m \x1b[1;34m2\x1b[0m
\x1b[1;37m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 112 | 112 | n/a | n/a | n/a |
| `cl100k_base` | 94 | 94 | n/a | n/a | n/a |

#### manual.invoking.jsonargs

```
# input
jq --jsonargs '$ARGS.positional[]' 1 '{"a":2}'

# jq
1
{
  "a": 2
}

# tq -o json
1
{
  "a": 2
}

# tq
1
a: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 7 | -4 | -36.36% |
| `cl100k_base` | 11 | 11 | 7 | -4 | -36.36% |

#### manual.invoking.library-path

```
# input
jq -n -L tests/fixtures/modules 'include "included"; included_value'

# jq
"included"

# tq -o json
"included"

# tq
included
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual.invoking.monochrome-output

```
# input
jq -M .

# jq
{
  "z": 1,
  "a": 2
}

# tq -o json
{
  "z": 1,
  "a": 2
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a | n/a |

#### manual.invoking.no-color-forced

```
# input
jq -C .

# jq
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"foo"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m
\x1b[1;39m}\x1b[0m

# tq -o json
\x1b[1;39m{\x1b[0m
  \x1b[1;34m"foo"\x1b[0m\x1b[1;39m:\x1b[0m \x1b[0;39m1\x1b[0m
\x1b[1;39m}\x1b[0m

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 63 | 63 | n/a | n/a | n/a |
| `cl100k_base` | 53 | 53 | n/a | n/a | n/a |

#### manual.invoking.null-input

```
# input
jq -n '[1,2]'

# jq
[
  1,
  2
]

# tq -o json
[
  1,
  2
]

# tq
[2]: 1,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual.invoking.raw-input

```
# input
jq -R .

# jq
"one"
"two"

# tq -o json
"one"
"two"

# tq
one
two
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 6 | 6 | 4 | -2 | -33.33% |

#### manual.invoking.raw-output

```
# input
jq -r .

# jq
hello

# tq -o json
hello

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | n/a | n/a | n/a |
| `cl100k_base` | 2 | 2 | n/a | n/a | n/a |

#### manual.invoking.raw-output0

```
# input
jq -n --raw-output0 '"a","b"'

# jq
a\x00b\x00

# tq -o json
a\x00b\x00

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | n/a | n/a | n/a |
| `cl100k_base` | 4 | 4 | n/a | n/a | n/a |

#### manual.invoking.rawfile

```
# input
jq --rawfile foo tests/fixtures/manual-invoking/bar '$foo'

# jq
"{\"a\":1}\n\"two\"\n"

# tq -o json
"{\"a\":1}\n\"two\"\n"

# tq
"{\"a\":1}\n\"two\"\n"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 12 | 12 | 12 | 0 | +0% |
| `cl100k_base` | 12 | 12 | 12 | 0 | +0% |

#### manual.invoking.run-tests

```
# input
jq --run-tests

# jq
Test #1: '.' at line number 2
1 of 1 tests passed (0 malformed, 0 skipped)
Test jq_state: .[]
Test jq_state: .[] | if .%2 == 0 then halt_error else . end

# tq -o json
Test #1: '.' at line number 2
1 of 1 tests passed (0 malformed, 0 skipped)
Test jq_state: .[]
Test jq_state: .[] | if .%2 == 0 then halt_error else . end

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 52 | 52 | n/a | n/a | n/a |
| `cl100k_base` | 52 | 52 | n/a | n/a | n/a |

#### manual.invoking.seq

```
# input
jq --seq .

# jq --seq
\x1e1
\x1e2

# tq -o json --seq
\x1e1
\x1e2

# tq
<not run>
```

| Tokenizer | jq --seq | tq -o json --seq | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | n/a | n/a | n/a |
| `cl100k_base` | 6 | 6 | n/a | n/a | n/a |

#### manual.invoking.shell-unix

```
# input
jq '.["foo"]'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.slurp

```
# input
jq -s length

# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.slurpfile

```
# input
jq --slurpfile foo tests/fixtures/manual-invoking/bar '$foo'

# jq
[
  {
    "a": 1
  },
  "two"
]

# tq -o json
[
  {
    "a": 1
  },
  "two"
]

# tq
[2]:
  - a: 1
  - two
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 14 | -3 | -17.65% |
| `cl100k_base` | 17 | 17 | 14 | -3 | -17.65% |

#### manual.invoking.sort-keys

```
# input
jq -S .

# jq
{
  "a": 0,
  "z": {
    "a": 1,
    "b": 2
  }
}

# tq -o json
{
  "a": 0,
  "z": {
    "a": 1,
    "b": 2
  }
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | n/a | n/a | n/a |
| `cl100k_base` | 30 | 30 | n/a | n/a | n/a |

#### manual.invoking.stdin-default-filter

```
# input
jq

# jq
"foo"

# tq -o json
"foo"

# tq
foo
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual.invoking.stream-errors

```
# input
jq --stream-errors .

# jq
[
  [
    0
  ],
  "a"
]
[
  "Invalid numeric literal at line 1, column 7",
  [
    1
  ]
]

# tq -o json
[
  [
    0
  ],
  "a"
]
[
  "Invalid numeric literal at line 1, column 7",
  [
    1
  ]
]

# tq
[2]:
  - [1]: 0
  - a
[2]:
  - "Invalid numeric literal at line 1, column 7"
  - [1]: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 41 | +3 | +7.89% |
| `cl100k_base` | 38 | 38 | 41 | +3 | +7.89% |

#### manual.invoking.stream-nested

```
# input
jq --stream .

# jq
[
  [
    0
  ],
  []
]
[
  [
    1
  ],
  "a"
]
[
  [
    2,
    0
  ],
  "b"
]
[
  [
    2,
    0
  ]
]
[
  [
    2
  ]
]

# tq -o json
[
  [
    0
  ],
  []
]
[
  [
    1
  ],
  "a"
]
[
  [
    2,
    0
  ],
  "b"
]
[
  [
    2,
    0
  ]
]
[
  [
    2
  ]
]

# tq
[2]:
  - [1]: 0
  - [0]:
[2]:
  - [1]: 1
  - a
[2]:
  - [2]: 2,0
  - b
[1]:
  - [2]: 2,0
[1]:
  - [1]: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 68 | 68 | 72 | +4 | +5.88% |
| `cl100k_base` | 68 | 68 | 72 | +4 | +5.88% |

#### manual.invoking.stream-reduce

```
# input
jq -n --stream 'reduce inputs as $event (0; . + 1)'

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.stream-scalar

```
# input
jq --stream .

# jq
[
  [],
  "a"
]

# tq -o json
[
  [],
  "a"
]

# tq
[2]:
  - [0]:
  - a
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 12 | +4 | +50% |
| `cl100k_base` | 8 | 8 | 12 | +4 | +50% |

#### manual.invoking.tab-output

```
# input
jq -n --tab '{a:{b:1}}'

# jq
{
	"a": {
		"b": 1
	}
}

# tq -o json
{
	"a": {
		"b": 1
	}
}

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | n/a | n/a | n/a |
| `cl100k_base` | 16 | 16 | n/a | n/a | n/a |

#### manual.invoking.unbuffered

```
# input
jq --unbuffered .

# jq
1
2

# tq -o json
1
2

# tq
1
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual.invoking.unquoted-filter

```
# input
jq foo

# jq


[stderr]
jq: error: foo/0 is not defined at <top-level>, line 1, column 1:
    foo
    ^^^
jq: 1 compile error

# tq -o json


[stderr]
tq: query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/0

# tq


[stderr]
tq: query compilation failed: TQ-RESOLVE-BUILTIN-001: unknown filter foo/0
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual.invoking.user-function

```
# input
jq 'def f: .foo; f'

# jq
42

# tq -o json
42

# tq
42
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.invoking.version

```
# input
jq --version

# jq
jq-1.8.1

# tq -o json
tq 0.1.0 (TOON v3; jq target 1.8.x; revision unknown)

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 25 | n/a | n/a | n/a |
| `cl100k_base` | 8 | 25 | n/a | n/a | n/a |

#### manual.invoking.whitespace-stream

```
# input
jq .

# jq
1
true
[
  2
]

# tq -o json
1
true
[
  2
]

# tq
1
true
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 10 | 0 | +0% |
| `cl100k_base` | 10 | 10 | 10 | 0 | +0% |

<!-- tq-manual-compare:end -->
