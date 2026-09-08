# Modules coverage audit

Source: [jq manual, Modules](https://jqlang.org/manual/#modules). The audit covers the sole fenced block plus every concrete module/path/metadata example and syntax signature in the prose. Module fixtures live under `tests/fixtures/manual-modules/`.

| Source line | Example | Cases | Disposition |
| ---: | --- | --- | --- |
| 16 | module system | `manual.modules.import` | new |
| 18 | import/include search paths | `manual.modules.include` | new |
| 20 | path substitutions | `manual.modules.path-dot`, `manual.modules.path-tilde`, `manual.modules.path-origin` | adapted |
| 22 | `~/` substitution | `manual.modules.path-tilde` | adapted |
| 24 | `$ORIGIN/` substitution | `manual.modules.path-origin` | adapted |
| 26 | `./` and including-file paths | `manual.modules.path-dot`, `manual.modules.path-dot-including` | adapted/new |
| 28 | import search metadata | `manual.modules.import-search` | new |
| 30 | default search path | `manual.modules.default-search-path` | new |
| 31 | fenced default path value | `manual.modules.default-search-path` | new |
| 37 | null/empty search termination | `manual.modules.search-terminator` | adapted |
| 39 | `foo/bar` lookup candidates | `manual.modules.relative-single`, `manual.modules.relative-directory` | new |
| 41 | repeated components | `manual.modules.repeated-component` | new |
| 43 | `foo.jq` and `foo/foo.jq` | `manual.modules.foo-single`, `manual.modules.foo-directory` | new |
| 45 | automatic HOME `.jq` source | `manual.modules.home-auto-source` | adapted |
| 47–53 | import syntax, namespace, metadata, search | `manual.modules.import`, `manual.modules.import-metadata`, `manual.modules.import-search` | new/covered |
| 55–59 | include syntax, scope, metadata | `manual.modules.include`, `manual.modules.include-metadata` | new/covered |
| 61–67 | JSON import and metadata/search | `manual.modules.import-json` | new/covered |
| 69–73 | module metadata directive and constant rule | `manual.modules.modulemeta`, `manual.modules.metadata-constant` | new |
| 75–79 | `modulemeta` shape and program use | `manual.modules.modulemeta`, `manual.modules.modulemeta-deps` | new/covered |

The jq-target cases deliberately keep tq enabled. Tilde, `$ORIGIN`, HOME auto-sourcing, repeated path components, empty roots, and JSON-module imports therefore appear as explicit tq compatibility differences where behavior is currently restricted or unsupported. Result-sequence cases retain the harness’s native TOON output handling.
