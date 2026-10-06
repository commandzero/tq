## ADDED Requirements

### Requirement: TOON 4.1 option surface
The CLI SHALL expose canonical TOON 4.1 indentation, delimiter and strictness controls while retaining existing framing, raw and color modes. It SHALL NOT accept `--fold-keys` or `--flatten-depth`, alias them to another behavior, or silently ignore them. Help and migration documentation SHALL identify literal dotted keys and the removed options. Public TOON configuration SHALL NOT expose key folding or path expansion.

#### Scenario: Removed folding flag
- **WHEN** an invocation supplies `--fold-keys`
- **THEN** argument parsing fails with the ordinary unknown-option contract before input consumption

#### Scenario: Removed flatten-depth flag
- **WHEN** an invocation supplies `--flatten-depth 2`
- **THEN** argument parsing fails before input consumption instead of changing object structure or ignoring the option

#### Scenario: Remaining format controls
- **WHEN** output uses a supported delimiter and indentation with default, sequence or exactly-one unframed framing
- **THEN** canonical 4.1 formatting applies independently of the unchanged selected framing

### Requirement: TOON 4.1 bounded input discovery
Automatic input discovery SHALL recognize valid TOON 4.1 keyed root headers and leading BOM/full-line comments within its configured lookahead without misclassifying keyed headers as JSON arrays or falling down after commitment. Explicit TOON input and `.toon` files SHALL accept root `[]`. A bare `[]` without format selection SHALL retain the existing JSON-first shared-array tie-break, since its value is identical in both formats. Explicit format selection SHALL continue to disable detection/faildown.

#### Scenario: Keyed root header
- **WHEN** automatic input begins with `[2:]{age}:` followed by valid entry rows
- **THEN** bounded discovery selects TOON and evaluation receives an object, not an array

#### Scenario: Comment-prefixed document
- **WHEN** a valid TOON object or header follows BOM and full-line comments within the lookahead bound
- **THEN** discovery and decoding agree without interpreting comment text as scalar data

#### Scenario: Empty root file
- **WHEN** a `.toon` file or explicit TOON stdin contains `[]`
- **THEN** evaluation receives an empty array and canonical default output is `[]` followed by LF

#### Scenario: Shared empty-array syntax
- **WHEN** syntax probing receives bare `[]` without an explicit format or recognized extension
- **THEN** existing JSON-first tie-breaking remains valid and does not change the resulting empty-array value

#### Scenario: Bounded ambiguous prefix
- **WHEN** comments or incomplete headers exhaust the configured probing lookahead
- **THEN** discovery follows its bounded diagnostic/commitment contract without collecting the complete document
