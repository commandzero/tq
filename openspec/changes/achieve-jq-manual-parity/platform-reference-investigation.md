# Platform reference investigation

This records reference discovery and the subsequent native investigation.
The current-source native checkpoint executes all 608 cases, with 602 exact
matches and six separately reviewed disparities. It is not final release
approval: a later user-function composition finding requires another source
revision and fresh executable-bound evidence.

The verified-source report is
`target/linux-review/manual-linux-v2-source-verified.toon`. All fourteen pinned
manual files matched after authorized transfer. Explicit stable Rust compiler
and rustdoc 1.98.0 built tq SHA-256
`f774395e856f103f13eb0d0cf3fc7357d1be2097dc442fd566f5be92136d5e3b`.
The full workspace run passed 720 tests with zero failures and two ignored
reference-only tests. Actual POSIX shell, PTY color, date/ambient-policy, math,
and CPU/RSS benchmark tests pass. The task-local util-linux `script` executable
supplied the missing PTY tool without changing the host's installed packages.

Compact JSON matches 575 of 581 cases; TOON preserves all 581 execution
contracts. The six review entries are `exp(1)`, `tgamma(0.5)`, `y0(1)`,
`yn(0;1)`, the extreme integer-exponent conversion witness, and regex flag `l`.
The native 92-input CLI math probe has 81 exact and 11 differing samples, no
process issues, and unchanged executable/runtime hashes before and after.
See `docs/jq-compatibility-disparities.md` for exact observations and restrictions.

## Linux x86_64 candidate

The available native host reports jq 1.8.1 and glibc 2.43. Its jq executable is
dynamically linked, so the executable hash alone does not identify the reference
implementation. Read-only inspection on 2026-09-08 recorded these SHA-256 hashes:

| Artifact | SHA-256 |
| --- | --- |
| `/usr/bin/jq` | `136748786226819bf582738e8be963638c9d721aa0c5d1d650b506a2a52ddb97` |
| `/usr/lib64/libjq.so.1` | `5de9294d71f67b22f56889348dc424c45ae2167065d5e766eea30ff8d3b011ce` |
| `/usr/lib64/libonig.so.5` | `73a1423e3d1c5f8d1f076376cbbfcf8e4f9dbb2a683121bdfd9353eb714ab7f9` |
| `/usr/lib64/libc.so.6` | `01cccbe278d898add05282986a9346d4dda4b3d2b84bc496f9c04c66016528db` |
| `/usr/lib64/libm.so.6` | `6f7365a58fc0778dbc8869d70cdd4822ab93d879c29e84bb491d447961735e56` |
| `/lib64/ld-linux-x86-64.so.2` | `f0949392f14253241540dfeb152f5bd3cbd75f8a3e4f8d3fabc286a2daa27a68` |

Exact `jq --build-configuration` output, excluding its final LF:

```text
--build=x86_64-redhat-linux-gnu --host=x86_64-redhat-linux-gnu --program-prefix= --disable-dependency-tracking --prefix=/usr --exec-prefix=/usr --bindir=/usr/bin --sbindir=/usr/bin --sysconfdir=/etc --datadir=/usr/share --includedir=/usr/include --libdir=/usr/lib64 --libexecdir=/usr/libexec --localstatedir=/var --runstatedir=/run --sharedstatedir=/var/lib --mandir=/usr/share/man --infodir=/usr/share/info --disable-static build_alias=x86_64-redhat-linux-gnu host_alias=x86_64-redhat-linux-gnu CC=gcc 'CFLAGS=-O2 -flto=auto -ffat-lto-objects -fexceptions -g -grecord-gcc-switches -pipe -Wall -Werror=format-security -Wp,-U_FORTIFY_SOURCE,-D_FORTIFY_SOURCE=3 -Wp,-D_GLIBCXX_ASSERTIONS -specs=/usr/lib/rpm/redhat/redhat-hardened-cc1 -fstack-protector-strong -specs=/usr/lib/rpm/redhat/redhat-annobin-cc1  -m64 -march=x86-64 -mtune=generic -fasynchronous-unwind-tables -fstack-clash-protection -fcf-protection -mtls-dialect=gnu2 -fno-omit-frame-pointer -mno-omit-leaf-frame-pointer  ' 'LDFLAGS=-Wl,-z,relro -Wl,--as-needed  -Wl,-z,pack-relative-relocs -Wl,-z,now -specs=/usr/lib/rpm/redhat/redhat-hardened-ld -specs=/usr/lib/rpm/redhat/redhat-hardened-ld-errors -specs=/usr/lib/rpm/redhat/redhat-annobin-cc1  -Wl,--build-id=sha1 -specs=/usr/lib/rpm/redhat/redhat-package-notes  ' LT_SYS_LIBRARY_PATH=/usr/lib64:
```

