## Why

`input_line_number` currently fails unless the caller enables platform access, even though tq derives the value from input it is already decoding. This differs from jq's default behavior and makes a harmless input property subject to a policy intended for ambient clock, timezone, and host metadata.

## What Changes

- Make `input_line_number` available without `--allow-platform` when the CLI or library runtime has input context.
- Keep `now`, local-time operations, and `input_filename` subject to the existing platform capability policy.
- Add compatibility and policy regression cases that distinguish decoder-owned line metadata from ambient platform access.
- Update CLI help, compatibility documentation, and generated evidence so they no longer describe `input_line_number` as gated platform metadata.
- Preserve the current line-number contract. This change does not add exact physical-line tracking for every decoded format or record.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `jq-regex-date-platform`: Classify `input_line_number` as decoder-owned input context that is available by default, while retaining the platform gate for ambient and source-identity operations.

## Impact

The change affects built-in evaluation in `tq-core`, runtime context assembly in `tq-cli`, ambient-policy tests, jq differential fixtures and reports, CLI help text, and the regex/date/platform compatibility documentation. It relaxes one runtime policy check and adds no dependency or command-line option.
