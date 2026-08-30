## ADDED Requirements

### Requirement: Ordered parallel decode boundary
The runtime SHALL treat ordered output from parallel selected decoding as the original input result sequence. Concurrent decoding MUST NOT make VM evaluation concurrent or change result, error, or stable-sort ordering.

#### Scenario: Fallible downstream filter
- **WHEN** ordered decoded elements enter a fallible VM filter
- **THEN** the VM evaluates them serially in source order and reports the same first runtime error as serial execution

#### Scenario: Stable blocking sort
- **WHEN** equal sort keys originate in different decode batches
- **THEN** their relative result order matches serial execution
