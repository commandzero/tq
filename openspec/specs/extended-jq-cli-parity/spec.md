# Extended jq CLI Parity Specification

## Purpose

Define reviewed jq 1.8.x command-line compatibility, explicit TOON/YAML
adaptations, and governed access to scripting integrations.

## Requirements

### Requirement: Reviewed jq option parity
The system SHALL implement each promoted jq 1.8.x option according to executable
baseline cases for argv parsing, input consumption, output bytes, diagnostics,
and exit status.

#### Scenario: Supported option combination
- **WHEN** a reviewed jq-compatible option combination is invoked
- **THEN** tq matches the target contract or an explicit TOON/YAML adaptation

#### Scenario: Invalid combination
- **WHEN** options conflict or require a deferred capability
- **THEN** tq rejects them before input with a stable usage or unsupported status

### Requirement: Governed scripting integration
Filesystem, environment, terminal, and module-facing CLI features MUST honor
library capability policy, resource limits, and secret-safe reporting.

#### Scenario: Broken pipe and multiple files
- **WHEN** ordered files produce results and a downstream reader closes
- **THEN** complete prior frames remain valid, open resources are cleaned up, and tq exits without a noisy pipe error