The harness validates these runtime-library identities and executes the full
catalog with controlled locale/timezone, retaining matched jq/tq observations.
A jq-only native dependency does not add an FFI dependency to tq.

Read-only jq probes also establish that these are platform contracts, not
universal aliases. Both builds exit 0 with empty stderr for the combined query
`[scalb(2;0.5),scalb(infinite;-infinite),(2|gamma)]` under `-nc`:

| Expression | macOS arm64 jq 1.8.1 | Linux x86_64 jq 1.8.1 |
| --- | --- | --- |
| `scalb(2;0.5)` | `2` | `null` |
| `scalb(infinite;-infinite)` | `1.7976931348623157e+308` | `null` |
| `2|gamma` | `1` | `0` |

tq now maps these verified Linux contracts at compile time using safe Rust:
Linux `gamma` uses the `libm::lgamma` implementation and Linux `scalb`
rejects fractional exponents while following the verified IEEE
zero/infinity cases for infinite exponents. The macOS behavior remains the
non-Linux target path. These jq-only observations do not approve a tq
disparity or establish a general function identity from one input. Existing
safe Rust APIs remain the implementation constraint.

The expanded Linux scalb probe produced the following six verified outcomes;
the macOS column is intentionally unpopulated because those six macOS
invocations were not run:

| Expression | Linux x86_64 jq 1.8.1 |
| --- | --- |
| `scalb(2;infinite)` | `1.7976931348623157e+308` |
| `scalb(2;-infinite)` | `0` |
| `scalb(0;infinite)` | `null` |
| `scalb(0;-infinite)` | `0` |
| `scalb(infinite;infinite)` | `1.7976931348623157e+308` |
| `scalb(infinite;-infinite)` | `null` |

This confirms that only fractional exponents and invalid
infinity-times-zero cases project as null; non-finite exponents are not a
blanket domain error.

The separate integer-exponent conversion probe
`[ldexp(2;2147483648),ldexp(2;2147483647),ldexp(2;-2147483649),ldexp(2;nan),scalbln(2;nan)]`
returned `[1.7976931348623157e+308,1.7976931348623157e+308,0,2,2]` on
macOS and `[0,1.7976931348623157e+308,0,0,0]` on Linux. tq intentionally
retains a deterministic safe saturating conversion rather than reproducing
target-dependent C floating-to-integer conversion.

## Other targets

The macOS arm64 and reviewed Ironhide x86_64 Linux references are now in the
executable pin. Primary review confirmed that adding Linux preserved the entire
source inventory, original cases, and macOS reference. The Linux pin includes
all five runtime-library identities, so the same executable with a different
libc or libjq does not qualify.

The release-host workflow now includes native shell checks and the strict
manual campaign. It requires separate reviewed artifact URL/hash pairs for
Linux and macOS and fails closed without matched executable/runtime evidence.
An Ironhide pin does not establish compatibility with Ubuntu's runtime libraries.
Native Windows execution is explicitly deferred until a runner is available;
its test definitions remain and no Windows compatibility result is claimed.
