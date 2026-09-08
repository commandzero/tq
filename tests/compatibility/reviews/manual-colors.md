# jq manual: colors coverage review

Source: [`jq-manual/colors.md`](/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/colors.md), jq 1.8. The machine-readable ledger is [manual-colors.toon](manual-colors.toon). This section has no fenced or tabular examples.

Inventory: 26 records and 3 raw-byte MVP cases. The default-palette case copies the exact eight-entry palette from line 27; the custom case repeats the documented `1;31` component across all eight slots; and the ANSI matrix assigns every listed style (`1`, `2`, `4`, `5`, `7`, `8`) and color (`30` through `37`) value to a slot. Each case keeps jq and tq enabled, with `-C`, explicit `JQ_COLORS`, and empty `NO_COLOR`; tq's current disregard for `JQ_COLORS` therefore remains observable.

The eight slot records (lines 18–25) all reuse the aggregate object fixture, which emits null, false, true, a number, a string, an array, an object, and object keys. There are no non-executable records and no fenced blocks. The harness cannot make claims about tty auto-detection; it only compares captured bytes from forced-color invocations.
