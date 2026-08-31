## ADDED Requirements

### Requirement: Remaining input consumption
The `inputs` built-in SHALL pull and decode the remaining ordered documents from the active stdin or file source set. A document consumed by `inputs` MUST NOT later become a separate top-level evaluation input. Decoding, proxy, byte, depth, cancellation, and source-order behavior MUST remain the same as top-level CLI input processing.

#### Scenario: Consume remaining stdin values
- **WHEN** three JSON values are supplied on stdin and the first evaluation runs `[., inputs]`
- **THEN** one result containing all three values is emitted and tq does not run the filter again for the second or third value

#### Scenario: Consume remaining files
- **WHEN** the filter calls `inputs` while tq is processing the first of several input files
- **THEN** it emits documents from the remaining files in command-line order

#### Scenario: No remaining input
- **WHEN** `inputs` is evaluated after the active source set is exhausted
- **THEN** it emits zero results rather than `null`

#### Scenario: Remaining input fails to decode
- **WHEN** `inputs` reaches malformed structured input without proxy-on-error
- **THEN** evaluation stops with the same classified input failure used by top-level processing

#### Scenario: Null input mode
- **WHEN** `inputs` is evaluated under `--null-input` with no file sources
- **THEN** it emits zero results

