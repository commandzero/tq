## 1. Executable Baselines and Front End

- [x] 1.1 Add jq differential cases for definitions, parameters, scope, shadowing, recursion, imports, metadata, cycles, and failures
- [x] 1.2 Implement source-spanned `def`, `include`, and `import` grammar
- [x] 1.3 Resolve definitions, captures, symbols, arity, and recursive references before input

## 2. Managed Calls and Modules

- [x] 2.1 Define and validate the user-call bytecode ABI and managed call-frame limits
- [x] 2.2 Compile and execute parameterized generator calls without native recursion
- [x] 2.3 Implement confined module roots, canonical resolution, digest caching, and cycle diagnostics

## 3. Release Evidence

- [x] 3.1 Add hostile module/resource tests, fuzz coverage, explain output, and cleanup regressions
- [x] 3.2 Run full compatibility and performance campaigns and publish module-path documentation
