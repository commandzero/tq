# Benchmark evidence

The focused `standard` campaign ran on 2026-08-31 against the frozen
`usgs-all-month` JSON corpus of 7,877,699 bytes, using release builds and ten
samples per tool and case. The raw report is
`benchmarks/.work/is8-standard-fixed.json` and is intentionally ignored as
local campaign output.

The first elevated RSS run found that document decoding built a complete
`serde_json::Value` and then converted it into tq's `Value`, keeping both trees
alive near peak memory. JSON now deserializes directly into tq's existing
ordered, arbitrary-precision-aware value visitor.

| Case | jq median | tq median | tq / jq | Goal |
| --- | ---: | ---: | ---: | ---: |
| Direct call | 124,602.5 us | 178,439.5 us | 1.432 | <= 2.0 |
| `map` callback | 119,603.5 us | 114,836 us | 0.960 | <= 2.0 |
| `select` callback | 118,765.5 us | 121,192.5 us | 1.020 | <= 2.0 |
| `sort_by` callback | 165,276.5 us | 171,245.5 us | 1.036 | <= 2.0 |

| Case | jq peak RSS | tq peak RSS | tq / jq | Goal |
| --- | ---: | ---: | ---: | ---: |
| Direct call | 65,814,528 bytes | 75,317,248 bytes | 1.144 | <= 1.5 |
| `map` callback | 65,191,936 bytes | 75,120,640 bytes | 1.152 | <= 1.5 |
| `select` callback | 65,191,936 bytes | 75,464,704 bytes | 1.158 | <= 1.5 |
| `sort_by` callback | 68,190,208 bytes | 77,283,328 bytes | 1.133 | <= 1.5 |

All measured wall-time and peak-RSS goals pass. RSS came from an elevated
macOS campaign with complete child-process-group inspection enabled.
