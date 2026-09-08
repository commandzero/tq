# tq compatibility language

The manual audit relates source material to executable compatibility checks.

## Language

**Source passage**: A located piece of the reference manual, such as a paragraph,
signature list, or code block. A passage may describe several executable examples.

**Executable example**: One concrete query-and-input scenario represented by one
compatibility case. Shared input alone and prose claims are not executable examples.

**Compatibility case**: A runnable check with tool-specific invocations and an
expected result or process contract. Several source passages may cite the same case.

**Coverage note**: An audited prose claim, scope statement, or exclusion with its
rationale. A note may cite one or several executable cases as supporting evidence;
the claim is distinct from each of its evidence relationships.

**Coverage evidence**: One relationship between a coverage note and an executable
case. Multiple witnesses do not turn a source claim into one multi-case example.

**Fixture**: Reusable input for compatibility cases. Reusing a fixture does not
make the cases one executable example.

**Compatibility verdict**: The assessed relationship between an observed case
and its reference contract, distinct from whether the source has been audited.
