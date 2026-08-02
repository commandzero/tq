## 1. Reference and Dependency Decisions

- [ ] 1.1 Baseline jq regex syntax, Unicode, offsets, captures, flags, substitutions, and limit behavior
- [ ] 1.2 Baseline UTC/local date ranges and environment/platform built-ins on release hosts
- [ ] 1.3 Select and record regex/time dependencies plus explicit divergence and capability policy

## 2. Built-in Implementation

- [ ] 2.1 Implement bounded regex compilation, match/capture/scan/split/substitution built-ins
- [ ] 2.2 Implement UTC parsing, formatting, epoch conversion, and stable range diagnostics
- [ ] 2.3 Implement opt-in environment/platform I/O with redaction and path/resource governance

## 3. Release Evidence

- [ ] 3.1 Add hostile regex/time/I/O tests, fuzz targets, and cross-platform manifests
- [ ] 3.2 Run compatibility/performance campaigns and publish all platform divergences
