## Purpose

Define bounded and deterministic parallel decoding for independent elements of a statically selected JSON array.

## ADDED Requirements

### Requirement: Ordered parallel selected decoding
For an eligible JSON document, the system SHALL frame independent elements of the statically selected array, decode and project element batches concurrently, and deliver results to downstream execution in source order.

#### Scenario: Workers complete out of order
- **WHEN** a later element batch finishes decoding before an earlier batch
- **THEN** downstream execution receives both batches in original array order

#### Scenario: Projection omits a field
- **WHEN** an element does not contain the statically selected projection
- **THEN** the parallel decoder produces the same null placeholder and path as the serial selected decoder

### Requirement: Deterministic failure order
The parallel decoder MUST report the earliest input-order decoding or resource failure and MUST NOT expose values from a later batch before that failure.

#### Scenario: Multiple malformed batches
- **WHEN** more than one independently scheduled batch would fail
- **THEN** the reported diagnostic corresponds to the earliest failing input batch

### Requirement: Sound serial fallback
The system SHALL use the existing serial selected decoder when the input, query proof, execution mode, or configured worker count is not eligible for parallel selected decoding.

#### Scenario: One worker configured
- **WHEN** the effective Rayon worker count is one
- **THEN** execution uses the serial selected decoder and retains its observable behavior
