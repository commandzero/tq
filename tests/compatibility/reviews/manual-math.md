# jq manual math coverage

Source: [`jq-manual/math.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/math.md). The canonical ledger is [manual-math.toon](manual-math.toon), with an equivalent [JSON comparison copy](manual-math.toon).

The source has prose and signature lists only. It has no tables or fenced examples.

The corrected ledger separates 62 executable examples from 6 coverage notes. Every named function has a case in [manual-math.jsonl](../cases/manual-math.jsonl), and all jq/tq adapters stay enabled so unsupported functions remain visible.

See the [model and storage comparison](manual-math-model.md) for the schema, migration details, and measured character/token savings.

The one-input witnesses use small values in each function's domain. For example,
`acosh` uses 1, `atanh` uses 0, and `sqrt` uses 4. The ledger records every input.

The two-input witnesses are finite, valid, and simple: `atan2(0;1)`, `copysign(2;-1)`, `drem(5;2)`, `fdim(5;2)`, `fmax(2;3)`, `fmin(2;3)`, `fmod(5;2)`, `hypot(3;4)`, `jn(0;0)`, `ldexp(1;3)`, `nextafter(1;2)`, `nexttoward(1;2)`, `pow(2;3)`, `remainder(5;2)`, `scalb(2;3)`, `scalbln(2;3)`, and `yn(0;1)`. `fma(2;3;4)` covers the three-input signature.

There is a target discrepancy. The published two-input list names `frexp` and `modf`, but jq 1.8.1 at `target/reference-build/jq/jq` reports both as undefined (`frexp/2` and `modf/2`). Their cases retain the documented query and record a compile error as informative, with the review disposition `reference-unavailable`.

jq's builtin list provides `frexp/0` and `modf/0`. Additional cases test those
arities with inputs 8 and 3.5, respectively. The observed reference outputs
are `[0.5,4]` and `[0.5,3]`.
