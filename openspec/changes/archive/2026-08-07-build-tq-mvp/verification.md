# Verification report: build-tq-mvp

## Summary

| Dimension | Status |
| --- | --- |
| Completeness | 198/198 task boxes are checked, but task 15.7 remains substantively incomplete |
| Correctness | Build, compatibility, and benchmark gates pass; two mandatory diagnostic contracts remain unresolved |
| Coherence | Typestate plans, streaming I/O, compatibility publication, and bounded benchmark correctness agree with the design; diagnostic metadata does not yet agree with the specs |
| Archive readiness | Blocked until the two critical diagnostic findings are fixed and reverified |

## Verified evidence

- Formatting, strict workspace Clippy, workspace documentation, stable workspace tests, Rust 1.87.0 workspace tests, and strict OpenSpec validation pass.
- The release fuzz evidence covers `query_parser`, `toon_decoder`, `bytecode_decode`, `vm_program`, and `cli_args` without a crash artifact.
- `tests/compatibility/reviews/coverage-v1.json` contains 154 cases and 831 observations for jq 1.8.2, yq 4.53.2, and tq over JSON, YAML, and TOON. Its jq/tq difference allowlist remains executable.
- `docs/requirements-traceability.tsv` has one row per scenario. Every route now declares a test, report assertion, or manual finding and names a source symbol/heading checked by `requirements_traceability.rs`.
- Benchmark semantic correctness uses an incremental, format-independent digest. It consumes JSON result texts and TOON Text Sequence records individually instead of retaining the complete normalized result sequence.
- The `commandzero/tq-benchmarks` archive retains the refreshed standard and natural-large campaigns, including unfavorable resource and timeout outcomes. The final large event rows remain below the 128 MiB objective.

## Critical issues

### Critical: runtime diagnostic context

The runtime type-error contract requires the query operator span plus input document/path context. `VmError::Runtime` currently retains only a message, and its conversion to the shared diagnostic adds no query or input labels. The manual probe `printf '"x"\n' | tq --input-format json --output-format json '. + 1'` reports only `runtime error: cannot add string and number`.

### Critical: resource diagnostic details

The resource contract requires the limit name, configured threshold, observed or attempted value, execution phase, and relevant span. VM and CLI resource errors currently retain only a static resource name. The manual probe using `--max-vm-steps 1 '. | .'` reports only `VM resource limit exceeded: vm-steps`.

## Manual release checks

### Manual: spool interruption cleanup

The spool owns its securely created temporary file through RAII, and dropping the active preparation state removes the artifact. The spool cleanup unit test exercises the same ownership path after transitioning to disk.

### Manual: downstream pipe closure

The CLI entry point classifies `BrokenPipe` from both direct I/O and structured-output errors and returns success without emitting a diagnostic or panic.

### Manual: user interrupt

The CLI installs a `SIGINT` flag, the VM polls the shared cancellation flag, and VM/spool drops release worker and temporary state before interrupted status is returned.

## Warnings

None. The prior traceability, benchmark-normalization, and stale-verification warnings were remediated without reclassifying the two critical findings.

## Assessment

`build-tq-mvp` is **not ready for archive**. Resolve both critical diagnostic contracts, change task 15.7 back to incomplete while that work is pending or complete it in the same change, and rerun verification.
