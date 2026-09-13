## Purpose

Define consistent semantic terminal colors for every supported tq output format while preserving the underlying serialized data and bounded streaming behavior.

## ADDED Requirements

### Requirement: Default semantic palette
Colored output SHALL use the following eight-slot palette by default, without requiring an environment variable or configuration file. This assignment also provides a jq testing example:

```sh
export JQ_COLORS='0;39:0;94:0;94:0;35:0;32:0;90:0;90:0;36'
jq -Cn '{nil:null,yes:true,no:false,n:42,s:"hello",items:[1,"two"],empty:{}}'
```

The eight slots follow [jq's documented order](https://jqlang.org/manual/v1.8/#colors):

| jq slot | Default style | SGR |
| --- | --- | --- |
| null | normal document foreground | `0;39` |
| false | boolean light-blue | `0;94` |
| true | boolean light-blue | `0;94` |
| numbers | magenta | `0;35` |
| strings | green | `0;32` |
| arrays | light-black delimiters and punctuation | `0;90` |
| objects | light-black delimiters and punctuation | `0;90` |
| object keys | cyan | `0;36` |

The leading `0` resets emphasis and background before selecting the foreground. ANSI palette slots determine the colors; their rendered RGB values remain the terminal palette's responsibility. This example was checked against jq 1.8.2 for all eight slots. Unlike jq, tq SHALL color enclosing quotation marks with structural delimiters, not string or key content. No additional palette slot is introduced for quotes.

#### Scenario: Typed values and keys
- **WHEN** colored output uses the default palette to render an object with null, boolean, number, and string values
- **THEN** key text is cyan, null has normal document styling, booleans are light-blue, numbers are magenta, and string content is green
- **AND** syntactic quotation marks and punctuation are light-black

#### Scenario: JSON uses tq's palette
- **WHEN** a user selects JSON output with color enabled and no valid palette override
- **THEN** JSON uses tq's default palette, just like the other output formats

### Requirement: Compatible JQ_COLORS overrides
The CLI SHALL retain its supported `JQ_COLORS` environment-variable interface and apply overrides across every supported output format. Eight colon-separated SGR entries SHALL map to null, false, true, numbers, strings, arrays, objects, and object keys, in that order. The existing seven-entry form SHALL remain accepted and use the number entry for object keys. Each entry SHALL be nonempty and contain only ASCII digits and semicolons, preserving the existing parser's acceptance rules. Unset or invalid values SHALL fall back to tq's complete default palette. Invalid values SHALL NOT be echoed in diagnostics.

The CLI SHALL resolve the palette once per invocation when environment access is permitted. When environment access is denied, it SHALL use the built-in palette without reading `JQ_COLORS`. Setting a palette SHALL NOT itself enable color or override `NO_COLOR`, monochrome selection, or terminal capability policy. No new theme file or theme-selection flag is introduced.

#### Scenario: Custom eight-slot palette
- **WHEN** environment access is permitted and a valid eight-entry `JQ_COLORS` is supplied with color enabled
- **THEN** every output writer uses those entries for the corresponding scalar, key, and structural tokens
- **AND** false and true may have different styles

#### Scenario: Seven-slot compatibility and invalid input
- **WHEN** a valid seven-entry palette is supplied
- **THEN** object-key content uses the number entry
- **AND** an invalid palette instead selects the complete tq default, not a partially applied palette or jq's default

### Requirement: Quotation marks share structural styles
Opening and closing syntactic quotation marks SHALL use the same palette entry as the enclosing structure's delimiters and separators, rather than the string or object-key entry. Array brackets and their separators SHALL use the array entry; object braces, colons, and separators SHALL use the object entry. Key quotes SHALL use the object entry. Value quotes SHALL use their immediate enclosing container's entry, with the object entry as the fallback for a standalone quoted scalar. Nested containers SHALL select their own structural entry; separators between them retain their parent's entry.

YAML and TOON sequence syntax SHALL use the array entry, and mapping syntax SHALL use the object entry. TOON table header/row fields SHALL use object context for field quotes and separators; outer array syntax SHALL retain array context. CSV/TSV headers, field separators, and syntactic field quotes SHALL use the object entry. Document markers SHALL use the object entry. These rules SHALL apply to custom palettes as well as the built-in palette; quotation marks SHALL NOT be hardcoded to light black.

#### Scenario: Default JSON quote colors
- **WHEN** the default palette renders `{"key":["value"]}` in color
- **THEN** all enclosing quotes and the characters `{ } : [ ]` are light black, key content is cyan, and value content is green

#### Scenario: Distinct custom structural styles
- **WHEN** a custom palette assigns different styles to arrays, objects, strings, and keys
- **THEN** quotes around a string directly inside an array use the array style, quotes around keys and strings directly inside an object use the object style, and a root string's quotes use the object style
- **AND** string and key content retain their respective custom styles

### Requirement: Complete output-format coverage
Color policy SHALL apply independently of the input format to JSON, YAML, TOON, JSON Lines and its NDJSON alias, JSON Text Sequences, TOON Text Sequences, CSV, and TSV. All supported pretty, compact, indentation, key-ordering, escaping, TOON folding, delimiter, and framing variants SHALL retain their existing serialization behavior. JSON5 SHALL remain input-only; this change SHALL NOT add an output writer.

#### Scenario: Cross-format output selection
- **WHEN** the same valid result is written with forced color in each supported output format
- **THEN** every writer accepts color and applies the shared roles to the tokens its format emits

#### Scenario: JSON file input
- **WHEN** a file ending in `.json` is queried with `-o json -C`
- **THEN** its output uses the shared palette and the filename does not bypass coloring

### Requirement: Roles follow serialized meaning
Writers SHALL distinguish syntax from content using the serialized token's role. Object keys, including TOON table headers and CSV/TSV header fields, SHALL use the object-key entry. Scalar contents SHALL use their serialized type, so a quoted numeric-looking string uses the string entry (green by default). Escaped content SHALL retain its scalar/key role, including JSON escape spellings and doubled CSV quotes; opening and closing syntactic quotation marks SHALL use the structural entry defined above.

Container delimiters and separators SHALL use their structural entry. TOON array counts SHALL use the number entry; counts and ordinary numeric array values SHALL NOT be styled as indices. Tabs, spaces, newlines, and RS transport bytes SHALL retain their bytes without acquiring printable substitutes.

Roles SHALL apply only where the writer emits the corresponding syntax. The noninteractive CLI SHALL NOT synthesize array-index labels, focus state, or trailing commas. These do not introduce additional `JQ_COLORS` entries.

#### Scenario: Escaped punctuation is content
- **WHEN** a string contains braces, commas, a quote, or the word `true`
- **THEN** that content keeps the string role and only actual serializer syntax receives punctuation or quotation-mark styling

#### Scenario: Delimited fields preserve type
- **WHEN** a CSV or TSV row uses the default palette and contains string `"42"`, number `42`, boolean `true`, and null
- **THEN** the string content is green, the number magenta, the boolean light-blue, and the empty null field gains no placeholder characters
- **AND** header field content is cyan and delimiters and syntactic quotes are light-black

### Requirement: Terminal policy and explicit overrides
The CLI SHALL enable color automatically for every supported output format when stdout is a terminal, terminal access is permitted, and no nonempty `NO_COLOR` value is visible under the environment capability policy. Automatic output to a file or pipe SHALL remain plain. The last explicit `-C/--color-output` or `-M/--monochrome-output` flag SHALL win; `-C` SHALL override `NO_COLOR` and terminal detection, and `-M` SHALL disable decoration. Explicit color SHALL fail before input consumption when terminal capability is denied. Environment denial SHALL prevent reading `NO_COLOR` or other ambient theme information. Non-process/library writers SHALL default to plain output and SHALL NOT independently inspect the environment or terminal.

#### Scenario: Terminal and redirected output
- **WHEN** no explicit color flag is supplied, capabilities permit detection, and `NO_COLOR` is unset
- **THEN** a terminal receives colored presentation and a file or pipe receives plain format bytes

#### Scenario: Explicit overrides
- **WHEN** `NO_COLOR=1` is visible and stdout is redirected
- **THEN** `-C` enables color, `-C -M` disables color, and `-M -C` enables color

#### Scenario: Restricted caller
- **WHEN** terminal access is denied by the caller
- **THEN** automatic color stays disabled and explicit `-C` returns a usage/policy error before consuming input

### Requirement: Reversible presentation and frame boundaries
Color SHALL add only tq-generated ANSI SGR sequences around serialized token spans. Removing those generated sequences SHALL produce bytes identical to the same invocation with color disabled. This invariant SHALL include accepted exact number spellings, escaping, key order, whitespace, delimiters, and record/frame boundaries. Existing format and strict-conversion contracts SHALL apply to the undecorated serialization. Forced-color output is a terminal presentation stream; consumers requiring directly parseable format bytes SHALL use automatic redirected output or `-M`.

Writers SHALL finish style spans before emitting RS, LF, or CR framing bytes and SHALL resume styling after embedded line breaks as needed. Each complete record SHALL finish with reset presentation state. Empty output SHALL contain no ANSI sequences. The renderer SHALL attempt a style reset after a partial write failure where the output sink remains usable, without replacing the original error or retrying a broken pipe.

#### Scenario: Sequence framing survives styling
- **WHEN** multiple JSON, JSON Lines, JSON-sequence, or TOON-sequence results are colored
- **THEN** removing generated SGR yields exactly the corresponding plain sequence, with unchanged RS/LF bytes and no style carried between records

#### Scenario: No results or a rejected atomic result
- **WHEN** the query emits nothing or a result fails validation before publication
- **THEN** no styling-only output is published for that result

### Requirement: Raw and proxy bytes remain verbatim
Raw string results under `-r`, `-j`, or `--raw-output0` SHALL bypass styling, even with `-C`. Non-string results that those modes serialize structurally SHALL use the selected format's palette. Proxy-on-error bytes SHALL pass through unchanged with no inserted styling. Query format operators such as `@json` and `@csv` SHALL remain string-producing operations; their contents SHALL NOT be reparsed to assign nested syntax colors.

#### Scenario: Raw formatted string
- **WHEN** `-r -C` writes the string result of `@json`
- **THEN** the raw bytes match the same invocation with `-M`, with no tq-generated escapes

#### Scenario: Proxy between colored results
- **WHEN** an unparseable source is proxied between accepted colored structured results
- **THEN** proxy bytes are unchanged and preceding tq styles have already been reset

### Requirement: Bounded coloring and output accounting
Coloring SHALL preserve incremental writing and the selected plan's bounded-memory guarantees, including identity transcode and long sequences. It SHALL NOT require retaining a complete serialized result or reparsing its output. Actual emitted bytes, including generated SGR, SHALL count toward output-byte limits. Any staged colored bytes SHALL count toward applicable spool limits. Existing atomic publication, cancellation, and broken-pipe behavior SHALL be preserved.

#### Scenario: Large streamed output
- **WHEN** a transcode or sequence writer emits many values with color enabled
- **THEN** retained coloring state does not grow with total result size or previously emitted records

#### Scenario: Styling reaches output limit
- **WHEN** generated SGR would exceed the remaining output-byte allowance
- **THEN** tq reports the existing output resource-limit outcome and preserves the selected writer's publication guarantees
