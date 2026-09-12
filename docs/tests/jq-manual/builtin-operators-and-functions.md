---
type: Report
title: "jq manual: builtin operators and functions coverage review"
description: "Recorded review of jq manual: builtin operators and functions coverage review."
generated: { by: codex/gpt-6, at: 2026-09-10T02:12:04Z }
---

# jq manual: builtin operators and functions coverage review

Source: [pinned jq manual source inventory](../../../tests/compatibility/reviews/jq-manual/source-examples.toon)

Inventory: 185 records, including 146 table examples and 5 fenced blocks.

Coverage cases: 174 new, 1 exact existing, 0 gaps, and 6 non-executable records.

Execution: full compatibility campaign with `target/reference-build/jq/jq (jq-1.8.1)` and `target/debug/tq (tq 0.1.0)`. All 174 new cases executed in tq's JSON path; 103 matched jq semantically and 71 had recorded jq/tq differences. The existing `date.strptime` case matched.

Coverage status is about whether the manual example has a case. Execution status is separate: `match`, `divergent`, or `not-run`. Expected-output values are copied from the manual where the source gives them. Synthesized setup is called out in the reason field.

## Example inventory

1. **table-001** (line 36, table; heading `Addition: \`+\``)
   - Query: `.a + 1`
   - Input: `{"a": 7}`
   - Expected output: `8`
   - Case ID(s): `manual-bof-table-001`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

2. **table-002** (line 42, table; heading `Addition: \`+\``)
   - Query: `.a + .b`
   - Input: `{"a": [1,2], "b": [3,4]}`
   - Expected output: `[1,2,3,4]`
   - Case ID(s): `manual-bof-table-002`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

3. **table-003** (line 48, table; heading `Addition: \`+\``)
   - Query: `.a + null`
   - Input: `{"a": 1}`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-003`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

4. **table-004** (line 54, table; heading `Addition: \`+\``)
   - Query: `.a + 1`
   - Input: `{}`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-004`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

5. **table-005** (line 60, table; heading `Addition: \`+\``)
   - Query: `{a: 1} + {b: 2} + {c: 3} + {a: 42}`
   - Input: `null`
   - Expected output: `{"a": 42, "b": 2, "c": 3}`
   - Case ID(s): `manual-bof-table-005`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

6. **table-006** (line 70, table; heading `Subtraction: \`-\``)
   - Query: `4 - .a`
   - Input: `{"a":3}`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-006`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

7. **table-007** (line 76, table; heading `Subtraction: \`-\``)
   - Query: `. - ["xml", "yaml"]`
   - Input: `["xml", "yaml", "json"]`
   - Expected output: `["json"]`
   - Case ID(s): `manual-bof-table-007`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

8. **table-008** (line 92, table; heading `Multiplication, division, modulo: \`*\`, \`/\`, \`%\``)
   - Query: `10 / . * 3`
   - Input: `5`
   - Expected output: `6`
   - Case ID(s): `manual-bof-table-008`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

9. **table-009** (line 98, table; heading `Multiplication, division, modulo: \`*\`, \`/\`, \`%\``)
   - Query: `. / ", "`
   - Input: `"a, b,c,d, e"`
   - Expected output: `["a","b,c,d","e"]`
   - Case ID(s): `manual-bof-table-009`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 5 (runtime-type-path)

10. **table-010** (line 104, table; heading `Multiplication, division, modulo: \`*\`, \`/\`, \`%\``)
   - Query: `{"k": {"a": 1, "b": 2}} * {"k": {"a": 0,"c": 3}}`
   - Input: `null`
   - Expected output: `{"k": {"a": 0, "b": 2, "c": 3}}`
   - Case ID(s): `manual-bof-table-010`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

11. **table-011** (line 110, table; heading `Multiplication, division, modulo: \`*\`, \`/\`, \`%\``)
   - Query: `.[] | (1 / .)?`
   - Input: `[1,0,-1]`
   - Expected output: `1 | -1`
   - Case ID(s): `manual-bof-table-011`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

12. **table-012** (line 125, table; heading `\`abs\``)
   - Query: `map(abs)`
   - Input: `[-10, -1.1, -1e-1]`
   - Expected output: `[10,1.1,1e-1]`
   - Case ID(s): `manual-bof-table-012`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

13. **table-013** (line 147, table; heading `\`length\``)
   - Query: `.[] | length`
   - Input: `[[1,2], "string", {"a":2}, null, -5]`
   - Expected output: `2 | 6 | 1 | 0 | 5`
   - Case ID(s): `manual-bof-table-013`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

14. **table-014** (line 161, table; heading `\`utf8bytelength\``)
   - Query: `utf8bytelength`
   - Input: `"\u03bc"`
   - Expected output: `2`
   - Case ID(s): `manual-bof-table-014`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

15. **table-015** (line 177, table; heading `\`keys\`, \`keys_unsorted\``)
   - Query: `keys`
   - Input: `{"abc": 1, "abcd": 2, "Foo": 3}`
   - Expected output: `["Foo", "abc", "abcd"]`
   - Case ID(s): `manual-bof-table-015`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

16. **table-016** (line 183, table; heading `\`keys\`, \`keys_unsorted\``)
   - Query: `keys`
   - Input: `[42,3,35]`
   - Expected output: `[0,1,2]`
   - Case ID(s): `manual-bof-table-016`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

17. **table-017** (line 195, table; heading `\`has(key)\``)
   - Query: `map(has("foo"))`
   - Input: `[{"foo": 42}, {}]`
   - Expected output: `[true, false]`
   - Case ID(s): `manual-bof-table-017`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

18. **table-018** (line 201, table; heading `\`has(key)\``)
   - Query: `map(has(2))`
   - Input: `[[0,1], ["a","b","c"]]`
   - Expected output: `[false, true]`
   - Case ID(s): `manual-bof-table-018`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

19. **table-019** (line 211, table; heading `\`in\``)
   - Query: `.[] | in({"foo": 42})`
   - Input: `["foo", "bar"]`
   - Expected output: `true | false`
   - Case ID(s): `manual-bof-table-019`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

20. **table-020** (line 218, table; heading `\`in\``)
   - Query: `map(in([3,4]))`
   - Input: `[2, 0]`
   - Expected output: `[false, true]`
   - Case ID(s): `manual-bof-table-020`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

