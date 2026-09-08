# jq manual: builtin operators and functions coverage review

Source: `/Users/reno/Development/commandzero/tq-benchmarks/jq-manual/builtin-operators-and-functions.md`

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
