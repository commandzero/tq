## 1. Reference and Dependency Decisions

- [x] 1.1 Baseline jq regex syntax, Unicode, offsets, captures, flags, substitutions, and limit behavior
- [x] 1.2 Baseline UTC/local date ranges and environment/platform built-ins on release hosts
- [x] 1.3 Select and record regex/time dependencies plus explicit divergence and capability policy

## 2. Built-in Implementation

- [x] 2.1 Implement bounded regex compilation, match/capture/scan/split/substitution built-ins
- [x] 2.2 Implement UTC parsing, formatting, epoch conversion, and stable range diagnostics
- [x] 2.3 Implement opt-in environment/platform I/O with redaction and path/resource governance

## 3. Release Evidence

- [x] 3.1 Add hostile regex/time/I/O tests, fuzz targets, and cross-platform manifests
- [x] 3.2 Run compatibility/performance campaigns and publish all platform divergences
