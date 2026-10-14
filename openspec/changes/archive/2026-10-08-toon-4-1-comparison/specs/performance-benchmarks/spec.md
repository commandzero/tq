## MODIFIED Requirements

### Requirement: Stack Overflow scenario review pages
The separately invoked Stack Overflow benchmark harness SHALL publish a summary at `docs/tests/stack-overflow/overview.md` and one Markdown page per checked-in scenario. The `docs/tests/stack-overflow/index.md` page SHALL remain frontmatter-free navigation linking the overview and every scenario page, without a generated Results summary. Each scenario page SHALL identify its source question, selected answer, exact benchmark query, and input. Adapted benchmark queries MUST NOT be presented as complete coverage of the original question. The harness SHALL generate a bottom `## Results` section on the overview and scenario pages with correctness outcomes and comparable tool metrics while preserving authored review text outside the generated section.

The harness SHALL support rendering saved measurements without executing benchmarks. Rendering MUST validate that the measurements belong to the selected scenarios and MUST retain unsuccessful outcomes. Historical measurements without authoritative RSS provenance MUST be labeled unverified rather than accepted as current benchmark evidence.

#### Scenario: Publish a Stack Overflow campaign
- **WHEN** the separately invoked campaign produces a report
- **THEN** the overview summarizes coverage, the frontmatter-free index links the overview and every scenario page without duplicating generated results, and each scenario page shows the tools and their measured metrics together

#### Scenario: Regenerate reviewed results
- **WHEN** saved measurements are rendered into existing scenario pages
- **THEN** the harness replaces generated results, preserves authored review text, and does not run any measured tools

#### Scenario: Reject unrelated saved measurements
- **WHEN** a saved campaign does not match the selected scenario identities, queries, or inputs
- **THEN** rendering fails before publishing misleading scenario results