21. **prose-map-select** (line 220, prose-query; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `[1,2,3] | map(select(. >= 2))`
   - Input: `null`
   - Expected output: `[2,3]`
   - Case ID(s): `manual-bof-prose-map-select`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

22. **fence-map-block** (line 238, fenced-block; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: none
   - Input: `[1]`
   - Expected output: `[2] | [1,1] | [] | [2] | [1] | []`
   - Case ID(s): `manual-bof-code-map-001`, `manual-bof-code-map-002`, `manual-bof-code-map-003`, `manual-bof-code-map-004`, `manual-bof-code-map-005`, `manual-bof-code-map-006`
   - Status: **covered**. fenced block contains six independent commands; each command has its own executable case
   - Execution: **match**. jq/tq JSON semantic observations match

23. **code-map-001** (line 239, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map(.+1)`
   - Input: `[1]`
   - Expected output: `[2]`
   - Case ID(s): `manual-bof-code-map-001`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

24. **code-map-002** (line 240, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map(., .)`
   - Input: `[1]`
   - Expected output: `[1,1]`
   - Case ID(s): `manual-bof-code-map-002`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

25. **code-map-003** (line 241, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map(empty)`
   - Input: `[1]`
   - Expected output: `[]`
   - Case ID(s): `manual-bof-code-map-003`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

26. **code-map-004** (line 243, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map_values(.+1)`
   - Input: `[1]`
   - Expected output: `[2]`
   - Case ID(s): `manual-bof-code-map-004`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

27. **code-map-005** (line 244, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map_values(., .)`
   - Input: `[1]`
   - Expected output: `[1]`
   - Case ID(s): `manual-bof-code-map-005`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

28. **code-map-006** (line 245, code; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map_values(empty)`
   - Input: `[1]`
   - Expected output: `[]`
   - Case ID(s): `manual-bof-code-map-006`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

29. **prose-map-equivalence-1** (line 248, prose-signature; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `[.[] | f]`
   - Input: none
   - Case ID(s): none
   - Status: **non-executable**. placeholder filter f; no concrete input/output example
   - Execution: **not-run**. placeholder filter f; no concrete input/output example

30. **prose-map-equivalence-2** (line 248, prose-signature; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `.[] |= f`
   - Input: none
   - Case ID(s): none
   - Status: **non-executable**. placeholder filter f; no concrete input/output example
   - Execution: **not-run**. placeholder filter f; no concrete input/output example

31. **table-021** (line 252, table; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map(.+1)`
   - Input: `[1,2,3]`
   - Expected output: `[2,3,4]`
   - Case ID(s): `manual-bof-table-021`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

32. **table-022** (line 258, table; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map_values(.+1)`
   - Input: `{"a": 1, "b": 2, "c": 3}`
   - Expected output: `{"a": 2, "b": 3, "c": 4}`
   - Case ID(s): `manual-bof-table-022`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

33. **table-023** (line 264, table; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map(., .)`
   - Input: `[1,2]`
   - Expected output: `[1,1,2,2]`
   - Case ID(s): `manual-bof-table-023`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

34. **table-024** (line 270, table; heading `\`map(f)\`, \`map_values(f)\``)
   - Query: `map_values(. // empty)`
   - Input: `{"a": null, "b": true, "c": false}`
   - Expected output: `{"b":true}`
   - Case ID(s): `manual-bof-table-024`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

35. **table-025** (line 280, table; heading `\`pick(pathexps)\``)
   - Query: `pick(.a, .b.c, .x)`
   - Input: `{"a": 1, "b": {"c": 2, "d": 3}, "e": 4}`
   - Expected output: `{"a":1,"b":{"c":2},"x":null}`
   - Case ID(s): `manual-bof-table-025`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

36. **table-026** (line 286, table; heading `\`pick(pathexps)\``)
   - Query: `pick(.[2], .[0], .[0])`
   - Input: `[1,2,3,4]`
   - Expected output: `[1,null,3]`
   - Case ID(s): `manual-bof-table-026`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

37. **prose-path-boolean** (line 302, prose-query; heading `\`path(path_expression)\``)
   - Query: `path(..|select(type=="boolean"))`
   - Input: `{"a":[true,false,1]}`
   - Expected output: `["a",0] | ["a",1]`
   - Case ID(s): `manual-bof-synth-path-boolean`
   - Status: **new**. synthesized representative boolean-containing input
   - Execution: **divergent**. tq JSON exited 5 (runtime-type-path)

38. **table-027** (line 304, table; heading `\`path(path_expression)\``)
   - Query: `path(.a[0].b)`
   - Input: `null`
   - Expected output: `["a",0,"b"]`
   - Case ID(s): `manual-bof-table-027`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

39. **table-028** (line 310, table; heading `\`path(path_expression)\``)
   - Query: `[path(..)]`
   - Input: `{"a":[{"b":1}]}`
   - Expected output: `[[],["a"],["a",0],["a",0,"b"]]`
   - Case ID(s): `manual-bof-table-028`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 5 (runtime-type-path)

40. **table-029** (line 320, table; heading `\`del(path_expression)\``)
   - Query: `del(.foo)`
   - Input: `{"foo": 42, "bar": 9001, "baz": 42}`
   - Expected output: `{"bar": 9001, "baz": 42}`
   - Case ID(s): `manual-bof-table-029`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

41. **table-030** (line 326, table; heading `\`del(path_expression)\``)
   - Query: `del(.[1, 2])`
   - Input: `["foo", "bar", "baz"]`
   - Expected output: `["foo"]`
   - Case ID(s): `manual-bof-table-030`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

42. **table-031** (line 336, table; heading `\`getpath(PATHS)\``)
   - Query: `getpath(["a","b"])`
   - Input: `null`
   - Expected output: `null`
   - Case ID(s): `manual-bof-table-031`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

43. **table-032** (line 342, table; heading `\`getpath(PATHS)\``)
   - Query: `[getpath(["a","b"], ["a","c"])]`
   - Input: `{"a":{"b":0, "c":1}}`
   - Expected output: `[0, 1]`
   - Case ID(s): `manual-bof-table-032`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

44. **table-033** (line 352, table; heading `\`setpath(PATHS; VALUE)\``)
   - Query: `setpath(["a","b"]; 1)`
   - Input: `null`
   - Expected output: `{"a": {"b": 1}}`
   - Case ID(s): `manual-bof-table-033`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

45. **table-034** (line 358, table; heading `\`setpath(PATHS; VALUE)\``)
   - Query: `setpath(["a","b"]; 1)`
   - Input: `{"a":{"b":0}}`
   - Expected output: `{"a": {"b": 1}}`
   - Case ID(s): `manual-bof-table-034`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

46. **table-035** (line 364, table; heading `\`setpath(PATHS; VALUE)\``)
   - Query: `setpath([0,"a"]; 1)`
   - Input: `null`
   - Expected output: `[{"a":1}]`
   - Case ID(s): `manual-bof-table-035`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

47. **table-036** (line 374, table; heading `\`delpaths(PATHS)\``)
   - Query: `delpaths([["a","b"]])`
   - Input: `{"a":{"b":1},"x":{"y":2}}`
   - Expected output: `{"a":{},"x":{"y":2}}`
   - Case ID(s): `manual-bof-table-036`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

48. **table-037** (line 386, table; heading `\`to_entries\`, \`from_entries\`, \`with_entries(f)\``)
   - Query: `to_entries`
   - Input: `{"a": 1, "b": 2}`
   - Expected output: `[{"key":"a", "value":1}, {"key":"b", "value":2}]`
   - Case ID(s): `manual-bof-table-037`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

49. **table-038** (line 392, table; heading `\`to_entries\`, \`from_entries\`, \`with_entries(f)\``)
   - Query: `from_entries`
   - Input: `[{"key":"a", "value":1}, {"key":"b", "value":2}]`
   - Expected output: `{"a": 1, "b": 2}`
   - Case ID(s): `manual-bof-table-038`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

50. **table-039** (line 398, table; heading `\`to_entries\`, \`from_entries\`, \`with_entries(f)\``)
   - Query: `with_entries(.key |= "KEY_" + .)`
   - Input: `{"a": 1, "b": 2}`
   - Expected output: `{"KEY_a": 1, "KEY_b": 2}`
   - Case ID(s): `manual-bof-table-039`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

51. **table-040** (line 410, table; heading `\`select(boolean_expression)\``)
   - Query: `map(select(. >= 2))`
   - Input: `[1,5,3,0,7]`
   - Expected output: `[5,3,7]`
   - Case ID(s): `manual-bof-table-040`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

52. **table-041** (line 416, table; heading `\`select(boolean_expression)\``)
   - Query: `.[] | select(.id == "second")`
   - Input: `[{"id": "first", "val": 1}, {"id": "second", "val": 2}]`
   - Expected output: `{"id": "second", "val": 2}`
   - Case ID(s): `manual-bof-table-041`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

53. **table-042** (line 426, table; heading `\`arrays\`, \`objects\`, \`iterables\`, \`booleans\`, \`numbers\`, \`normals\`, \`finites\`, \`strings\`, \`nulls\`, \`values\`, \`scalars\``)
   - Query: `.[]|numbers`
   - Input: `[[],{},1,"foo",null,true,false]`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-042`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

54. **table-043** (line 438, table; heading `\`empty\``)
   - Query: `1, empty, 2`
   - Input: `null`
   - Expected output: `1 | 2`
   - Case ID(s): `manual-bof-table-043`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

55. **table-044** (line 445, table; heading `\`empty\``)
   - Query: `[1,2,empty,3]`
   - Input: `null`
   - Expected output: `[1,2,3]`
   - Case ID(s): `manual-bof-table-044`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

56. **table-045** (line 455, table; heading `\`error\`, \`error(message)\``)
   - Query: `try error catch .`
   - Input: `"error message"`
   - Expected output: `"error message"`
   - Case ID(s): `manual-bof-table-045`
   - Status: **new**.
   - Execution: **divergent**. result sequence

57. **table-046** (line 461, table; heading `\`error\`, \`error(message)\``)
   - Query: `try error("invalid value: \(.)") catch .`
   - Input: `42`
   - Expected output: `"invalid value: 42"`
   - Case ID(s): `manual-bof-table-046`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

58. **prose-halt-error** (line 477, prose-query; heading `\`halt_error\`, \`halt_error(exit_code)\``)
   - Query: `"Error: something went wrong\n"|halt_error(1)`
   - Input: `null`
   - Expected output: `raw stderr without newline; exit status 1`
   - Case ID(s): `manual-bof-prose-halt-error`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

59. **table-047** (line 483, table; heading `\`$__loc__\``)
   - Query: `try error("\($__loc__)") catch .`
   - Input: `null`
   - Expected output: `"{\"file\":\"<top-level>\",\"line\":1}"`
   - Case ID(s): `manual-bof-table-047`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

60. **table-048** (line 495, table; heading `\`paths\`, \`paths(node_filter)\``)
   - Query: `[paths]`
   - Input: `[1,[[],{"a":2}]]`
   - Expected output: `[[0],[1],[1,0],[1,1],[1,1,"a"]]`
   - Case ID(s): `manual-bof-table-048`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

61. **table-049** (line 501, table; heading `\`paths\`, \`paths(node_filter)\``)
   - Query: `[paths(type == "number")]`
   - Input: `[1,[[],{"a":2}]]`
   - Expected output: `[[0],[1,1,"a"]]`
   - Case ID(s): `manual-bof-table-049`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

62. **table-050** (line 515, table; heading `\`add\`, \`add(generator)\``)
   - Query: `add`
   - Input: `["a","b","c"]`
   - Expected output: `"abc"`
   - Case ID(s): `manual-bof-table-050`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

63. **table-051** (line 521, table; heading `\`add\`, \`add(generator)\``)
   - Query: `add`
   - Input: `[1, 2, 3]`
   - Expected output: `6`
   - Case ID(s): `manual-bof-table-051`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

64. **table-052** (line 527, table; heading `\`add\`, \`add(generator)\``)
   - Query: `add`
   - Input: `[]`
   - Expected output: `null`
   - Case ID(s): `manual-bof-table-052`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

65. **table-053** (line 533, table; heading `\`add\`, \`add(generator)\``)
   - Query: `add(.[].a)`
   - Input: `[{"a":3}, {"a":5}, {"b":6}]`
   - Expected output: `8`
   - Case ID(s): `manual-bof-table-053`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

66. **table-054** (line 549, table; heading `\`any\`, \`any(condition)\`, \`any(generator; condition)\``)
   - Query: `any`
   - Input: `[true, false]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-054`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

67. **table-055** (line 555, table; heading `\`any\`, \`any(condition)\`, \`any(generator; condition)\``)
   - Query: `any`
   - Input: `[false, false]`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-055`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

68. **table-056** (line 561, table; heading `\`any\`, \`any(condition)\`, \`any(generator; condition)\``)
   - Query: `any`
   - Input: `[]`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-056`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

69. **table-057** (line 577, table; heading `\`all\`, \`all(condition)\`, \`all(generator; condition)\``)
   - Query: `all`
   - Input: `[true, false]`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-057`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

70. **table-058** (line 583, table; heading `\`all\`, \`all(condition)\`, \`all(generator; condition)\``)
   - Query: `all`
   - Input: `[true, true]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-058`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

71. **table-059** (line 589, table; heading `\`all\`, \`all(condition)\`, \`all(generator; condition)\``)
   - Query: `all`
   - Input: `[]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-059`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

72. **table-060** (line 601, table; heading `\`flatten\`, \`flatten(depth)\``)
   - Query: `flatten`
   - Input: `[1, [2], [[3]]]`
   - Expected output: `[1, 2, 3]`
   - Case ID(s): `manual-bof-table-060`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

73. **table-061** (line 607, table; heading `\`flatten\`, \`flatten(depth)\``)
   - Query: `flatten(1)`
   - Input: `[1, [2], [[3]]]`
   - Expected output: `[1, 2, [3]]`
   - Case ID(s): `manual-bof-table-061`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

74. **table-062** (line 613, table; heading `\`flatten\`, \`flatten(depth)\``)
   - Query: `flatten`
   - Input: `[[]]`
   - Expected output: `[]`
   - Case ID(s): `manual-bof-table-062`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

75. **table-063** (line 619, table; heading `\`flatten\`, \`flatten(depth)\``)
   - Query: `flatten`
   - Input: `[{"foo": "bar"}, [{"foo": "baz"}]]`
   - Expected output: `[{"foo": "bar"}, {"foo": "baz"}]`
   - Case ID(s): `manual-bof-table-063`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

76. **table-064** (line 635, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `range(2; 4)`
   - Input: `null`
   - Expected output: `2 | 3`
   - Case ID(s): `manual-bof-table-064`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

77. **table-065** (line 642, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `[range(2; 4)]`
   - Input: `null`
   - Expected output: `[2,3]`
   - Case ID(s): `manual-bof-table-065`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

78. **table-066** (line 648, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `[range(4)]`
   - Input: `null`
   - Expected output: `[0,1,2,3]`
   - Case ID(s): `manual-bof-table-066`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

79. **table-067** (line 654, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `[range(0; 10; 3)]`
   - Input: `null`
   - Expected output: `[0,3,6,9]`
   - Case ID(s): `manual-bof-table-067`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

80. **table-068** (line 660, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `[range(0; 10; -1)]`
   - Input: `null`
   - Expected output: `[]`
   - Case ID(s): `manual-bof-table-068`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

81. **table-069** (line 666, table; heading `\`range(upto)\`, \`range(from; upto)\`, \`range(from; upto; by)\``)
   - Query: `[range(0; -5; -1)]`
   - Input: `null`
   - Expected output: `[0,-1,-2,-3,-4]`
   - Case ID(s): `manual-bof-table-069`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

82. **table-070** (line 676, table; heading `\`floor\``)
   - Query: `floor`
   - Input: `3.14159`
   - Expected output: `3`
   - Case ID(s): `manual-bof-table-070`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

83. **table-071** (line 686, table; heading `\`sqrt\``)
   - Query: `sqrt`
   - Input: `9`
   - Expected output: `3`
   - Case ID(s): `manual-bof-table-071`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

84. **table-072** (line 696, table; heading `\`tonumber\``)
   - Query: `.[] | tonumber`
   - Input: `[1, "1"]`
   - Expected output: `1 | 1`
   - Case ID(s): `manual-bof-table-072`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

85. **table-073** (line 707, table; heading `\`toboolean\``)
   - Query: `.[] | toboolean`
   - Input: `["true", "false", true, false]`
   - Expected output: `true | false | true | false`
   - Case ID(s): `manual-bof-table-073`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

86. **table-074** (line 720, table; heading `\`tostring\``)
   - Query: `.[] | tostring`
   - Input: `[1, "1", [1]]`
   - Expected output: `"1" | "1" | "[1]"`
   - Case ID(s): `manual-bof-table-074`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

87. **table-075** (line 732, table; heading `\`type\``)
   - Query: `map(type)`
   - Input: `[0, false, [], {}, null, "hello"]`
   - Expected output: `["number", "boolean", "array", "object", "null", "string"]`
   - Case ID(s): `manual-bof-table-075`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

88. **table-076** (line 746, table; heading `\`infinite\`, \`nan\`, \`isinfinite\`, \`isnan\`, \`isfinite\`, \`isnormal\``)
   - Query: `.[] | (infinite * .) < 0`
   - Input: `[-1, 1]`
   - Expected output: `true | false`
   - Case ID(s): `manual-bof-table-076`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

89. **table-077** (line 753, table; heading `\`infinite\`, \`nan\`, \`isinfinite\`, \`isnan\`, \`isfinite\`, \`isnormal\``)
   - Query: `infinite, nan | type`
   - Input: `null`
   - Expected output: `"number" | "number"`
   - Case ID(s): `manual-bof-table-077`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

90. **table-078** (line 776, table; heading `\`sort\`, \`sort_by(path_expression)\``)
   - Query: `sort`
   - Input: `[8,3,null,6]`
   - Expected output: `[null,3,6,8]`
   - Case ID(s): `manual-bof-table-078`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

91. **table-079** (line 782, table; heading `\`sort\`, \`sort_by(path_expression)\``)
   - Query: `sort_by(.foo)`
   - Input: `[{"foo":4, "bar":10}, {"foo":3, "bar":10}, {"foo":2, "bar":1}]`
   - Expected output: `[{"foo":2, "bar":1}, {"foo":3, "bar":10}, {"foo":4, "bar":10}]`
   - Case ID(s): `manual-bof-table-079`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

92. **table-080** (line 788, table; heading `\`sort\`, \`sort_by(path_expression)\``)
   - Query: `sort_by(.foo, .bar)`
   - Input: `[{"foo":4, "bar":10}, {"foo":3, "bar":20}, {"foo":2, "bar":1}, {"foo":3, "bar":10}]`
   - Expected output: `[{"foo":2, "bar":1}, {"foo":3, "bar":10}, {"foo":3, "bar":20}, {"foo":4, "bar":10}]`
   - Case ID(s): `manual-bof-table-080`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

93. **table-081** (line 800, table; heading `\`group_by(path_expression)\``)
   - Query: `group_by(.foo)`
   - Input: `[{"foo":1, "bar":10}, {"foo":3, "bar":100}, {"foo":1, "bar":1}]`
   - Expected output: `[[{"foo":1, "bar":10}, {"foo":1, "bar":1}], [{"foo":3, "bar":100}]]`
   - Case ID(s): `manual-bof-table-081`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

94. **table-082** (line 812, table; heading `\`min\`, \`max\`, \`min_by(path_exp)\`, \`max_by(path_exp)\``)
   - Query: `min`
   - Input: `[5,4,2,7]`
   - Expected output: `2`
   - Case ID(s): `manual-bof-table-082`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

95. **table-083** (line 818, table; heading `\`min\`, \`max\`, \`min_by(path_exp)\`, \`max_by(path_exp)\``)
   - Query: `max_by(.foo)`
   - Input: `[{"foo":1, "bar":14}, {"foo":2, "bar":3}]`
   - Expected output: `{"foo":2, "bar":3}`
   - Case ID(s): `manual-bof-table-083`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

96. **table-084** (line 830, table; heading `\`unique\`, \`unique_by(path_exp)\``)
   - Query: `unique`
   - Input: `[1,2,5,3,5,3,1,3]`
   - Expected output: `[1,2,3,5]`
   - Case ID(s): `manual-bof-table-084`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

97. **table-085** (line 836, table; heading `\`unique\`, \`unique_by(path_exp)\``)
   - Query: `unique_by(.foo)`
   - Input: `[{"foo": 1, "bar": 2}, {"foo": 1, "bar": 3}, {"foo": 4, "bar": 5}]`
   - Expected output: `[{"foo": 1, "bar": 2}, {"foo": 4, "bar": 5}]`
   - Case ID(s): `manual-bof-table-085`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

98. **table-086** (line 842, table; heading `\`unique\`, \`unique_by(path_exp)\``)
   - Query: `unique_by(length)`
   - Input: `["chunky", "bacon", "kitten", "cicada", "asparagus"]`
   - Expected output: `["bacon", "chunky", "asparagus"]`
   - Case ID(s): `manual-bof-table-086`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

99. **table-087** (line 852, table; heading `\`reverse\``)
   - Query: `reverse`
   - Input: `[1,2,3,4]`
   - Expected output: `[4,3,2,1]`
   - Case ID(s): `manual-bof-table-087`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

100. **table-088** (line 862, table; heading `\`contains(element)\``)
   - Query: `contains("bar")`
   - Input: `"foobar"`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-088`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

101. **table-089** (line 868, table; heading `\`contains(element)\``)
   - Query: `contains(["baz", "bar"])`
   - Input: `["foobar", "foobaz", "blarp"]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-089`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

102. **table-090** (line 874, table; heading `\`contains(element)\``)
   - Query: `contains(["bazzzzz", "bar"])`
   - Input: `["foobar", "foobaz", "blarp"]`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-090`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

103. **table-091** (line 880, table; heading `\`contains(element)\``)
   - Query: `contains({foo: 12, bar: [{barp: 12}]})`
   - Input: `{"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]}`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-091`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

104. **table-092** (line 886, table; heading `\`contains(element)\``)
   - Query: `contains({foo: 12, bar: [{barp: 15}]})`
   - Input: `{"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]}`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-092`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

105. **table-093** (line 896, table; heading `\`indices(s)\``)
   - Query: `indices(", ")`
   - Input: `"a,b, cd, efg, hijk"`
   - Expected output: `[3,7,12]`
   - Case ID(s): `manual-bof-table-093`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

106. **table-094** (line 902, table; heading `\`indices(s)\``)
   - Query: `indices(1)`
   - Input: `[0,1,2,1,3,1,4]`
   - Expected output: `[1,3,5]`
   - Case ID(s): `manual-bof-table-094`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

107. **table-095** (line 908, table; heading `\`indices(s)\``)
   - Query: `indices([1,2])`
   - Input: `[0,1,2,3,1,4,2,5,1,2,6,7]`
   - Expected output: `[1,8]`
   - Case ID(s): `manual-bof-table-095`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

108. **table-096** (line 918, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `index(", ")`
   - Input: `"a,b, cd, efg, hijk"`
   - Expected output: `3`
   - Case ID(s): `manual-bof-table-096`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

109. **table-097** (line 924, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `index(1)`
   - Input: `[0,1,2,1,3,1,4]`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-097`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

110. **table-098** (line 930, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `index([1,2])`
   - Input: `[0,1,2,3,1,4,2,5,1,2,6,7]`
   - Expected output: `1`
   - Case ID(s): `manual-bof-table-098`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

111. **table-099** (line 936, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `rindex(", ")`
   - Input: `"a,b, cd, efg, hijk"`
   - Expected output: `12`
   - Case ID(s): `manual-bof-table-099`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

112. **table-100** (line 942, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `rindex(1)`
   - Input: `[0,1,2,1,3,1,4]`
   - Expected output: `5`
   - Case ID(s): `manual-bof-table-100`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

113. **table-101** (line 948, table; heading `\`index(s)\`, \`rindex(s)\``)
   - Query: `rindex([1,2])`
   - Input: `[0,1,2,3,1,4,2,5,1,2,6,7]`
   - Expected output: `8`
   - Case ID(s): `manual-bof-table-101`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

114. **table-102** (line 958, table; heading `\`inside\``)
   - Query: `inside("foobar")`
   - Input: `"bar"`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-102`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

115. **table-103** (line 964, table; heading `\`inside\``)
   - Query: `inside(["foobar", "foobaz", "blarp"])`
   - Input: `["baz", "bar"]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-103`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

116. **table-104** (line 970, table; heading `\`inside\``)
   - Query: `inside(["foobar", "foobaz", "blarp"])`
   - Input: `["bazzzzz", "bar"]`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-104`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

117. **table-105** (line 976, table; heading `\`inside\``)
   - Query: `inside({"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]})`
   - Input: `{"foo": 12, "bar": [{"barp": 12}]}`
   - Expected output: `true`
   - Case ID(s): `manual-bof-table-105`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

118. **table-106** (line 982, table; heading `\`inside\``)
   - Query: `inside({"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]})`
   - Input: `{"foo": 12, "bar": [{"barp": 15}]}`
   - Expected output: `false`
   - Case ID(s): `manual-bof-table-106`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

119. **table-107** (line 992, table; heading `\`startswith(str)\``)
   - Query: `[.[]|startswith("foo")]`
   - Input: `["fo", "foo", "barfoo", "foobar", "barfoob"]`
   - Expected output: `[false, true, false, true, false]`
   - Case ID(s): `manual-bof-table-107`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

120. **table-108** (line 1002, table; heading `\`endswith(str)\``)
   - Query: `[.[]|endswith("foo")]`
   - Input: `["foobar", "barfoo"]`
   - Expected output: `[false, true]`
   - Case ID(s): `manual-bof-table-108`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

121. **table-109** (line 1012, table; heading `\`combinations\`, \`combinations(n)\``)
   - Query: `combinations`
   - Input: `[[1,2], [3, 4]]`
   - Expected output: `[1, 3] | [1, 4] | [2, 3] | [2, 4]`
   - Case ID(s): `manual-bof-table-109`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

122. **table-110** (line 1021, table; heading `\`combinations\`, \`combinations(n)\``)
   - Query: `combinations(2)`
   - Input: `[0, 1]`
   - Expected output: `[0, 0] | [0, 1] | [1, 0] | [1, 1]`
   - Case ID(s): `manual-bof-table-110`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

123. **table-111** (line 1034, table; heading `\`ltrimstr(str)\``)
   - Query: `[.[]|ltrimstr("foo")]`
   - Input: `["fo", "foo", "barfoo", "foobar", "afoo"]`
   - Expected output: `["fo","","barfoo","bar","afoo"]`
   - Case ID(s): `manual-bof-table-111`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

124. **table-112** (line 1044, table; heading `\`rtrimstr(str)\``)
   - Query: `[.[]|rtrimstr("foo")]`
   - Input: `["fo", "foo", "barfoo", "foobar", "foob"]`
   - Expected output: `["fo","","bar","foobar","foob"]`
   - Case ID(s): `manual-bof-table-112`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

125. **table-113** (line 1054, table; heading `\`trimstr(str)\``)
   - Query: `[.[]|trimstr("foo")]`
   - Input: `["fo", "foo", "barfoo", "foobarfoo", "foob"]`
   - Expected output: `["fo","","bar","bar","b"]`
   - Case ID(s): `manual-bof-table-113`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

126. **table-114** (line 1070, table; heading `\`trim\`, \`ltrim\`, \`rtrim\``)
   - Query: `trim, ltrim, rtrim`
   - Input: `" abc "`
   - Expected output: `"abc" | "abc " | " abc"`
   - Case ID(s): `manual-bof-table-114`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

127. **table-115** (line 1082, table; heading `\`explode\``)
   - Query: `explode`
   - Input: `"foobar"`
   - Expected output: `[102,111,111,98,97,114]`
   - Case ID(s): `manual-bof-table-115`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

128. **table-116** (line 1092, table; heading `\`implode\``)
   - Query: `implode`
   - Input: `[65, 66, 67]`
   - Expected output: `"ABC"`
   - Case ID(s): `manual-bof-table-116`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

129. **table-117** (line 1104, table; heading `\`split(str)\``)
   - Query: `split(", ")`
   - Input: `"a, b,c,d, e, "`
   - Expected output: `["a","b,c,d","e",""]`
   - Case ID(s): `manual-bof-table-117`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

130. **table-118** (line 1116, table; heading `\`join(str)\``)
   - Query: `join(", ")`
   - Input: `["a","b,c,d","e"]`
   - Expected output: `"a, b,c,d, e"`
   - Case ID(s): `manual-bof-table-118`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

131. **table-119** (line 1122, table; heading `\`join(str)\``)
   - Query: `join(" ")`
   - Input: `["a",1,2.3,true,null,false]`
   - Expected output: `"a 1 2.3 true false"`
   - Case ID(s): `manual-bof-table-119`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

132. **table-120** (line 1128, table; heading `\`join(str)\``)
   - Query: `join(", ")`
   - Input: `{"a":1,"b":2}`
   - Expected output: `"1, 2"`
   - Case ID(s): `manual-bof-table-120`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

133. **table-121** (line 1138, table; heading `\`ascii_downcase\`, \`ascii_upcase\``)
   - Query: `ascii_upcase`
   - Input: `"useful but not for é"`
   - Expected output: `"USEFUL BUT NOT FOR é"`
   - Case ID(s): `manual-bof-table-121`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

134. **table-122** (line 1150, table; heading `\`while(cond; update)\``)
   - Query: `[while(.<100; .*2)]`
   - Input: `1`
   - Expected output: `[1,2,4,8,16,32,64]`
   - Case ID(s): `manual-bof-table-122`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

135. **table-123** (line 1162, table; heading `\`repeat(exp)\``)
   - Query: `[repeat(.*2, error)?]`
   - Input: `1`
   - Expected output: `[2]`
   - Case ID(s): `manual-bof-table-123`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

136. **table-124** (line 1174, table; heading `\`until(cond; next)\``)
   - Query: `[.,1]|until(.[0] < 1; [.[0] - 1, .[1] * .[0]])|.[1]`
   - Input: `4`
   - Expected output: `24`
   - Case ID(s): `manual-bof-table-124`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

137. **fence-recurse-input** (line 1184, fenced-block; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: none
   - Input: `{"name":"/","children":[{"name":"/bin","children":[{"name":"/bin/ls","children":[]},{"name":"/bin/sh","children":[]}]},{"name":"/home","children":[{"name":"/home/stephen","children":[{"name":"/home/stephen/jq","children":[]}]}]}]}`
   - Case ID(s): `manual-bof-code-recurse-query`
   - Status: **covered**. input-only JSON block is used as the fixture for the following recurse query
   - Execution: **match**. jq/tq JSON semantic observations match

138. **code-recurse-input** (line 1185, code-input; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: none
   - Input: `{"name":"/","children":[{"name":"/bin","children":[{"name":"/bin/ls","children":[]},{"name":"/bin/sh","children":[]}]},{"name":"/home","children":[{"name":"/home/stephen","children":[{"name":"/home/stephen/jq","children":[]}]}]}]}`
   - Case ID(s): none
   - Status: **non-executable**. fixture-only JSON block; exercised by the following recurse query
   - Execution: **not-run**. fixture-only JSON block; exercised by the following recurse query

139. **fence-recurse-query** (line 1196, fenced-block; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(.children[]) | .name`
   - Input: `{"name":"/","children":[{"name":"/bin","children":[{"name":"/bin/ls","children":[]},{"name":"/bin/sh","children":[]}]},{"name":"/home","children":[{"name":"/home/stephen","children":[{"name":"/home/stephen/jq","children":[]}]}]}]}`
   - Expected output: `/ | /bin | /bin/ls | /bin/sh | /home | /home/stephen | /home/stephen/jq`
   - Case ID(s): `manual-bof-code-recurse-query`
   - Status: **covered**. exact executable query and fixture are covered by the listed case
   - Execution: **match**. jq/tq JSON semantic observations match

140. **code-recurse-query** (line 1197, code; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(.children[]) | .name`
   - Input: `{"name":"/","children":[{"name":"/bin","children":[{"name":"/bin/ls","children":[]},{"name":"/bin/sh","children":[]}]},{"name":"/home","children":[{"name":"/home/stephen","children":[{"name":"/home/stephen/jq","children":[]}]}]}]}`
   - Expected output: `/ | /bin | /bin/ls | /bin/sh | /home | /home/stephen | /home/stephen/jq`
   - Case ID(s): `manual-bof-code-recurse-query`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

141. **prose-recurse-equivalent** (line 1200, prose-signature; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(.[]?)`
   - Input: `{"a":[1]}`
   - Expected output: `{"a":[1]} | [1] | 1`
   - Case ID(s): `manual-bof-synth-recurse-default`
   - Status: **new**. synthesized representative recursive input
   - Execution: **match**. jq/tq JSON semantic observations match

142. **prose-recurse-identical** (line 1202, prose-signature; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(f; true)`
   - Input: none
   - Case ID(s): none
   - Status: **non-executable**. placeholder f and equivalence statement, no concrete input/output example
   - Execution: **not-run**. placeholder f and equivalence statement, no concrete input/output example

143. **prose-recurse-unbounded** (line 1204, prose-query; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(.+1; true)`
   - Case query: `limit(3; recurse(.+1; true))`
   - Input: `0`
   - Expected output: `0 | 1 | 2`
   - Case ID(s): `manual-bof-synth-recurse-bounded`
   - Status: **new**. the unbounded source query is exercised through bounded case query `limit(3; recurse(.+1; true))`; see case_query
   - Execution: **match**. jq/tq JSON semantic observations match

144. **table-125** (line 1208, table; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(.foo[])`
   - Input: `{"foo":[{"foo": []}, {"foo":[{"foo":[]}]}]}`
   - Expected output: `{"foo":[{"foo":[]},{"foo":[{"foo":[]}]}]} | {"foo":[]} | {"foo":[{"foo":[]}]} | {"foo":[]}`
   - Case ID(s): `manual-bof-table-125`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

145. **table-126** (line 1217, table; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse`
   - Input: `{"a":0,"b":[1]}`
   - Expected output: `{"a":0,"b":[1]} | 0 | [1] | 1`
   - Case ID(s): `manual-bof-table-126`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

146. **table-127** (line 1226, table; heading `\`recurse(f)\`, \`recurse\`, \`recurse(f; condition)\``)
   - Query: `recurse(. * .; . < 20)`
   - Input: `2`
   - Expected output: `2 | 4 | 16`
   - Case ID(s): `manual-bof-table-127`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

147. **table-128** (line 1238, table; heading `\`walk(f)\``)
   - Query: `walk(if type == "array" then sort else . end)`
   - Input: `[[4, 1, 7], [8, 5, 2], [3, 6, 9]]`
   - Expected output: `[[1,4,7],[2,5,8],[3,6,9]]`
   - Case ID(s): `manual-bof-table-128`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

148. **table-129** (line 1244, table; heading `\`walk(f)\``)
   - Query: `walk( if type == "object" then with_entries( .key |= sub( "^_+"; "") ) else . end )`
   - Input: `[ { "_a": { "__b": 2 } } ]`
   - Expected output: `[{"a":{"b":2}}]`
   - Case ID(s): `manual-bof-table-129`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 2 (cli-usage)

149. **table-130** (line 1272, table; heading `\`$ENV\`, \`env\``)
   - Query: `$ENV.PAGER`
   - Input: `null`
   - Expected output: `"less"`
   - Case ID(s): `manual-bof-table-130`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

150. **table-131** (line 1278, table; heading `\`$ENV\`, \`env\``)
   - Query: `env.PAGER`
   - Input: `null`
   - Expected output: `"less"`
   - Case ID(s): `manual-bof-table-131`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

151. **table-132** (line 1288, table; heading `\`transpose\``)
   - Query: `transpose`
   - Input: `[[1], [2,3]]`
   - Expected output: `[[1,2],[null,3]]`
   - Case ID(s): `manual-bof-table-132`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

152. **table-133** (line 1298, table; heading `\`bsearch(x)\``)
   - Query: `bsearch(0)`
   - Input: `[0,1]`
   - Expected output: `0`
   - Case ID(s): `manual-bof-table-133`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

153. **table-134** (line 1304, table; heading `\`bsearch(x)\``)
   - Query: `bsearch(0)`
   - Input: `[1,2,3]`
   - Expected output: `-1`
   - Case ID(s): `manual-bof-table-134`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

154. **table-135** (line 1310, table; heading `\`bsearch(x)\``)
   - Query: `bsearch(4) as $ix | if $ix < 0 then .[-(1+$ix)] = 4 else . end`
   - Input: `[1,2,3]`
   - Expected output: `[1,2,3,4]`
   - Case ID(s): `manual-bof-table-135`
   - Status: **new**.
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

155. **table-136** (line 1320, table; heading `String interpolation: \`\(exp)\``)
   - Query: `"The input was \(.), which is one less than \(.+1)"`
   - Input: `42`
   - Expected output: `"The input was 42, which is one less than 43"`
   - Case ID(s): `manual-bof-table-136`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

156. **table-137** (line 1330, table; heading `Convert to/from JSON`)
   - Query: `[.[]|tostring]`
   - Input: `[1, "foo", ["foo"]]`
   - Expected output: `["1","foo","[\"foo\"]"]`
   - Case ID(s): `manual-bof-table-137`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

157. **table-138** (line 1336, table; heading `Convert to/from JSON`)
   - Query: `[.[]|tojson]`
   - Input: `[1, "foo", ["foo"]]`
   - Expected output: `["1","\"foo\"","[\"foo\"]"]`
   - Case ID(s): `manual-bof-table-138`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

158. **table-139** (line 1342, table; heading `Convert to/from JSON`)
   - Query: `[.[]|tojson|fromjson]`
   - Input: `[1, "foo", ["foo"]]`
   - Expected output: `[1,"foo",["foo"]]`
   - Case ID(s): `manual-bof-table-139`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

159. **prose-format-text** (line 1352, prose-signature; heading `Format strings and escaping`)
   - Query: `@text`
   - Input: `1`
   - Expected output: `"1"`
   - Case ID(s): `manual-bof-synth-format-text`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

160. **prose-format-json** (line 1356, prose-signature; heading `Format strings and escaping`)
   - Query: `@json`
   - Input: `"x"`
   - Expected output: `"\"x\""`
   - Case ID(s): `manual-bof-synth-format-json`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

161. **prose-format-html** (line 1360, prose-signature; heading `Format strings and escaping`)
   - Query: `@html`
   - Input: `"<>&'\""`
   - Expected output: `"&lt;&gt;&amp;&apos;&quot;"`
   - Case ID(s): `manual-bof-synth-format-html`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

162. **prose-format-uri** (line 1364, prose-signature; heading `Format strings and escaping`)
   - Query: `@uri`
   - Input: `"a b"`
   - Expected output: `"a%20b"`
   - Case ID(s): `manual-bof-synth-format-uri`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

163. **prose-format-urid** (line 1368, prose-signature; heading `Format strings and escaping`)
   - Query: `@urid`
   - Input: `"a%20b"`
   - Expected output: `"a b"`
   - Case ID(s): `manual-bof-synth-format-urid`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

164. **prose-format-csv** (line 1372, prose-signature; heading `Format strings and escaping`)
   - Query: `@csv`
   - Input: `["a","b"]`
   - Expected output: `"\"a\",\"b\""`
   - Case ID(s): `manual-bof-synth-format-csv`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

165. **prose-format-tsv** (line 1376, prose-signature; heading `Format strings and escaping`)
   - Query: `@tsv`
   - Input: `["a","b"]`
   - Expected output: `"a\tb"`
   - Case ID(s): `manual-bof-synth-format-tsv`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

166. **prose-format-sh** (line 1380, prose-signature; heading `Format strings and escaping`)
   - Query: `@sh`
   - Input: `"a b"`
   - Expected output: `"'a b'"`
   - Case ID(s): `manual-bof-synth-format-sh`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

167. **prose-format-base64** (line 1384, prose-signature; heading `Format strings and escaping`)
   - Query: `@base64`
   - Input: `"hello"`
   - Expected output: `"aGVsbG8="`
   - Case ID(s): `manual-bof-synth-format-base64`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

168. **prose-format-base64d** (line 1388, prose-signature; heading `Format strings and escaping`)
   - Query: `@base64d`
   - Input: `"aGVsbG8="`
   - Expected output: `"hello"`
   - Case ID(s): `manual-bof-synth-format-base64d`
   - Status: **new**. synthesized representative input for the format-token definition
   - Execution: **match**. jq/tq JSON semantic observations match

169. **fence-format-query** (line 1394, fenced-block; heading `Format strings and escaping`)
   - Query: `@uri "https://www.google.com/search?q=\(.search)"`
   - Input: `{"search":"what is jq?"}`
   - Expected output: `"https://www.google.com/search?q=what%20is%20jq%3F"`
   - Case ID(s): `manual-bof-code-format-uri`
   - Status: **covered**. exact executable query and fixture are covered by the listed case
   - Execution: **match**. jq/tq JSON semantic observations match

170. **code-format-uri** (line 1395, code; heading `Format strings and escaping`)
   - Query: `@uri "https://www.google.com/search?q=\(.search)"`
   - Input: `{"search":"what is jq?"}`
   - Expected output: `"https://www.google.com/search?q=what%20is%20jq%3F"`
   - Case ID(s): `manual-bof-code-format-uri`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

171. **fence-format-output** (line 1400, fenced-block; heading `Format strings and escaping`)
   - Query: none
   - Input: none
   - Expected output: `"https://www.google.com/search?q=what%20is%20jq%3F"`
   - Case ID(s): `manual-bof-code-format-uri`
   - Status: **non-executable**. output-only fenced block; covered as the expected output of the preceding query case
   - Execution: **not-run**. output-only fenced block; covered as the expected output of the preceding query case

172. **table-140** (line 1406, table; heading `Format strings and escaping`)
   - Query: `@html`
   - Input: `"This works if x < y"`
   - Expected output: `"This works if x &lt; y"`
   - Case ID(s): `manual-bof-table-140`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

173. **table-141** (line 1412, table; heading `Format strings and escaping`)
   - Query: `@sh "echo \(.)"`
   - Input: `"O'Hara's Ale"`
   - Expected output: `"echo 'O'\\''Hara'\\''s Ale'"`
   - Case ID(s): `manual-bof-table-141`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

174. **table-142** (line 1418, table; heading `Format strings and escaping`)
   - Query: `@base64`
   - Input: `"This is a message"`
   - Expected output: `"VGhpcyBpcyBhIG1lc3NhZ2U="`
   - Case ID(s): `manual-bof-table-142`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

175. **table-143** (line 1424, table; heading `Format strings and escaping`)
   - Query: `@base64d`
   - Input: `"VGhpcyBpcyBhIG1lc3NhZ2U="`
   - Expected output: `"This is a message"`
   - Case ID(s): `manual-bof-table-143`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

176. **table-144** (line 1458, table; heading `Dates`)
   - Query: `fromdate`
   - Input: `"2015-03-05T23:51:47Z"`
   - Expected output: `1425599507`
   - Case ID(s): `manual-bof-table-144`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

177. **table-145** (line 1464, table; heading `Dates`)
   - Query: `strptime("%Y-%m-%dT%H:%M:%SZ")`
   - Input: `"2015-03-05T23:51:47Z"`
   - Expected output: `[2015,2,5,23,51,47,4,63]`
   - Case ID(s): `date.strptime`
   - Status: **covered**. exact query and fixture match existing compatibility case
   - Execution: **match**. jq/tq JSON semantic observations match

178. **table-146** (line 1470, table; heading `Dates`)
   - Query: `strptime("%Y-%m-%dT%H:%M:%SZ")|mktime`
   - Input: `"2015-03-05T23:51:47Z"`
   - Expected output: `1425599507`
   - Case ID(s): `manual-bof-table-146`
   - Status: **new**.
   - Execution: **match**. jq/tq JSON semantic observations match

179. **prose-sql-1480** (line 1480, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `INDEX(stream; index_expression)`
   - Query: `INDEX(.[]; .id)`
   - Input: `[{"id":"a","v":1},{"id":"b","v":2}]`
   - Expected output: `{"a":{"id":"a","v":1},"b":{"id":"b","v":2}}`
   - Case ID(s): `manual-bof-synth-sql-index`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

180. **prose-sql-1484** (line 1484, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `JOIN($idx; stream; idx_expr; join_expr)`
   - Query: `INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id; .)`
   - Input: `[{"id":"a","v":1},{"id":"b","v":2}]`
   - Expected output: `[{"id":"a","v":1},{"id":"a","v":1}] | [{"id":"b","v":2},{"id":"b","v":2}]`
   - Case ID(s): `manual-bof-synth-sql-join4`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

181. **prose-sql-1488** (line 1488, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `JOIN($idx; stream; idx_expr)`
   - Query: `INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id)`
   - Input: `[{"id":"a","v":1},{"id":"b","v":2}]`
   - Expected output: `[{"id":"a","v":1},{"id":"a","v":1}] | [{"id":"b","v":2},{"id":"b","v":2}]`
   - Case ID(s): `manual-bof-synth-sql-join3`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

182. **prose-sql-1492** (line 1492, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `JOIN($idx; idx_expr)`
   - Query: `JOIN({"a":{"id":"a","v":1}}; .id)`
   - Input: `{"id":"a"}`
   - Case ID(s): `manual-bof-synth-sql-join2`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

183. **prose-sql-1496** (line 1496, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `IN(s)`
   - Query: `IN(["a","b"][])`
   - Input: `"b"`
   - Expected output: `true`
   - Case ID(s): `manual-bof-synth-sql-in1`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

184. **prose-sql-1500** (line 1500, prose-signature; heading `SQL-Style Operators`)
   - Source signature: `IN(source; s)`
   - Query: `IN(.[]; ["b"][])`
   - Input: `["a","b"]`
   - Expected output: `true`
   - Case ID(s): `manual-bof-synth-sql-in2`
   - Status: **new**. synthesized representative input and executable query for the signature
   - Execution: **divergent**. tq JSON exited 3 (query-compile)

185. **prose-builtins-arity** (line 1506, prose-signature; heading `\`builtins\``)
   - Query: `all/0, all/1, all/2`
   - Input: none
   - Case ID(s): none
   - Status: **non-executable**. arity illustration only; no concrete input/output example
   - Execution: **not-run**. arity illustration only; no concrete input/output example

## Limitations

- The repository-wide manual gate was not green in the worktree during this run because it still reported missing examples in other manual sections. The section-level source-to-catalog check for all 146 table examples passed.
- The manual's join(" ") output at source line 1122 is retained verbatim in the oracle ledger. jq-1.8.1 emits two spaces before `false` because null joins as an empty field; this is a source-oracle discrepancy, not silently normalized.
- `$ENV.PAGER` and `env.PAGER` use a per-case `PAGER=less` environment override, plus tq environment admission, so the published example is reproducible.
- The unbounded `recurse(.+1; true)` prose query is recorded with a bounded `limit(3; ...)` case query. The original source query and the adaptation are both retained.

<!-- tq-manual-compare:begin section=builtin-operators-and-functions -->
## Results

[Case collection](../../../tests/compatibility/reviews/jq-manual/builtin-operators-and-functions.toon)


| Verdict | Cases |
| --- | ---: |
| match | 229 |

Independent output campaigns must pass too. Compact JSON compares exact stdout bytes and process behavior; TOON compares ordered values and process behavior with the JSON execution.

| Output campaign | Matches | Cases |
| --- | ---: | ---: |
| compact_json | 227 | 227 |
| toon | 227 | 227 |

A match requires equivalent JSON results and process behavior, or a matching non-JSON CLI contract. Reviewed disparities retain exact observations and count separately from matches. Historical expected-difference labels do not pass either gate.

Missing features and unaccepted mismatches remain failures. Reference discrepancies describe errors in the imported manual, not successful compatibility.

JSON equivalence ignores whitespace and object key order but retains array and result-sequence order. Error-only cases do not count as JSON matches or size samples. Raw CLI cases keep their original arguments and have no JSON/TOON size measurement.

### Output size

225 eligible examples. Counts use the `o200k_base` and `cl100k_base` tokenizers over complete stdout, including trailing newlines. The totals compare default `-o json` output with default LF-terminated `-o toon` results; explicitly requested `--seq -o toon` output is shown in the cases but excluded from size totals. Diff is TOON tokens minus JSON tokens. % is the signed percent difference `(TOON - JSON) / JSON`, so savings are negative and growth is positive.

| Tokenizer | JSON tokens | TOON tokens | Diff | % |
| --- | ---: | ---: | ---: | ---: |
| `o200k_base` | 2950 | 2086 | -864 | -29.29% |
| `cl100k_base` | 2952 | 2089 | -863 | -29.23% |

Only successful jq/JSON/TOON-equivalent results enter the totals. A negative `Diff` means TOON uses fewer tokens; `%` is negative for savings and positive for growth. The manual is a correctness corpus, not a representative workload benchmark.

### Reviewed disparities and historical differences

| Case | Reason |
| --- | --- |

### Cases

Each case shows the original jq invocation, then the complete jq, tq JSON, and tq TOON output. Control bytes use `\xNN` escapes so record separators remain visible. This section is generated by the separate `tq-manual-compare` command.

#### builtin.all

```
# input
jq 'all(.[]; . < 4)'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.any

```
# input
jq 'any(.[]; . > 2)'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.arrays

```
# input
jq '.[] | arrays'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### builtin.ascii-downcase

```
# input
jq ascii_downcase

# jq
"abcé"

# tq -o json
"abcé"

# tq
abcé
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 3 | -1 | -25% |
| `cl100k_base` | 4 | 4 | 3 | -1 | -25% |

#### builtin.booleans

```
# input
jq '.[] | booleans'

# jq
false
true

# tq -o json
false
true

# tq
false
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### builtin.empty

```
# input
jq empty

# jq

# tq -o json

# tq
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### builtin.error

```
# input
jq 'error("boom")'

# jq


[stderr]
jq: error (at <stdin>:0): boom

# tq -o json


[stderr]
tq: runtime error: boom

# tq


[stderr]
tq: runtime error: boom
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### builtin.explode

```
# input
jq explode

# jq
[
  65,
  233
]

# tq -o json
[
  65,
  233
]

# tq
[2]: 65,233
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### builtin.fromjson

```
# input
jq fromjson

# jq
{
  "a": 1
}

# tq -o json
{
  "a": 1
}

# tq
a: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### builtin.getpath

```
# input
jq 'getpath(["a",0])'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.group-by

```
# input
jq 'group_by(.k)'

# jq
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq -o json
[
  [
    {
      "k": 1
    }
  ],
  [
    {
      "k": 2
    },
    {
      "k": 2
    }
  ]
]

# tq
[2]:
  - [1]{k}:
    1
  - [2]{k}:
    2
    2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 43 | 43 | 31 | -12 | -27.91% |
| `cl100k_base` | 43 | 43 | 31 | -12 | -27.91% |

#### builtin.has

```
# input
jq '[has("a"),has("b")]'

# jq
[
  true,
  false
]

# tq -o json
[
  true,
  false
]

# tq
[2]: true,false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### builtin.implode

```
# input
jq implode

# jq
"Aé"

# tq -o json
"Aé"

# tq
Aé
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### builtin.in

```
# input
jq 'in({"a":1})'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.iterables

```
# input
jq '.[] | iterables'

# jq
[]
{}

# tq -o json
[]
{}

# tq
[0]:

```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 3 | +1 | +50% |
| `cl100k_base` | 2 | 2 | 3 | +1 | +50% |

#### builtin.keys

```
# input
jq keys

# jq
[
  "10",
  "2",
  "a",
  "z"
]

# tq -o json
[
  "10",
  "2",
  "a",
  "z"
]

# tq
[4]: "10","2",a,z
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 11 | -7 | -38.89% |
| `cl100k_base` | 18 | 18 | 11 | -7 | -38.89% |

#### builtin.keys-unsorted

```
# input
jq keys_unsorted

# jq
[
  "z",
  "a"
]

# tq -o json
[
  "z",
  "a"
]

# tq
[2]: z,a
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 6 | -4 | -40% |
| `cl100k_base` | 10 | 10 | 6 | -4 | -40% |

#### builtin.length

```
# input
jq 'map(length)'

# jq
[
  2,
  1,
  1,
  0
]

# tq -o json
[
  2,
  1,
  1,
  0
]

# tq
[4]: 2,1,1,0
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### builtin.ltrimstr

```
# input
jq 'ltrimstr("pre")'

# jq
"fix"

# tq -o json
"fix"

# tq
fix
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### builtin.map

```
# input
jq 'map(. * 2)'

# jq
[
  2,
  4,
  6
]

# tq -o json
[
  2,
  4,
  6
]

# tq
[3]: 2,4,6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### builtin.map-values

```
# input
jq 'map_values(. + 1)'

# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### builtin.max

```
# input
jq max

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.max-by

```
# input
jq 'max_by(.k)'

# jq
{
  "k": 2
}

# tq -o json
{
  "k": 2
}

# tq
k: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### builtin.min

```
# input
jq min

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.min-by

```
# input
jq 'min_by(.k)'

# jq
{
  "k": 1
}

# tq -o json
{
  "k": 1
}

# tq
k: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 5 | -4 | -44.44% |
| `cl100k_base` | 9 | 9 | 5 | -4 | -44.44% |

#### builtin.nulls

```
# input
jq '.[] | nulls'

# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.numbers

```
# input
jq '.[] | numbers'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.objects

```
# input
jq '.[] | objects'

# jq
{}

# tq -o json
{}

# tq

```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 1 | 0 | +0% |
| `cl100k_base` | 1 | 1 | 1 | 0 | +0% |

#### builtin.path

```
# input
jq 'path(.a[0])'

# jq
[
  "a",
  0
]

# tq -o json
[
  "a",
  0
]

# tq
[2]: a,0
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 7 | -3 | -30% |
| `cl100k_base` | 10 | 10 | 7 | -3 | -30% |

#### builtin.reverse

```
# input
jq reverse

# jq
[
  3,
  2,
  1
]

# tq -o json
[
  3,
  2,
  1
]

# tq
[3]: 3,2,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### builtin.scalars

```
# input
jq '.[] | scalars'

# jq
null
false
0
"x"

# tq -o json
null
false
0
"x"

# tq
null
false
0
x
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### builtin.select

```
# input
jq '.[] | select(. % 2 == 0)'

# jq
2
4

# tq -o json
2
4

# tq
2
4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### builtin.setpath

```
# input
jq 'setpath(["a",1];7)'

# jq
{
  "a": [
    null,
    7
  ]
}

# tq -o json
{
  "a": [
    null,
    7
  ]
}

# tq
a[2]: null,7
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 8 | -8 | -50% |
| `cl100k_base` | 16 | 16 | 8 | -8 | -50% |

#### builtin.sort

```
# input
jq sort

# jq
[
  null,
  false,
  3,
  "x",
  []
]

# tq -o json
[
  null,
  false,
  3,
  "x",
  []
]

# tq
[5]:
  - null
  - false
  - 3
  - x
  - [0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 25 | +7 | +38.89% |
| `cl100k_base` | 18 | 18 | 25 | +7 | +38.89% |

#### builtin.sort-by

```
# input
jq 'sort_by(.n)'

# jq
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq -o json
[
  {
    "n": 1
  },
  {
    "n": 2
  },
  {
    "n": 2
  }
]

# tq
[3]{n}:
  1
  2
  2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 18 | -17 | -48.57% |
| `cl100k_base` | 35 | 35 | 18 | -17 | -48.57% |

#### builtin.strings

```
# input
jq '.[] | strings'

# jq
"x"

# tq -o json
"x"

# tq
x
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.to-entries

```
# input
jq to_entries

# jq
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq -o json
[
  {
    "key": "z",
    "value": 1
  },
  {
    "key": "a",
    "value": 2
  }
]

# tq
[2]{key,value}:
  z,1
  a,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 17 | -21 | -55.26% |
| `cl100k_base` | 38 | 38 | 17 | -21 | -55.26% |

#### builtin.tojson

```
# input
jq tojson

# jq
"{\"a\":1}"

# tq -o json
"{\"a\":1}"

# tq
"{\"a\":1}"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### builtin.tonumber

```
# input
jq 'map(tonumber)'

# jq
[
  -2,
  1.5,
  1E+3
]

# tq -o json
[
  -2,
  1.5,
  1E+3
]

# tq
[3]: -2,1.5,1000
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 13 | -6 | -31.58% |
| `cl100k_base` | 19 | 19 | 13 | -6 | -31.58% |

#### builtin.tostring

```
# input
jq 'map(tostring)'

# jq
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq -o json
[
  "null",
  "true",
  "1",
  "x",
  "[2]"
]

# tq
[5]: "null","true","1",x,"[2]"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 15 | -7 | -31.82% |
| `cl100k_base` | 22 | 22 | 15 | -7 | -31.82% |

#### builtin.type

```
# input
jq 'map(type)'

# jq
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq -o json
[
  "null",
  "boolean",
  "number",
  "string",
  "array",
  "object"
]

# tq
[6]: "null",boolean,number,string,array,object
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 14 | -12 | -46.15% |
| `cl100k_base` | 26 | 26 | 13 | -13 | -50% |

#### builtin.unique

```
# input
jq unique

# jq
[
  1,
  2,
  3
]

# tq -o json
[
  1,
  2,
  3
]

# tq
[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### builtin.unique-by-generator

```
# input
jq 'unique_by(.a,.b)'

# jq
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq -o json
[
  {
    "a": 1,
    "b": 1,
    "id": "first"
  },
  {
    "a": 1,
    "b": 2,
    "id": "distinct"
  }
]

# tq
[2]{a,b,id}:
  1,1,first
  1,2,distinct
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 52 | 52 | 24 | -28 | -53.85% |
| `cl100k_base` | 52 | 52 | 25 | -27 | -51.92% |

#### builtin.utf8bytelength

```
# input
jq utf8bytelength

# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### builtin.values

```
# input
jq '.[] | values'

# jq
false
0
"x"

# tq -o json
false
0
"x"

# tq
false
0
x
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### builtin.with-entries

```
# input
jq 'with_entries(.value += 1)'

# jq
{
  "a": 2,
  "b": 3
}

# tq -o json
{
  "a": 2,
  "b": 3
}

# tq
a: 2
b: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### date.strftime

```
# input
jq 'gmtime | strftime("%Y-%m-%dT%H:%M:%SZ")'

# jq
"2015-03-05T23:51:47Z"

# tq -o json
"2015-03-05T23:51:47Z"

# tq
"2015-03-05T23:51:47Z"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 | +0% |
| `cl100k_base` | 15 | 15 | 15 | 0 | +0% |

#### date.strptime

```
# input
jq 'strptime("%Y-%m-%dT%H:%M:%SZ")'

# jq
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq -o json
[
  2015,
  2,
  5,
  23,
  51,
  47,
  4,
  63
]

# tq
[8]: 2015,2,5,23,51,47,4,63
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 35 | 35 | 21 | -14 | -40% |
| `cl100k_base` | 35 | 35 | 21 | -14 | -40% |

#### date.utc-roundtrip

```
# input
jq '[todateiso8601, gmtime, (gmtime | mktime)]'

# jq
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq -o json
[
  "2015-03-05T23:51:47Z",
  [
    2015,
    2,
    5,
    23,
    51,
    47,
    4,
    63
  ],
  1425599507
]

# tq
[3]:
  - "2015-03-05T23:51:47Z"
  - [8]: 2015,2,5,23,51,47,4,63
  - 1425599507
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 62 | 62 | 51 | -11 | -17.74% |
| `cl100k_base` | 62 | 62 | 51 | -11 | -17.74% |

#### manual-bof-code-format-uri

```
# input
jq '@uri "https://www.google.com/search?q=\(.search)"'

# jq
"https://www.google.com/search?q=what%20is%20jq%3F"

# tq -o json
"https://www.google.com/search?q=what%20is%20jq%3F"

# tq
"https://www.google.com/search?q=what%20is%20jq%3F"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 19 | 19 | 19 | 0 | +0% |
| `cl100k_base` | 19 | 19 | 19 | 0 | +0% |

#### manual-bof-code-map-001

```
# input
jq 'map(.+1)'

# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-code-map-002

```
# input
jq 'map(., .)'

# jq
[
  1,
  1
]

# tq -o json
[
  1,
  1
]

# tq
[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual-bof-code-map-003

```
# input
jq 'map(empty)'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual-bof-code-map-004

```
# input
jq 'map_values(.+1)'

# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-code-map-005

```
# input
jq 'map_values(., .)'

# jq
[
  1
]

# tq -o json
[
  1
]

# tq
[1]: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-code-map-006

```
# input
jq 'map_values(empty)'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual-bof-code-recurse-query

```
# input
jq 'recurse(.children[]) | .name'

# jq
"/"
"/bin"
"/bin/ls"
"/bin/sh"
"/home"
"/home/stephen"
"/home/stephen/jq"

# tq -o json
"/"
"/bin"
"/bin/ls"
"/bin/sh"
"/home"
"/home/stephen"
"/home/stephen/jq"

# tq
/
/bin
/bin/ls
/bin/sh
/home
/home/stephen
/home/stephen/jq
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 31 | 31 | 24 | -7 | -22.58% |
| `cl100k_base` | 31 | 31 | 24 | -7 | -22.58% |

#### manual-bof-prose-halt-error

```
# input
jq '"Error: something went wrong\n"|halt_error(1)'

# jq


[stderr]
Error: something went wrong

# tq -o json


[stderr]
Error: something went wrong

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a | n/a |

#### manual-bof-prose-map-select

```
# input
jq '[1,2,3] | map(select(. >= 2))'

# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual-bof-synth-format-base64

```
# input
jq '@base64'

# jq
"aGVsbG8="

# tq -o json
"aGVsbG8="

# tq
aGVsbG8=
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-synth-format-base64d

```
# input
jq '@base64d'

# jq
"hello"

# tq -o json
"hello"

# tq
hello
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual-bof-synth-format-csv

```
# input
jq '@csv'

# jq
"\"a\",\"b\""

# tq -o json
"\"a\",\"b\""

# tq
"\"a\",\"b\""
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 7 | 0 | +0% |
| `cl100k_base` | 7 | 7 | 7 | 0 | +0% |

#### manual-bof-synth-format-html

```
# input
jq '@html'

# jq
"&lt;&gt;&amp;&apos;&quot;"

# tq -o json
"&lt;&gt;&amp;&apos;&quot;"

# tq
&lt;&gt;&amp;&apos;&quot;
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 10 | -1 | -9.09% |
| `cl100k_base` | 11 | 11 | 10 | -1 | -9.09% |

#### manual-bof-synth-format-json

```
# input
jq '@json'

# jq
"\"x\""

# tq -o json
"\"x\""

# tq
"\"x\""
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual-bof-synth-format-sh

```
# input
jq '@sh'

# jq
"'a b'"

# tq -o json
"'a b'"

# tq
'a b'
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 3 | -1 | -25% |
| `cl100k_base` | 4 | 4 | 3 | -1 | -25% |

#### manual-bof-synth-format-text

```
# input
jq '@text'

# jq
"1"

# tq -o json
"1"

# tq
"1"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### manual-bof-synth-format-tsv

```
# input
jq '@tsv'

# jq
"a\tb"

# tq -o json
"a\tb"

# tq
"a\tb"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-synth-format-uri

```
# input
jq '@uri'

# jq
"a%20b"

# tq -o json
"a%20b"

# tq
a%20b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual-bof-synth-format-urid

```
# input
jq '@urid'

# jq
"a b"

# tq -o json
"a b"

# tq
a b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### manual-bof-synth-path-boolean

```
# input
jq 'path(..|select(type=="boolean"))'

# jq
[
  "a",
  0
]
[
  "a",
  1
]

# tq -o json
[
  "a",
  0
]
[
  "a",
  1
]

# tq
[2]: a,0
[2]: a,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 14 | -6 | -30% |
| `cl100k_base` | 20 | 20 | 14 | -6 | -30% |

#### manual-bof-synth-recurse-bounded

```
# input
jq 'limit(3; recurse(.+1; true))'

# jq
0
1
2

# tq -o json
0
1
2

# tq
0
1
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-synth-recurse-default

```
# input
jq 'recurse(.[]?)'

# jq
{
  "a": [
    1
  ]
}
[
  1
]
1

# tq -o json
{
  "a": [
    1
  ]
}
[
  1
]
1

# tq
a[1]: 1
[1]: 1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 15 | -6 | -28.57% |
| `cl100k_base` | 21 | 21 | 15 | -6 | -28.57% |

#### manual-bof-synth-sql-in1

```
# input
jq 'IN(["a","b"][])'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-synth-sql-in2

```
# input
jq 'IN(.[]; ["b"][])'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-synth-sql-index

```
# input
jq 'INDEX(.[]; .id)'

# jq
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq -o json
{
  "a": {
    "id": "a",
    "v": 1
  },
  "b": {
    "id": "b",
    "v": 2
  }
}

# tq
a:
  id: a
  v: 1
b:
  id: b
  v: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 44 | 44 | 26 | -18 | -40.91% |
| `cl100k_base` | 44 | 44 | 26 | -18 | -40.91% |

#### manual-bof-synth-sql-join2

```
# input
jq 'JOIN({"a":{"id":"a","v":1}}; .id)'

# jq


[stderr]
jq: error (at <stdin>:0): Cannot index string with string "id"

# tq -o json


[stderr]
tq: runtime error: field access cannot be applied to string

# tq


[stderr]
tq: runtime error: field access cannot be applied to string
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | 0 | 0 | n/a |
| `cl100k_base` | 0 | 0 | 0 | 0 | n/a |

#### manual-bof-synth-sql-join3

```
# input
jq 'INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id)'

# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq
[2]{id,v}:
  a,1
  a,1
[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 34 | -42 | -55.26% |
| `cl100k_base` | 76 | 76 | 34 | -42 | -55.26% |

#### manual-bof-synth-sql-join4

```
# input
jq 'INDEX(.[]; .id) as $idx | JOIN($idx; .[]; .id; .)'

# jq
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq -o json
[
  {
    "id": "a",
    "v": 1
  },
  {
    "id": "a",
    "v": 1
  }
]
[
  {
    "id": "b",
    "v": 2
  },
  {
    "id": "b",
    "v": 2
  }
]

# tq
[2]{id,v}:
  a,1
  a,1
[2]{id,v}:
  b,2
  b,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 76 | 76 | 34 | -42 | -55.26% |
| `cl100k_base` | 76 | 76 | 34 | -42 | -55.26% |

#### manual-bof-table-001

```
# input
jq '.a + 1'

# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-002

```
# input
jq '.a + .b'

# jq
[
  1,
  2,
  3,
  4
]

# tq -o json
[
  1,
  2,
  3,
  4
]

# tq
[4]: 1,2,3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-003

```
# input
jq '.a + null'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-004

```
# input
jq '.a + 1'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-005

```
# input
jq '{a: 1} + {b: 2} + {c: 3} + {a: 42}'

# jq
{
  "a": 42,
  "b": 2,
  "c": 3
}

# tq -o json
{
  "a": 42,
  "b": 2,
  "c": 3
}

# tq
a: 42
b: 2
c: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 23 | 23 | 15 | -8 | -34.78% |
| `cl100k_base` | 23 | 23 | 15 | -8 | -34.78% |

#### manual-bof-table-006

```
# input
jq '4 - .a'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-007

```
# input
jq '. - ["xml", "yaml"]'

# jq
[
  "json"
]

# tq -o json
[
  "json"
]

# tq
[1]: json
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | 6 | 6 | 5 | -1 | -16.67% |

#### manual-bof-table-008

```
# input
jq '10 / . * 3'

# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-009

```
# input
jq '. / ", "'

# jq
[
  "a",
  "b,c,d",
  "e"
]

# tq -o json
[
  "a",
  "b,c,d",
  "e"
]

# tq
[3]: a,"b,c,d",e
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 11 | -5 | -31.25% |
| `cl100k_base` | 16 | 16 | 11 | -5 | -31.25% |

#### manual-bof-table-010

```
# input
jq '{"k": {"a": 1, "b": 2}} * {"k": {"a": 0,"c": 3}}'

# jq
{
  "k": {
    "a": 0,
    "b": 2,
    "c": 3
  }
}

# tq -o json
{
  "k": {
    "a": 0,
    "b": 2,
    "c": 3
  }
}

# tq
k:
  a: 0
  b: 2
  c: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 20 | -10 | -33.33% |
| `cl100k_base` | 30 | 30 | 20 | -10 | -33.33% |

#### manual-bof-table-011

```
# input
jq '.[] | (1 / .)?'

# jq
1
-1

# tq -o json
1
-1

# tq
1
-1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual-bof-table-012

```
# input
jq 'map(abs)'

# jq
[
  10,
  1.1,
  0.1
]

# tq -o json
[
  10,
  1.1,
  0.1
]

# tq
[3]: 10,1.1,0.1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 14 | -4 | -22.22% |
| `cl100k_base` | 18 | 18 | 14 | -4 | -22.22% |

#### manual-bof-table-013

```
# input
jq '.[] | length'

# jq
2
6
1
0
5

# tq -o json
2
6
1
0
5

# tq
2
6
1
0
5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 10 | 0 | +0% |
| `cl100k_base` | 10 | 10 | 10 | 0 | +0% |

#### manual-bof-table-014

```
# input
jq utf8bytelength

# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-015

```
# input
jq keys

# jq
[
  "Foo",
  "abc",
  "abcd"
]

# tq -o json
[
  "Foo",
  "abc",
  "abcd"
]

# tq
[3]: Foo,abc,abcd
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 9 | -5 | -35.71% |
| `cl100k_base` | 14 | 14 | 9 | -5 | -35.71% |

#### manual-bof-table-016

```
# input
jq keys

# jq
[
  0,
  1,
  2
]

# tq -o json
[
  0,
  1,
  2
]

# tq
[3]: 0,1,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-017

```
# input
jq 'map(has("foo"))'

# jq
[
  true,
  false
]

# tq -o json
[
  true,
  false
]

# tq
[2]: true,false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual-bof-table-018

```
# input
jq 'map(has(2))'

# jq
[
  false,
  true
]

# tq -o json
[
  false,
  true
]

# tq
[2]: false,true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual-bof-table-019

```
# input
jq '.[] | in({"foo": 42})'

# jq
true
false

# tq -o json
true
false

# tq
true
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-table-020

```
# input
jq 'map(in([3,4]))'

# jq
[
  false,
  true
]

# tq -o json
[
  false,
  true
]

# tq
[2]: false,true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual-bof-table-021

```
# input
jq 'map(.+1)'

# jq
[
  2,
  3,
  4
]

# tq -o json
[
  2,
  3,
  4
]

# tq
[3]: 2,3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-022

```
# input
jq 'map_values(.+1)'

# jq
{
  "a": 2,
  "b": 3,
  "c": 4
}

# tq -o json
{
  "a": 2,
  "b": 3,
  "c": 4
}

# tq
a: 2
b: 3
c: 4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 23 | 23 | 15 | -8 | -34.78% |
| `cl100k_base` | 23 | 23 | 15 | -8 | -34.78% |

#### manual-bof-table-023

```
# input
jq 'map(., .)'

# jq
[
  1,
  1,
  2,
  2
]

# tq -o json
[
  1,
  1,
  2,
  2
]

# tq
[4]: 1,1,2,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-024

```
# input
jq 'map_values(. // empty)'

# jq
{
  "b": true
}

# tq -o json
{
  "b": true
}

# tq
b: true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 4 | -4 | -50% |
| `cl100k_base` | 8 | 8 | 4 | -4 | -50% |

#### manual-bof-table-025

```
# input
jq 'pick(.a, .b.c, .x)'

# jq
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq -o json
{
  "a": 1,
  "b": {
    "c": 2
  },
  "x": null
}

# tq
a: 1
b:
  c: 2
x: null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 29 | 29 | 17 | -12 | -41.38% |
| `cl100k_base` | 29 | 29 | 17 | -12 | -41.38% |

#### manual-bof-table-026

```
# input
jq 'pick(.[2], .[0], .[0])'

# jq
[
  1,
  null,
  3
]

# tq -o json
[
  1,
  null,
  3
]

# tq
[3]: 1,null,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 9 | -4 | -30.77% |
| `cl100k_base` | 13 | 13 | 9 | -4 | -30.77% |

#### manual-bof-table-027

```
# input
jq 'path(.a[0].b)'

# jq
[
  "a",
  0,
  "b"
]

# tq -o json
[
  "a",
  0,
  "b"
]

# tq
[3]: a,0,b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 8 | -6 | -42.86% |
| `cl100k_base` | 14 | 14 | 8 | -6 | -42.86% |

#### manual-bof-table-028

```
# input
jq '[path(..)]'

# jq
[
  [],
  [
    "a"
  ],
  [
    "a",
    0
  ],
  [
    "a",
    0,
    "b"
  ]
]

# tq -o json
[
  [],
  [
    "a"
  ],
  [
    "a",
    0
  ],
  [
    "a",
    0,
    "b"
  ]
]

# tq
[4]:
  - [0]:
  - [1]: a
  - [2]: a,0
  - [3]: a,0,b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 34 | -6 | -15% |
| `cl100k_base` | 40 | 40 | 34 | -6 | -15% |

#### manual-bof-table-029

```
# input
jq 'del(.foo)'

# jq
{
  "bar": 9001,
  "baz": 42
}

# tq -o json
{
  "bar": 9001,
  "baz": 42
}

# tq
bar: 9001
baz: 42
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 11 | -6 | -35.29% |
| `cl100k_base` | 17 | 17 | 11 | -6 | -35.29% |

#### manual-bof-table-030

```
# input
jq 'del(.[1, 2])'

# jq
[
  "foo"
]

# tq -o json
[
  "foo"
]

# tq
[1]: foo
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 5 | -1 | -16.67% |
| `cl100k_base` | 6 | 6 | 5 | -1 | -16.67% |

#### manual-bof-table-031

```
# input
jq 'getpath(["a","b"])'

# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-032

```
# input
jq '[getpath(["a","b"], ["a","c"])]'

# jq
[
  0,
  1
]

# tq -o json
[
  0,
  1
]

# tq
[2]: 0,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual-bof-table-033

```
# input
jq 'setpath(["a","b"]; 1)'

# jq
{
  "a": {
    "b": 1
  }
}

# tq -o json
{
  "a": {
    "b": 1
  }
}

# tq
a:
  b: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 8 | -8 | -50% |
| `cl100k_base` | 16 | 16 | 8 | -8 | -50% |

#### manual-bof-table-034

```
# input
jq 'setpath(["a","b"]; 1)'

# jq
{
  "a": {
    "b": 1
  }
}

# tq -o json
{
  "a": {
    "b": 1
  }
}

# tq
a:
  b: 1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 8 | -8 | -50% |
| `cl100k_base` | 16 | 16 | 8 | -8 | -50% |

#### manual-bof-table-035

```
# input
jq 'setpath([0,"a"]; 1)'

# jq
[
  {
    "a": 1
  }
]

# tq -o json
[
  {
    "a": 1
  }
]

# tq
[1]{a}:
  1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 13 | 13 | 10 | -3 | -23.08% |
| `cl100k_base` | 13 | 13 | 10 | -3 | -23.08% |

#### manual-bof-table-036

```
# input
jq 'delpaths([["a","b"]])'

# jq
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq -o json
{
  "a": {},
  "x": {
    "y": 2
  }
}

# tq
a:
x:
  y: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 10 | -11 | -52.38% |
| `cl100k_base` | 21 | 21 | 10 | -11 | -52.38% |

#### manual-bof-table-037

```
# input
jq to_entries

# jq
[
  {
    "key": "a",
    "value": 1
  },
  {
    "key": "b",
    "value": 2
  }
]

# tq -o json
[
  {
    "key": "a",
    "value": 1
  },
  {
    "key": "b",
    "value": 2
  }
]

# tq
[2]{key,value}:
  a,1
  b,2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 17 | -21 | -55.26% |
| `cl100k_base` | 38 | 38 | 17 | -21 | -55.26% |

#### manual-bof-table-038

```
# input
jq from_entries

# jq
{
  "a": 1,
  "b": 2
}

# tq -o json
{
  "a": 1,
  "b": 2
}

# tq
a: 1
b: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### manual-bof-table-039

```
# input
jq 'with_entries(.key |= "KEY_" + .)'

# jq
{
  "KEY_a": 1,
  "KEY_b": 2
}

# tq -o json
{
  "KEY_a": 1,
  "KEY_b": 2
}

# tq
KEY_a: 1
KEY_b: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-040

```
# input
jq 'map(select(. >= 2))'

# jq
[
  5,
  3,
  7
]

# tq -o json
[
  5,
  3,
  7
]

# tq
[3]: 5,3,7
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-041

```
# input
jq '.[] | select(.id == "second")'

# jq
{
  "id": "second",
  "val": 2
}

# tq -o json
{
  "id": "second",
  "val": 2
}

# tq
id: second
val: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 9 | -7 | -43.75% |
| `cl100k_base` | 16 | 16 | 9 | -7 | -43.75% |

#### manual-bof-table-042

```
# input
jq '.[]|numbers'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-043

```
# input
jq '1, empty, 2'

# jq
1
2

# tq -o json
1
2

# tq
1
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-table-044

```
# input
jq '[1,2,empty,3]'

# jq
[
  1,
  2,
  3
]

# tq -o json
[
  1,
  2,
  3
]

# tq
[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-045

```
# input
jq 'try error catch .'

# jq
"error message"

# tq -o json
"error message"

# tq
error message
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 3 | -1 | -25% |
| `cl100k_base` | 4 | 4 | 3 | -1 | -25% |

#### manual-bof-table-046

```
# input
jq 'try error("invalid value: \(.)") catch .'

# jq
"invalid value: 42"

# tq -o json
"invalid value: 42"

# tq
"invalid value: 42"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 7 | 7 | 7 | 0 | +0% |
| `cl100k_base` | 7 | 7 | 7 | 0 | +0% |

#### manual-bof-table-047

```
# input
jq 'try error("\($__loc__)") catch .'

# jq
"{\"file\":\"<top-level>\",\"line\":1}"

# tq -o json
"{\"file\":\"<top-level>\",\"line\":1}"

# tq
"{\"file\":\"<top-level>\",\"line\":1}"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 14 | 0 | +0% |
| `cl100k_base` | 14 | 14 | 14 | 0 | +0% |

#### manual-bof-table-048

```
# input
jq '[paths]'

# jq
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1
  ],
  [
    1,
    0
  ],
  [
    1,
    1
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[5]:
  - [1]: 0
  - [1]: 1
  - [2]: 1,0
  - [2]: 1,1
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 58 | 58 | 50 | -8 | -13.79% |
| `cl100k_base` | 58 | 58 | 50 | -8 | -13.79% |

#### manual-bof-table-049

```
# input
jq '[paths(type == "number")]'

# jq
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq -o json
[
  [
    0
  ],
  [
    1,
    1,
    "a"
  ]
]

# tq
[2]:
  - [1]: 0
  - [3]: 1,1,a
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 22 | -4 | -15.38% |
| `cl100k_base` | 26 | 26 | 22 | -4 | -15.38% |

#### manual-bof-table-050

```
# input
jq add

# jq
"abc"

# tq -o json
"abc"

# tq
abc
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual-bof-table-051

```
# input
jq add

# jq
6

# tq -o json
6

# tq
6
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-052

```
# input
jq add

# jq
null

# tq -o json
null

# tq
null
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-053

```
# input
jq 'add(.[].a)'

# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-054

```
# input
jq any

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-055

```
# input
jq any

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-056

```
# input
jq any

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-057

```
# input
jq all

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-058

```
# input
jq all

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-059

```
# input
jq all

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-060

```
# input
jq flatten

# jq
[
  1,
  2,
  3
]

# tq -o json
[
  1,
  2,
  3
]

# tq
[3]: 1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-061

```
# input
jq 'flatten(1)'

# jq
[
  1,
  2,
  [
    3
  ]
]

# tq -o json
[
  1,
  2,
  [
    3
  ]
]

# tq
[3]:
  - 1
  - 2
  - [1]: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 21 | +3 | +16.67% |
| `cl100k_base` | 18 | 18 | 21 | +3 | +16.67% |

#### manual-bof-table-062

```
# input
jq flatten

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual-bof-table-063

```
# input
jq flatten

# jq
[
  {
    "foo": "bar"
  },
  {
    "foo": "baz"
  }
]

# tq -o json
[
  {
    "foo": "bar"
  },
  {
    "foo": "baz"
  }
]

# tq
[2]{foo}:
  bar
  baz
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 24 | 24 | 12 | -12 | -50% |
| `cl100k_base` | 24 | 24 | 12 | -12 | -50% |

#### manual-bof-table-064

```
# input
jq 'range(2; 4)'

# jq
2
3

# tq -o json
2
3

# tq
2
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-table-065

```
# input
jq '[range(2; 4)]'

# jq
[
  2,
  3
]

# tq -o json
[
  2,
  3
]

# tq
[2]: 2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual-bof-table-066

```
# input
jq '[range(4)]'

# jq
[
  0,
  1,
  2,
  3
]

# tq -o json
[
  0,
  1,
  2,
  3
]

# tq
[4]: 0,1,2,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-067

```
# input
jq '[range(0; 10; 3)]'

# jq
[
  0,
  3,
  6,
  9
]

# tq -o json
[
  0,
  3,
  6,
  9
]

# tq
[4]: 0,3,6,9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-068

```
# input
jq '[range(0; 10; -1)]'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual-bof-table-069

```
# input
jq '[range(0; -5; -1)]'

# jq
[
  0,
  -1,
  -2,
  -3,
  -4
]

# tq -o json
[
  0,
  -1,
  -2,
  -3,
  -4
]

# tq
[5]: 0,-1,-2,-3,-4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 14 | -8 | -36.36% |
| `cl100k_base` | 22 | 22 | 14 | -8 | -36.36% |

#### manual-bof-table-070

```
# input
jq floor

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-071

```
# input
jq sqrt

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-072

```
# input
jq '.[] | tonumber'

# jq
1
1

# tq -o json
1
1

# tq
1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-table-073

```
# input
jq '.[] | toboolean'

# jq
true
false
true
false

# tq -o json
true
false
true
false

# tq
true
false
true
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### manual-bof-table-074

```
# input
jq '.[] | tostring'

# jq
"1"
"1"
"[1]"

# tq -o json
"1"
"1"
"[1]"

# tq
"1"
"1"
"[1]"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 9 | 0 | +0% |
| `cl100k_base` | 9 | 9 | 9 | 0 | +0% |

#### manual-bof-table-075

```
# input
jq 'map(type)'

# jq
[
  "number",
  "boolean",
  "array",
  "object",
  "null",
  "string"
]

# tq -o json
[
  "number",
  "boolean",
  "array",
  "object",
  "null",
  "string"
]

# tq
[6]: number,boolean,array,object,"null",string
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 14 | -12 | -46.15% |
| `cl100k_base` | 26 | 26 | 14 | -12 | -46.15% |

#### manual-bof-table-076

```
# input
jq '.[] | (infinite * .) < 0'

# jq
true
false

# tq -o json
true
false

# tq
true
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### manual-bof-table-077

```
# input
jq 'infinite, nan | type'

# jq
"number"
"number"

# tq -o json
"number"
"number"

# tq
number
number
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 4 | -2 | -33.33% |
| `cl100k_base` | 6 | 6 | 4 | -2 | -33.33% |

#### manual-bof-table-078

```
# input
jq sort

# jq
[
  null,
  3,
  6,
  8
]

# tq -o json
[
  null,
  3,
  6,
  8
]

# tq
[4]: null,3,6,8
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 11 | -6 | -35.29% |
| `cl100k_base` | 17 | 17 | 11 | -6 | -35.29% |

#### manual-bof-table-079

```
# input
jq 'sort_by(.foo)'

# jq
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq -o json
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq
[3]{foo,bar}:
  2,1
  3,10
  4,10
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 56 | 56 | 26 | -30 | -53.57% |
| `cl100k_base` | 56 | 56 | 26 | -30 | -53.57% |

#### manual-bof-table-080

```
# input
jq 'sort_by(.foo, .bar)'

# jq
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 3,
    "bar": 20
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq -o json
[
  {
    "foo": 2,
    "bar": 1
  },
  {
    "foo": 3,
    "bar": 10
  },
  {
    "foo": 3,
    "bar": 20
  },
  {
    "foo": 4,
    "bar": 10
  }
]

# tq
[4]{foo,bar}:
  2,1
  3,10
  3,20
  4,10
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 74 | 74 | 32 | -42 | -56.76% |
| `cl100k_base` | 74 | 74 | 32 | -42 | -56.76% |

#### manual-bof-table-081

```
# input
jq 'group_by(.foo)'

# jq
[
  [
    {
      "foo": 1,
      "bar": 10
    },
    {
      "foo": 1,
      "bar": 1
    }
  ],
  [
    {
      "foo": 3,
      "bar": 100
    }
  ]
]

# tq -o json
[
  [
    {
      "foo": 1,
      "bar": 10
    },
    {
      "foo": 1,
      "bar": 1
    }
  ],
  [
    {
      "foo": 3,
      "bar": 100
    }
  ]
]

# tq
[2]:
  - [2]{foo,bar}:
    1,10
    1,1
  - [1]{foo,bar}:
    3,100
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 64 | 64 | 41 | -23 | -35.94% |
| `cl100k_base` | 64 | 64 | 41 | -23 | -35.94% |

#### manual-bof-table-082

```
# input
jq min

# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-083

```
# input
jq 'max_by(.foo)'

# jq
{
  "foo": 2,
  "bar": 3
}

# tq -o json
{
  "foo": 2,
  "bar": 3
}

# tq
foo: 2
bar: 3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 10 | -6 | -37.5% |
| `cl100k_base` | 16 | 16 | 10 | -6 | -37.5% |

#### manual-bof-table-084

```
# input
jq unique

# jq
[
  1,
  2,
  3,
  5
]

# tq -o json
[
  1,
  2,
  3,
  5
]

# tq
[4]: 1,2,3,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-085

```
# input
jq 'unique_by(.foo)'

# jq
[
  {
    "foo": 1,
    "bar": 2
  },
  {
    "foo": 4,
    "bar": 5
  }
]

# tq -o json
[
  {
    "foo": 1,
    "bar": 2
  },
  {
    "foo": 4,
    "bar": 5
  }
]

# tq
[2]{foo,bar}:
  1,2
  4,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 38 | 38 | 20 | -18 | -47.37% |
| `cl100k_base` | 38 | 38 | 20 | -18 | -47.37% |

#### manual-bof-table-086

```
# input
jq 'unique_by(length)'

# jq
[
  "bacon",
  "chunky",
  "asparagus"
]

# tq -o json
[
  "bacon",
  "chunky",
  "asparagus"
]

# tq
[3]: bacon,chunky,asparagus
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 11 | -6 | -35.29% |
| `cl100k_base` | 17 | 17 | 11 | -6 | -35.29% |

#### manual-bof-table-087

```
# input
jq reverse

# jq
[
  4,
  3,
  2,
  1
]

# tq -o json
[
  4,
  3,
  2,
  1
]

# tq
[4]: 4,3,2,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-088

```
# input
jq 'contains("bar")'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-089

```
# input
jq 'contains(["baz", "bar"])'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-090

```
# input
jq 'contains(["bazzzzz", "bar"])'

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-091

```
# input
jq 'contains({foo: 12, bar: [{barp: 12}]})'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-092

```
# input
jq 'contains({foo: 12, bar: [{barp: 15}]})'

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-093

```
# input
jq 'indices(", ")'

# jq
[
  3,
  7,
  12
]

# tq -o json
[
  3,
  7,
  12
]

# tq
[3]: 3,7,12
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-094

```
# input
jq 'indices(1)'

# jq
[
  1,
  3,
  5
]

# tq -o json
[
  1,
  3,
  5
]

# tq
[3]: 1,3,5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 10 | -4 | -28.57% |
| `cl100k_base` | 14 | 14 | 10 | -4 | -28.57% |

#### manual-bof-table-095

```
# input
jq 'indices([1,2])'

# jq
[
  1,
  8
]

# tq -o json
[
  1,
  8
]

# tq
[2]: 1,8
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 10 | 10 | 8 | -2 | -20% |
| `cl100k_base` | 10 | 10 | 8 | -2 | -20% |

#### manual-bof-table-096

```
# input
jq 'index(", ")'

# jq
3

# tq -o json
3

# tq
3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-097

```
# input
jq 'index(1)'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-098

```
# input
jq 'index([1,2])'

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-099

```
# input
jq 'rindex(", ")'

# jq
12

# tq -o json
12

# tq
12
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-100

```
# input
jq 'rindex(1)'

# jq
5

# tq -o json
5

# tq
5
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-101

```
# input
jq 'rindex([1,2])'

# jq
8

# tq -o json
8

# tq
8
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-102

```
# input
jq 'inside("foobar")'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-103

```
# input
jq 'inside(["foobar", "foobaz", "blarp"])'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-104

```
# input
jq 'inside(["foobar", "foobaz", "blarp"])'

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-105

```
# input
jq 'inside({"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]})'

# jq
true

# tq -o json
true

# tq
true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-106

```
# input
jq 'inside({"foo": 12, "bar":[1,2,{"barp":12, "blip":13}]})'

# jq
false

# tq -o json
false

# tq
false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-107

```
# input
jq '[.[]|startswith("foo")]'

# jq
[
  false,
  true,
  false,
  true,
  false
]

# tq -o json
[
  false,
  true,
  false,
  true,
  false
]

# tq
[5]: false,true,false,true,false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 9 | -8 | -47.06% |
| `cl100k_base` | 17 | 17 | 9 | -8 | -47.06% |

#### manual-bof-table-108

```
# input
jq '[.[]|endswith("foo")]'

# jq
[
  false,
  true
]

# tq -o json
[
  false,
  true
]

# tq
[2]: false,true
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 6 | -2 | -25% |
| `cl100k_base` | 8 | 8 | 6 | -2 | -25% |

#### manual-bof-table-109

```
# input
jq combinations

# jq
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq -o json
[
  1,
  3
]
[
  1,
  4
]
[
  2,
  3
]
[
  2,
  4
]

# tq
[2]: 1,3
[2]: 1,4
[2]: 2,3
[2]: 2,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 32 | -8 | -20% |
| `cl100k_base` | 40 | 40 | 32 | -8 | -20% |

#### manual-bof-table-110

```
# input
jq 'combinations(2)'

# jq
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq -o json
[
  0,
  0
]
[
  0,
  1
]
[
  1,
  0
]
[
  1,
  1
]

# tq
[2]: 0,0
[2]: 0,1
[2]: 1,0
[2]: 1,1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 40 | 40 | 32 | -8 | -20% |
| `cl100k_base` | 40 | 40 | 32 | -8 | -20% |

#### manual-bof-table-111

```
# input
jq '[.[]|ltrimstr("foo")]'

# jq
[
  "fo",
  "",
  "barfoo",
  "bar",
  "afoo"
]

# tq -o json
[
  "fo",
  "",
  "barfoo",
  "bar",
  "afoo"
]

# tq
[5]: fo,"",barfoo,bar,afoo
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 22 | 22 | 13 | -9 | -40.91% |
| `cl100k_base` | 22 | 22 | 13 | -9 | -40.91% |

#### manual-bof-table-112

```
# input
jq '[.[]|rtrimstr("foo")]'

# jq
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "foobar",
  "foob"
]

# tq
[5]: fo,"",bar,foobar,foob
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 21 | 21 | 13 | -8 | -38.1% |
| `cl100k_base` | 21 | 21 | 13 | -8 | -38.1% |

#### manual-bof-table-113

```
# input
jq '[.[]|trimstr("foo")]'

# jq
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq -o json
[
  "fo",
  "",
  "bar",
  "bar",
  "b"
]

# tq
[5]: fo,"",bar,bar,b
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 11 | -9 | -45% |
| `cl100k_base` | 20 | 20 | 11 | -9 | -45% |

#### manual-bof-table-114

```
# input
jq 'trim, ltrim, rtrim'

# jq
"abc"
"abc "
" abc"

# tq -o json
"abc"
"abc "
" abc"

# tq
abc
"abc "
" abc"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 8 | -1 | -11.11% |
| `cl100k_base` | 9 | 9 | 8 | -1 | -11.11% |

#### manual-bof-table-115

```
# input
jq explode

# jq
[
  102,
  111,
  111,
  98,
  97,
  114
]

# tq -o json
[
  102,
  111,
  111,
  98,
  97,
  114
]

# tq
[6]: 102,111,111,98,97,114
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 26 | 26 | 16 | -10 | -38.46% |
| `cl100k_base` | 26 | 26 | 16 | -10 | -38.46% |

#### manual-bof-table-116

```
# input
jq implode

# jq
"ABC"

# tq -o json
"ABC"

# tq
ABC
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual-bof-table-117

```
# input
jq 'split(", ")'

# jq
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq -o json
[
  "a",
  "b,c,d",
  "e",
  ""
]

# tq
[4]: a,"b,c,d",e,""
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-118

```
# input
jq 'join(", ")'

# jq
"a, b,c,d, e"

# tq -o json
"a, b,c,d, e"

# tq
"a, b,c,d, e"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 8 | 0 | +0% |
| `cl100k_base` | 8 | 8 | 8 | 0 | +0% |

#### manual-bof-table-119

```
# input
jq 'join(" ")'

# jq
"a 1 2.3 true  false"

# tq -o json
"a 1 2.3 true  false"

# tq
a 1 2.3 true  false
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 11 | 11 | 11 | 0 | +0% |
| `cl100k_base` | 11 | 11 | 11 | 0 | +0% |

#### manual-bof-table-120

```
# input
jq 'join(", ")'

# jq
"1, 2"

# tq -o json
"1, 2"

# tq
"1, 2"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-table-121

```
# input
jq ascii_upcase

# jq
"USEFUL BUT NOT FOR é"

# tq -o json
"USEFUL BUT NOT FOR é"

# tq
USEFUL BUT NOT FOR é
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 8 | 8 | 7 | -1 | -12.5% |
| `cl100k_base` | 8 | 8 | 7 | -1 | -12.5% |

#### manual-bof-table-122

```
# input
jq '[while(.<100; .*2)]'

# jq
[
  1,
  2,
  4,
  8,
  16,
  32,
  64
]

# tq -o json
[
  1,
  2,
  4,
  8,
  16,
  32,
  64
]

# tq
[7]: 1,2,4,8,16,32,64
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 18 | -12 | -40% |
| `cl100k_base` | 30 | 30 | 18 | -12 | -40% |

#### manual-bof-table-123

```
# input
jq '[repeat(.*2, error)?]'

# jq
[
  2
]

# tq -o json
[
  2
]

# tq
[1]: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-table-124

```
# input
jq '[.,1]|until(.[0] < 1; [.[0] - 1, .[1] * .[0]])|.[1]'

# jq
24

# tq -o json
24

# tq
24
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-125

```
# input
jq 'recurse(.foo[])'

# jq
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq -o json
{
  "foo": [
    {
      "foo": []
    },
    {
      "foo": [
        {
          "foo": []
        }
      ]
    }
  ]
}
{
  "foo": []
}
{
  "foo": [
    {
      "foo": []
    }
  ]
}
{
  "foo": []
}

# tq
foo[2]:
  - foo[0]:
  - foo[1]:
      - foo[0]:
foo[0]:
foo[1]:
  - foo[0]:
foo[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 70 | 70 | 40 | -30 | -42.86% |
| `cl100k_base` | 70 | 70 | 40 | -30 | -42.86% |

#### manual-bof-table-126

```
# input
jq recurse

# jq
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq -o json
{
  "a": 0,
  "b": [
    1
  ]
}
0
[
  1
]
1

# tq
a: 0
b[1]: 1
0
[1]: 1
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 30 | 30 | 22 | -8 | -26.67% |
| `cl100k_base` | 30 | 30 | 22 | -8 | -26.67% |

#### manual-bof-table-127

```
# input
jq 'recurse(. * .; . < 20)'

# jq
2
4
16

# tq -o json
2
4
16

# tq
2
4
16
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 6 | 6 | 6 | 0 | +0% |
| `cl100k_base` | 6 | 6 | 6 | 0 | +0% |

#### manual-bof-table-128

```
# input
jq 'walk(if type == "array" then sort else . end)'

# jq
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq -o json
[
  [
    1,
    4,
    7
  ],
  [
    2,
    5,
    8
  ],
  [
    3,
    6,
    9
  ]
]

# tq
[3]:
  - [3]: 1,4,7
  - [3]: 2,5,8
  - [3]: 3,6,9
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 50 | 50 | 39 | -11 | -22% |
| `cl100k_base` | 50 | 50 | 39 | -11 | -22% |

#### manual-bof-table-129

```
# input
jq 'walk( if type == "object" then with_entries( .key |= sub( "^_+"; "") ) else . end )'

# jq
[
  {
    "a": {
      "b": 2
    }
  }
]

# tq -o json
[
  {
    "a": {
      "b": 2
    }
  }
]

# tq
[1]:
  - a:
      b: 2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 20 | 20 | 13 | -7 | -35% |
| `cl100k_base` | 20 | 20 | 13 | -7 | -35% |

#### manual-bof-table-130

```
# input
jq '$ENV.PAGER'

# jq
"less"

# tq -o json
"less"

# tq
less
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual-bof-table-131

```
# input
jq env.PAGER

# jq
"less"

# tq -o json
"less"

# tq
less
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

#### manual-bof-table-132

```
# input
jq transpose

# jq
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq -o json
[
  [
    1,
    2
  ],
  [
    null,
    3
  ]
]

# tq
[2]:
  - [2]: 1,2
  - [2]: null,3
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 25 | 25 | 22 | -3 | -12% |
| `cl100k_base` | 25 | 25 | 22 | -3 | -12% |

#### manual-bof-table-133

```
# input
jq 'bsearch(0)'

# jq
0

# tq -o json
0

# tq
0
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual-bof-table-134

```
# input
jq 'bsearch(0)'

# jq
-1

# tq -o json
-1

# tq
-1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 3 | 0 | +0% |
| `cl100k_base` | 3 | 3 | 3 | 0 | +0% |

#### manual-bof-table-135

```
# input
jq 'bsearch(4) as $ix | if $ix < 0 then .[-(1+$ix)] = 4 else . end'

# jq
[
  1,
  2,
  3,
  4
]

# tq -o json
[
  1,
  2,
  3,
  4
]

# tq
[4]: 1,2,3,4
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 12 | -6 | -33.33% |
| `cl100k_base` | 18 | 18 | 12 | -6 | -33.33% |

#### manual-bof-table-136

```
# input
jq '"The input was \(.), which is one less than \(.+1)"'

# jq
"The input was 42, which is one less than 43"

# tq -o json
"The input was 42, which is one less than 43"

# tq
"The input was 42, which is one less than 43"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 14 | 14 | 14 | 0 | +0% |
| `cl100k_base` | 14 | 14 | 14 | 0 | +0% |

#### manual-bof-table-137

```
# input
jq '[.[]|tostring]'

# jq
[
  "1",
  "foo",
  "[\"foo\"]"
]

# tq -o json
[
  "1",
  "foo",
  "[\"foo\"]"
]

# tq
[3]: "1",foo,"[\"foo\"]"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 13 | -3 | -18.75% |
| `cl100k_base` | 16 | 16 | 13 | -3 | -18.75% |

#### manual-bof-table-138

```
# input
jq '[.[]|tojson]'

# jq
[
  "1",
  "\"foo\"",
  "[\"foo\"]"
]

# tq -o json
[
  "1",
  "\"foo\"",
  "[\"foo\"]"
]

# tq
[3]: "1","\"foo\"","[\"foo\"]"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 17 | 17 | 15 | -2 | -11.76% |
| `cl100k_base` | 16 | 16 | 15 | -1 | -6.25% |

#### manual-bof-table-139

```
# input
jq '[.[]|tojson|fromjson]'

# jq
[
  1,
  "foo",
  [
    "foo"
  ]
]

# tq -o json
[
  1,
  "foo",
  [
    "foo"
  ]
]

# tq
[3]:
  - 1
  - foo
  - [1]: foo
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 18 | 18 | 19 | +1 | +5.56% |
| `cl100k_base` | 18 | 18 | 19 | +1 | +5.56% |

#### manual-bof-table-140

```
# input
jq '@html'

# jq
"This works if x &lt; y"

# tq -o json
"This works if x &lt; y"

# tq
This works if x &lt; y
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 9 | 9 | 9 | 0 | +0% |
| `cl100k_base` | 9 | 9 | 9 | 0 | +0% |

#### manual-bof-table-141

```
# input
jq '@sh "echo \(.)"'

# jq
"echo 'O'\\''Hara'\\''s Ale'"

# tq -o json
"echo 'O'\\''Hara'\\''s Ale'"

# tq
"echo 'O'\\''Hara'\\''s Ale'"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 15 | 15 | 15 | 0 | +0% |
| `cl100k_base` | 15 | 15 | 15 | 0 | +0% |

#### manual-bof-table-142

```
# input
jq '@base64'

# jq
"VGhpcyBpcyBhIG1lc3NhZ2U="

# tq -o json
"VGhpcyBpcyBhIG1lc3NhZ2U="

# tq
VGhpcyBpcyBhIG1lc3NhZ2U=
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 16 | 16 | 15 | -1 | -6.25% |
| `cl100k_base` | 19 | 19 | 18 | -1 | -5.26% |

#### manual-bof-table-143

```
# input
jq '@base64d'

# jq
"This is a message"

# tq -o json
"This is a message"

# tq
This is a message
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual-bof-table-144

```
# input
jq fromdate

# jq
1425599507

# tq -o json
1425599507

# tq
1425599507
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual-bof-table-146

```
# input
jq 'strptime("%Y-%m-%dT%H:%M:%SZ")|mktime'

# jq
1425599507

# tq -o json
1425599507

# tq
1425599507
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 5 | 5 | 5 | 0 | +0% |
| `cl100k_base` | 5 | 5 | 5 | 0 | +0% |

#### manual.audit.documented-arities

```
# input
jq '["abs/0","acos/0","acosh/0","add/0","add/1","all/0","all/1","all/2","any/0","any/1","any/2","arrays/0","ascii_downcase/0","ascii_upcase/0","asin/0","asinh/0","atan/0","atan2/2","atanh/0","booleans/0","bsearch/1","builtins/0","capture/1","capture/2","cbrt/0","ceil/0","combinations/0","combinations/1","contains/1","copysign/2","cos/0","cosh/0","debug/0","debug/1","del/1","delpaths/1","drem/2","empty/0","endswith/1","env/0","erf/0","erfc/0","error/0","error/1","exp/0","exp10/0","exp2/0","explode/0","expm1/0","fabs/0","fdim/2","finites/0","first/0","first/1","flatten/0","flatten/1","floor/0","fma/3","fmax/2","fmin/2","fmod/2","frexp/0","from_entries/0","fromdate/0","fromdateiso8601/0","fromjson/0","fromstream/1","gamma/0","getpath/1","gmtime/0","group_by/1","gsub/2","gsub/3","halt_error/0","halt_error/1","halt/0","has/1","have_decnum/0","have_literal_numbers/0","hypot/2","implode/0","in/1","IN/1","IN/2","index/1","INDEX/2","indices/1","infinite/0","input_filename/0","input_line_number/0","input/0","inputs/0","inside/1","isempty/1","isfinite/0","isinfinite/0","isnan/0","isnormal/0","iterables/0","j0/0","j1/0","jn/2","join/1","JOIN/2","JOIN/3","JOIN/4","keys_unsorted/0","keys/0","last/0","last/1","ldexp/2","length/0","lgamma/0","limit/2","localtime/0","log/0","log10/0","log1p/0","log2/0","logb/0","ltrim/0","ltrimstr/1","map_values/1","map/1","match/1","match/2","max_by/1","max/0","min_by/1","min/0","mktime/0","modf/0","modulemeta/0","nan/0","nearbyint/0","nextafter/2","nexttoward/2","normals/0","not/0","now/0","nth/1","nth/2","nulls/0","numbers/0","objects/0","path/1","paths/0","paths/1","pick/1","pow/2","range/1","range/2","range/3","recurse/0","recurse/1","recurse/2","remainder/2","repeat/1","reverse/0","rindex/1","rint/0","round/0","rtrim/0","rtrimstr/1","scalars/0","scalb/2","scalbln/2","scan/1","scan/2","select/1","setpath/2","significand/0","sin/0","sinh/0","skip/2","sort_by/1","sort/0","split/1","split/2","splits/1","splits/2","sqrt/0","startswith/1","stderr/0","strflocaltime/1","strftime/1","strings/0","strptime/1","sub/2","sub/3","tan/0","tanh/0","test/1","test/2","tgamma/0","to_entries/0","toboolean/0","todate/0","todateiso8601/0","tojson/0","tonumber/0","tostream/0","tostring/0","transpose/0","trim/0","trimstr/1","trunc/0","truncate_stream/1","type/0","unique_by/1","unique/0","until/2","utf8bytelength/0","values/0","walk/1","while/2","with_entries/1","y0/0","y1/0","yn/2"] - builtins'

# jq
[]

# tq -o json
[]

# tq
[0]:
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 1 | 1 | 3 | +2 | +200% |
| `cl100k_base` | 1 | 1 | 3 | +2 | +200% |

#### manual.invoking.halt-error

```
# input
jq 'halt_error(7)'

# jq


[stderr]
failure

# tq -o json


[stderr]
failure

# tq
<not run>
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 0 | 0 | n/a | n/a | n/a |
| `cl100k_base` | 0 | 0 | n/a | n/a | n/a |

#### manual.math.floor

```
# input
jq floor

# jq
1

# tq -o json
1

# tq
1
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### manual.math.sqrt

```
# input
jq sqrt

# jq
2

# tq -o json
2

# tq
2
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 2 | 2 | 2 | 0 | +0% |
| `cl100k_base` | 2 | 2 | 2 | 0 | +0% |

#### platform.localtime-year

```
# input
jq 'strflocaltime("%Y")'

# jq
"2015"

# tq -o json
"2015"

# tq
"2015"
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 4 | 4 | 4 | 0 | +0% |
| `cl100k_base` | 4 | 4 | 4 | 0 | +0% |

#### platform.now-type

```
# input
jq 'now | type'

# jq
"number"

# tq -o json
"number"

# tq
number
```

| Tokenizer | jq | tq -o json | tq | Diff | % |
| --- | ---: | ---: | ---: | ---: | ---: |
| `o200k_base` | 3 | 3 | 2 | -1 | -33.33% |
| `cl100k_base` | 3 | 3 | 2 | -1 | -33.33% |

<!-- tq-manual-compare:end -->
