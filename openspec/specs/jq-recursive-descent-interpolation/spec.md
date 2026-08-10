# jq Recursive Descent and Interpolation Specification

## Purpose

Define jq-compatible recursive traversal and string interpolation semantics with
bounded execution and stable resource behavior.

## Requirements

### Requirement: Recursive descent
The system SHALL implement jq recursive descent over arrays and insertion-ordered
objects using the jq depth-first result order.

#### Scenario: Deep traversal
- **WHEN** `..` traverses a nested value within configured limits
- **THEN** each visited value is emitted in jq order without native-stack recursion

#### Scenario: Traversal limit
- **WHEN** traversal exceeds depth, work, result, or cancellation limits
- **THEN** execution stops with the stable limit class and releases traversal frames

### Requirement: String interpolation
The system SHALL parse and execute jq string interpolation with nested filters,
escaping, generator multiplicity, and source-aware errors.

#### Scenario: Generator interpolation
- **WHEN** one or more embedded filters emit multiple values
- **THEN** the system emits the same ordered string combinations as jq
