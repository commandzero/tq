## ADDED Requirements

### Requirement: Parallel selected-decode planning
Before semantic decoder consumption, automatic planning SHALL identify hybrid-blocking JSON plans whose input dependency is a statically selected array with an optional static element-local projection. The plan SHALL record whether bounded parallel selected decoding is eligible and why any fallback was selected.

#### Scenario: Eligible blocking projection
- **WHEN** analysis proves a blocking query collects values projected from independent elements of one static JSON array and multiple workers are available
- **THEN** the selected plan enables bounded parallel selected decoding

#### Scenario: Dynamic dependency
- **WHEN** the selected input path or element projection depends on runtime data
- **THEN** the plan retains serial decoding and explains that the dependency is not statically partitionable
