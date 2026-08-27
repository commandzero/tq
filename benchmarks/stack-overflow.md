# Stack Overflow jq benchmarks

This report benchmarks the 50 highest-voted Stack Overflow questions tagged
`jq`. See the [source list](https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50).
Each checked-in scenario keeps the question and one answer. The generator uses
the accepted answer when one exists, otherwise it uses the highest-voted answer.

## Method

- One warmup followed by five measured samples per tool and scenario.
- Time is the median wall-clock duration per scenario, reported in milliseconds.
- Memory is the maximum observed resident set size across measured samples, reported in MiB.
- Deltas are percentage changes relative to `jq`. Positive means slower or more memory.
- Correctness is checked before timing using the shared semantic-sequence gate.

## Execution time (ms)

| # | Scenario | jq | yq | yq Δ | tq | tq Δ |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 01 | [1: Parsing JSON with Unix tools](../tests/stack-overflow/01-parsing-json-with-unix-tools.json), `.name` | 75.811 | 77.865 | +2.7% | 83.487 | +10.1% |
| 02 | [2: How to remove double-quotes in jq output for parsing json files in bash?](../tests/stack-overflow/02-how-to-remove-double-quotes-in-jq-output-for-parsing-json-fi.json), `.name` | 74.695 | 83.075 | +11.2% | 78.492 | +5.1% |
| 03 | [3: Select objects based on value of variable in object using jq](../tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.json), `.[] \| select(.location == "Stockholm") \| .name` | 70.742 | 75.110 | +6.2% | 77.729 | +9.9% |
| 04 | [4: Using jq to parse and display multiple fields in a json serially](../tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.json), `.users[] \| "\(.first) \(.last)"` | 70.940 | 76.616 | +8.0% | 77.669 | +9.5% |
| 05 | [5: jq: how to filter an array of objects based on values in an inner array?](../tests/stack-overflow/05-jq-how-to-filter-an-array-of-objects-based-on-values-in-an-i.json), `.[] \| select(.Names[] == "foo_data") \| .Id` | 71.152 | 76.180 | +7.1% | 77.153 | +8.4% |
| 06 | [6: How to count items in JSON object using command line?](../tests/stack-overflow/06-how-to-count-items-in-json-object-using-command-line.json), `length` | 71.194 | 93.994 | +32.0% | 78.958 | +10.9% |
| 07 | [7: How to install JQ on Mac on the command line?](../tests/stack-overflow/07-how-to-install-jq-on-mac-on-the-command-line.json), `.` | 70.662 | 78.668 | +11.3% | 77.030 | +9.0% |
| 08 | [8: How do I select multiple fields in jq?](../tests/stack-overflow/08-how-do-i-select-multiple-fields-in-jq.json), `{"login": .login, "id": .id}` | 71.271 | 76.879 | +7.9% | 73.690 | +3.4% |
| 09 | [9: How to get key names from JSON using jq](../tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.json), `keys \| sort \| .[]` | 78.567 | 76.458 | -2.7% | 75.359 | -4.1% |
| 10 | [10: JQ: Select multiple conditions](../tests/stack-overflow/10-jq-select-multiple-conditions.json), `.[] \| select((.processedBarsVolume <= 5) and .processedBars > 0)` | 71.816 | 77.613 | +8.1% | 70.935 | -1.2% |
| 11 | [11: Passing bash variable to jq](../tests/stack-overflow/11-passing-bash-variable-to-jq.json), `.resource[] \| select(.username == "myemail@hotmail.com") \| .id` | 74.358 | 75.347 | +1.3% | 74.249 | -0.1% |
| 12 | [12: How to merge 2 JSON objects from 2 files using jq?](../tests/stack-overflow/12-how-to-merge-2-json-objects-from-2-files-using-jq.json), `.[] \| .value` | 70.394 | 75.154 | +6.8% | 71.034 | +0.9% |
| 13 | [13: How to convert arbitrary simple JSON to CSV using jq?](../tests/stack-overflow/13-how-to-convert-arbitrary-simple-json-to-csv-using-jq.json), `map({"code": .code, "name": .name, "level": .level, "country": .country})` | 74.363 | 72.168 | -3.0% | 74.576 | +0.3% |
| 14 | [14: How to use `jq` in a shell pipeline?](../tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.json), `.` | 70.082 | 78.589 | +12.1% | 71.962 | +2.7% |
| 15 | [15: How do I update a single value in a json document using jq?](../tests/stack-overflow/15-how-do-i-update-a-single-value-in-a-json-document-using-jq.json), `.shipping` | 75.538 | 79.422 | +5.1% | 72.550 | -4.0% |
| 16 | [16: how to parse a JSON String with jq (or other alternatives)?](../tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.json), `.c` | 74.192 | 77.171 | +4.0% | 76.387 | +3.0% |
| 17 | [17: Using jq or alternative command line tools to compare JSON files](../tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.json), `.[0].City == .[1].City` | 71.718 | 77.226 | +7.7% | 73.600 | +2.6% |
| 18 | [18: Get outputs from jq on a single line](../tests/stack-overflow/18-get-outputs-from-jq-on-a-single-line.json), `.issues[] \| {"key": .key, "status": .fields.status.name, "assignee": .fields.assignee.emailAddress}` | 78.431 | 77.479 | -1.2% | 73.321 | -6.5% |
| 19 | [19: jq to replace text directly on file (like sed -i)](../tests/stack-overflow/19-jq-to-replace-text-directly-on-file-like-sed-i.json), `.Actions[] \| .properties.other` | 76.230 | 79.654 | +4.5% | 72.816 | -4.5% |
| 20 | [20: Concat 2 fields in JSON using jq](../tests/stack-overflow/20-concat-2-fields-in-json-using-jq.json), `{"channel": (.profile_type + "." + .channel)}` | 76.880 | 80.173 | +4.3% | 75.571 | -1.7% |
| 21 | [21: How to sort a json file by keys and values of those keys in jq](../tests/stack-overflow/21-how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-.json), `.components.rows \| sort_by(.id)` | 75.014 | 78.792 | +5.0% | 69.592 | -7.2% |
| 22 | [22: Modify a key-value in a json using jq in-place](../tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.json), `.address = "abcde"` | 73.234 | 77.072 | +5.2% | 70.806 | -3.3% |
| 23 | [23: How to format a JSON string as a table using jq?](../tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.json), `.[] \| [.id, .name]` | 74.699 | 75.364 | +0.9% | 74.396 | -0.4% |
| 24 | [24: jq: print key and value for each entry in an object](../tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.json), `keys[]` | 77.345 | 78.121 | +1.0% | 70.431 | -8.9% |
| 25 | [25: Extract a specific field from JSON output using jq](../tests/stack-overflow/25-extract-a-specific-field-from-json-output-using-jq.json), `.example["sub-example"][] \| .name` | 73.822 | 77.278 | +4.7% | 75.105 | +1.7% |
| 26 | [26: How to run jq from gitbash in Windows?](../tests/stack-overflow/26-how-to-run-jq-from-gitbash-in-windows.json), `.` | 70.833 | 78.553 | +10.9% | 72.773 | +2.7% |
| 27 | [27: jq: output array of json objects](../tests/stack-overflow/27-jq-output-array-of-json-objects.json), `map({"name": .name, "email": .email})` | 78.222 | 74.443 | -4.8% | 73.475 | -6.1% |
| 28 | [28: Add new element to existing JSON array with jq](../tests/stack-overflow/28-add-new-element-to-existing-json-array-with-jq.json), `.data.messages += [{"date":"2010-01-07T19:55:99.999Z","xml":"new.xml","status":"OKKK","message":"added"}]` | 73.892 | 71.884 | -2.7% | 73.068 | -1.1% |
| 29 | [29: How do I use jq to convert number to string?](../tests/stack-overflow/29-how-do-i-use-jq-to-convert-number-to-string.json), `.[] \| .number \| tostring` | 73.584 | 77.320 | +5.1% | 70.740 | -3.9% |
| 30 | [30: get the first (or n&#39;th) element in a jq json parsing](../tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.json), `.[0]` | 77.816 | 79.483 | +2.1% | 70.639 | -9.2% |
| 31 | [31: Can I pass a string variable to jq rather than passing a file?](../tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.json), `.key` | 79.791 | 79.585 | -0.3% | 72.928 | -8.6% |
| 32 | [32: Iterating through JSON array in Shell script](../tests/stack-overflow/32-iterating-through-json-array-in-shell-script.json), `.[]` | 70.176 | 78.482 | +11.8% | 72.579 | +3.4% |
| 33 | [33: How to check for presence of &#39;key&#39; in jq before iterating over the values](../tests/stack-overflow/33-how-to-check-for-presence-of-39-key-39-in-jq-before-iteratin.json), `.result \| select(.property_history != null) \| .property_history \| map(select(.event_name == "Sold"))[0].date` | 74.274 | 71.900 | -3.2% | 73.607 | -0.9% |
| 34 | [34: How to filter array of objects by element property values using jq?](../tests/stack-overflow/34-how-to-filter-array-of-objects-by-element-property-values-us.json), `.theList[] \| select(.id == 2 or .id == 4)` | 69.859 | 77.874 | +11.5% | 74.621 | +6.8% |
| 35 | [35: jq Conditional output](../tests/stack-overflow/35-jq-conditional-output.json), `.geo != null` | 77.760 | 70.564 | -9.3% | 72.433 | -6.9% |
| 36 | [36: jq: Cannot index array with string](../tests/stack-overflow/36-jq-cannot-index-array-with-string.json), `.[] \| .aux[] \| .["def"]` | 71.200 | 80.658 | +13.3% | 70.888 | -0.4% |
| 37 | [37: Install jq JSON processor on Ubuntu 10.04](../tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.json), `.` | 79.069 | 70.680 | -10.6% | 72.813 | -7.9% |
| 38 | [38: How to combine the sequence of objects in jq into one object?](../tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.json), `map(.a)` | 74.417 | 80.695 | +8.4% | 74.039 | -0.5% |
| 39 | [39: jq not working on tag name with dashes and numbers](../tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.json), `.status` | 73.455 | 80.148 | +9.1% | 70.817 | -3.6% |
| 40 | [40: Convert string to json in jq](../tests/stack-overflow/40-convert-string-to-json-in-jq.json), `.response.text` | 77.311 | 79.875 | +3.3% | 70.138 | -9.3% |
| 41 | [41: Output specific key value in object for each element in array with jq for JSON](../tests/stack-overflow/41-output-specific-key-value-in-object-for-each-element-in-arra.json), `.[].AssetId` | 74.307 | 74.611 | +0.4% | 73.877 | -0.6% |
| 42 | [42: jq: how to query for array values that don&#39;t contain text &quot;foo&quot;?](../tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.json), `.[] \| .name` | 78.436 | 80.312 | +2.4% | 72.356 | -7.8% |
| 43 | [43: How do I keep colors when piping &quot;jq&quot; output to &quot;less&quot;?](../tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.json), `.` | 75.590 | 75.103 | -0.6% | 79.137 | +4.7% |
| 44 | [44: Exclude column from jq json output](../tests/stack-overflow/44-exclude-column-from-jq-json-output.json), `.[].group` | 80.717 | 74.536 | -7.7% | 73.560 | -8.9% |
| 45 | [45: How to use jq when the variable has reserved characters?](../tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.json), `.USD.price` | 77.685 | 77.401 | -0.4% | 72.165 | -7.1% |
| 46 | [46: How to extract a field from each object in an array with jq?](../tests/stack-overflow/46-how-to-extract-a-field-from-each-object-in-an-array-with-jq.json), `.[].username` | 80.798 | 72.491 | -10.3% | 75.109 | -7.0% |
| 47 | [47: How to check if element exists in array with jq](../tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.json), `.fruit[] \| select(. == "orange")` | 79.421 | 72.569 | -8.6% | 75.642 | -4.8% |
| 48 | [48: jq select value from array](../tests/stack-overflow/48-jq-select-value-from-array.json), `.[] \| select(.name == "foo")` | 77.993 | 73.425 | -5.9% | 79.259 | +1.6% |
| 49 | [49: getting all the values of an array with jq](../tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.json), `.response[] \| select(has("text")) \| .text` | 81.657 | 71.649 | -12.3% | 79.733 | -2.4% |
| 50 | [50: Using jq with bash to run command for each object in array](../tests/stack-overflow/50-using-jq-with-bash-to-run-command-for-each-object-in-array.json), `.[].user, .[].date, .[].email` | 79.750 | 71.145 | -10.8% | 75.347 | -5.5% |

## Peak memory (MiB)

| # | Scenario | jq | yq | yq Δ | tq | tq Δ |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 01 | [1: Parsing JSON with Unix tools](../tests/stack-overflow/01-parsing-json-with-unix-tools.json) | 1.34 | 2.94 | +118.6% | 1.36 | +1.2% |
| 02 | [2: How to remove double-quotes in jq output for parsing json files in bash?](../tests/stack-overflow/02-how-to-remove-double-quotes-in-jq-output-for-parsing-json-fi.json) | 1.66 | 1.86 | +12.3% | 1.81 | +9.4% |
| 03 | [3: Select objects based on value of variable in object using jq](../tests/stack-overflow/03-select-objects-based-on-value-of-variable-in-object-using-jq.json) | 1.83 | 1.22 | -33.3% | 2.14 | +17.1% |
| 04 | [4: Using jq to parse and display multiple fields in a json serially](../tests/stack-overflow/04-using-jq-to-parse-and-display-multiple-fields-in-a-json-seri.json) | 1.83 | 1.81 | -0.9% | 1.38 | -24.8% |
| 05 | [5: jq: how to filter an array of objects based on values in an inner array?](../tests/stack-overflow/05-jq-how-to-filter-an-array-of-objects-based-on-values-in-an-i.json) | 1.81 | 2.84 | +56.9% | 3.70 | +104.3% |
| 06 | [6: How to count items in JSON object using command line?](../tests/stack-overflow/06-how-to-count-items-in-json-object-using-command-line.json) | 1.81 | 3.53 | +94.8% | 1.88 | +3.4% |
| 07 | [7: How to install JQ on Mac on the command line?](../tests/stack-overflow/07-how-to-install-jq-on-mac-on-the-command-line.json) | 1.84 | 3.11 | +68.6% | 2.48 | +34.7% |
| 08 | [8: How do I select multiple fields in jq?](../tests/stack-overflow/08-how-do-i-select-multiple-fields-in-jq.json) | 1.83 | 1.84 | +0.9% | 1.33 | -27.4% |
| 09 | [9: How to get key names from JSON using jq](../tests/stack-overflow/09-how-to-get-key-names-from-json-using-jq.json) | 1.83 | 2.11 | +15.4% | 3.94 | +115.4% |
| 10 | [10: JQ: Select multiple conditions](../tests/stack-overflow/10-jq-select-multiple-conditions.json) | 1.81 | 4.81 | +165.5% | 1.83 | +0.9% |
| 11 | [11: Passing bash variable to jq](../tests/stack-overflow/11-passing-bash-variable-to-jq.json) | 2.80 | 1.64 | -41.3% | 1.81 | -35.2% |
| 12 | [12: How to merge 2 JSON objects from 2 files using jq?](../tests/stack-overflow/12-how-to-merge-2-json-objects-from-2-files-using-jq.json) | 1.81 | 1.22 | -32.8% | 1.81 | +0.0% |
| 13 | [13: How to convert arbitrary simple JSON to CSV using jq?](../tests/stack-overflow/13-how-to-convert-arbitrary-simple-json-to-csv-using-jq.json) | 1.33 | 1.81 | +36.5% | 1.83 | +37.6% |
| 14 | [14: How to use `jq` in a shell pipeline?](../tests/stack-overflow/14-how-to-use-jq-in-a-shell-pipeline.json) | 1.81 | 1.84 | +1.7% | 1.83 | +0.9% |
| 15 | [15: How do I update a single value in a json document using jq?](../tests/stack-overflow/15-how-do-i-update-a-single-value-in-a-json-document-using-jq.json) | 3.94 | 6.84 | +73.8% | 1.81 | -54.0% |
| 16 | [16: how to parse a JSON String with jq (or other alternatives)?](../tests/stack-overflow/16-how-to-parse-a-json-string-with-jq-or-other-alternatives.json) | 3.00 | 1.88 | -37.5% | 1.62 | -45.8% |
| 17 | [17: Using jq or alternative command line tools to compare JSON files](../tests/stack-overflow/17-using-jq-or-alternative-command-line-tools-to-compare-json-f.json) | 1.80 | 1.89 | +5.2% | 1.81 | +0.9% |
| 18 | [18: Get outputs from jq on a single line](../tests/stack-overflow/18-get-outputs-from-jq-on-a-single-line.json) | 1.83 | 1.91 | +4.3% | 1.83 | +0.0% |
| 19 | [19: jq to replace text directly on file (like sed -i)](../tests/stack-overflow/19-jq-to-replace-text-directly-on-file-like-sed-i.json) | 1.91 | 2.95 | +54.9% | 1.81 | -4.9% |
| 20 | [20: Concat 2 fields in JSON using jq](../tests/stack-overflow/20-concat-2-fields-in-json-using-jq.json) | 1.91 | 2.94 | +54.1% | 1.75 | -8.2% |
| 21 | [21: How to sort a json file by keys and values of those keys in jq](../tests/stack-overflow/21-how-to-sort-a-json-file-by-keys-and-values-of-those-keys-in-.json) | 2.58 | 1.33 | -48.5% | 1.83 | -29.1% |
| 22 | [22: Modify a key-value in a json using jq in-place](../tests/stack-overflow/22-modify-a-key-value-in-a-json-using-jq-in-place.json) | 1.78 | 1.64 | -7.9% | 1.81 | +1.8% |
| 23 | [23: How to format a JSON string as a table using jq?](../tests/stack-overflow/23-how-to-format-a-json-string-as-a-table-using-jq.json) | 1.78 | 2.33 | +30.7% | 1.81 | +1.8% |
| 24 | [24: jq: print key and value for each entry in an object](../tests/stack-overflow/24-jq-print-key-and-value-for-each-entry-in-an-object.json) | 1.84 | 1.80 | -2.5% | 1.81 | -1.7% |
| 25 | [25: Extract a specific field from JSON output using jq](../tests/stack-overflow/25-extract-a-specific-field-from-json-output-using-jq.json) | 3.62 | 3.69 | +1.7% | 3.77 | +3.9% |
| 26 | [26: How to run jq from gitbash in Windows?](../tests/stack-overflow/26-how-to-run-jq-from-gitbash-in-windows.json) | 1.81 | 4.78 | +163.8% | 1.81 | +0.0% |
| 27 | [27: jq: output array of json objects](../tests/stack-overflow/27-jq-output-array-of-json-objects.json) | 2.69 | 1.22 | -54.7% | 1.83 | -32.0% |
| 28 | [28: Add new element to existing JSON array with jq](../tests/stack-overflow/28-add-new-element-to-existing-json-array-with-jq.json) | 1.81 | 4.89 | +169.8% | 1.77 | -2.6% |
| 29 | [29: How do I use jq to convert number to string?](../tests/stack-overflow/29-how-do-i-use-jq-to-convert-number-to-string.json) | 1.83 | 2.88 | +57.3% | 1.83 | +0.0% |
| 30 | [30: get the first (or n&#39;th) element in a jq json parsing](../tests/stack-overflow/30-get-the-first-or-n-39-th-element-in-a-jq-json-parsing.json) | 2.58 | 2.12 | -17.6% | 1.84 | -28.5% |
| 31 | [31: Can I pass a string variable to jq rather than passing a file?](../tests/stack-overflow/31-can-i-pass-a-string-variable-to-jq-rather-than-passing-a-fil.json) | 2.69 | 1.86 | -30.8% | 1.81 | -32.6% |
| 32 | [32: Iterating through JSON array in Shell script](../tests/stack-overflow/32-iterating-through-json-array-in-shell-script.json) | 1.81 | 1.34 | -25.9% | 1.81 | +0.0% |
| 33 | [33: How to check for presence of &#39;key&#39; in jq before iterating over the values](../tests/stack-overflow/33-how-to-check-for-presence-of-39-key-39-in-jq-before-iteratin.json) | 1.86 | 1.83 | -1.7% | 1.83 | -1.7% |
| 34 | [34: How to filter array of objects by element property values using jq?](../tests/stack-overflow/34-how-to-filter-array-of-objects-by-element-property-values-us.json) | 1.80 | 1.22 | -32.2% | 1.83 | +1.7% |
| 35 | [35: jq Conditional output](../tests/stack-overflow/35-jq-conditional-output.json) | 2.69 | 1.64 | -39.0% | 1.78 | -33.7% |
| 36 | [36: jq: Cannot index array with string](../tests/stack-overflow/36-jq-cannot-index-array-with-string.json) | 1.80 | 2.95 | +64.3% | 1.66 | -7.8% |
| 37 | [37: Install jq JSON processor on Ubuntu 10.04](../tests/stack-overflow/37-install-jq-json-processor-on-ubuntu-10-04.json) | 1.66 | 1.80 | +8.5% | 1.80 | +8.5% |
| 38 | [38: How to combine the sequence of objects in jq into one object?](../tests/stack-overflow/38-how-to-combine-the-sequence-of-objects-in-jq-into-one-object.json) | 2.92 | 1.83 | -37.4% | 1.80 | -38.5% |
| 39 | [39: jq not working on tag name with dashes and numbers](../tests/stack-overflow/39-jq-not-working-on-tag-name-with-dashes-and-numbers.json) | 1.81 | 2.94 | +62.1% | 2.66 | +46.6% |
| 40 | [40: Convert string to json in jq](../tests/stack-overflow/40-convert-string-to-json-in-jq.json) | 1.64 | 1.81 | +10.5% | 1.81 | +10.5% |
| 41 | [41: Output specific key value in object for each element in array with jq for JSON](../tests/stack-overflow/41-output-specific-key-value-in-object-for-each-element-in-arra.json) | 1.83 | 12.91 | +606.0% | 1.78 | -2.6% |
| 42 | [42: jq: how to query for array values that don&#39;t contain text &quot;foo&quot;?](../tests/stack-overflow/42-jq-how-to-query-for-array-values-that-don-39-t-contain-text-.json) | 2.00 | 2.95 | +47.7% | 1.72 | -14.1% |
| 43 | [43: How do I keep colors when piping &quot;jq&quot; output to &quot;less&quot;?](../tests/stack-overflow/43-how-do-i-keep-colors-when-piping-quot-jq-quot-output-to-quot.json) | 1.69 | 1.80 | +6.5% | 1.64 | -2.8% |
| 44 | [44: Exclude column from jq json output](../tests/stack-overflow/44-exclude-column-from-jq-json-output.json) | 1.33 | 1.81 | +36.5% | 1.80 | +35.3% |
| 45 | [45: How to use jq when the variable has reserved characters?](../tests/stack-overflow/45-how-to-use-jq-when-the-variable-has-reserved-characters.json) | 1.81 | 6.84 | +277.6% | 1.83 | +0.9% |
| 46 | [46: How to extract a field from each object in an array with jq?](../tests/stack-overflow/46-how-to-extract-a-field-from-each-object-in-an-array-with-jq.json) | 3.45 | 2.12 | -38.5% | 1.80 | -48.0% |
| 47 | [47: How to check if element exists in array with jq](../tests/stack-overflow/47-how-to-check-if-element-exists-in-array-with-jq.json) | 1.34 | 1.64 | +22.1% | 3.95 | +194.2% |
| 48 | [48: jq select value from array](../tests/stack-overflow/48-jq-select-value-from-array.json) | 3.34 | 4.31 | +29.0% | 1.78 | -46.7% |
| 49 | [49: getting all the values of an array with jq](../tests/stack-overflow/49-getting-all-the-values-of-an-array-with-jq.json) | 1.81 | 1.88 | +3.4% | 1.83 | +0.9% |
| 50 | [50: Using jq with bash to run command for each object in array](../tests/stack-overflow/50-using-jq-with-bash-to-run-command-for-each-object-in-array.json) | 1.33 | 1.80 | +35.3% | 3.38 | +154.1% |

## Findings

- Mean per-scenario median time was 75.023 ms for jq, 76.937 ms for yq,
  and 74.173 ms for tq.
- tq was faster than jq in 31 of 50 scenarios.
- Mean peak RSS was 2.05 MiB for jq, 2.70 MiB for yq, and 2.00 MiB for tq.
- These are small, single-document inputs. Process startup and fixed runtime
  costs dominate, so this is a compatibility smoke test rather than a
  large-data throughput result.

Raw measurements are retained in `benchmarks/.work/stack-overflow.json`.
