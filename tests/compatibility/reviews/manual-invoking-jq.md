# Invoking jq coverage audit

Source: jq manual, [Invoking jq](https://jqlang.org/manual/#invoking-jq). See the [machine-readable ledger](manual-invoking-jq.toon).

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

