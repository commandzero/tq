## ADDED Requirements

### Requirement: Capability-governed jq environment variable

The CLI SHALL apply the existing environment capability contract to `$ENV` as well as `env`. `--allow-environment` SHALL admit one startup snapshot for the query, while the default-denied mode and an explicitly denying `CapabilityPolicy` SHALL prevent ambient values from becoming query-visible.

#### Scenario: Explicitly allowed environment
- **WHEN** `--allow-environment` is supplied and environment access is permitted by `CapabilityPolicy`
- **THEN** `$ENV` and `env` expose the same startup snapshot during that invocation

#### Scenario: Default environment denial
- **WHEN** a query references `$ENV` without `--allow-environment`
- **THEN** the query is not rejected as an unknown variable, but evaluation returns an environment-policy error before any environment value is exposed

#### Scenario: Library policy denial wins
- **WHEN** `--allow-environment` is supplied but the library's `CapabilityPolicy.environment` is false
- **THEN** the CLI rejects the incompatible request or otherwise returns the existing environment-policy classification before input processing, consistent with `env`

#### Scenario: Environment values stay out of diagnostics
- **WHEN** an environment-dependent query fails because ambient access is denied
- **THEN** stderr and machine-readable diagnostics identify the denied operation and policy class without serializing environment contents

#### Scenario: CLI special-variable names are reserved
- **WHEN** an external argument is supplied with the name `ENV` or `__loc__`
- **THEN** it does not replace the corresponding jq special variable reference
