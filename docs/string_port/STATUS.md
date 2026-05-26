# String port — Phase A1 only

This directory holds the scaffolding for the rustc `alloc::string`
port. As of this commit only the Phase A1 (parse) step succeeded;
all later phases are blocked on namespace remapping work.

## Reproducing the parse attempt

```bash
# Vendor the source
mkdir -p /tmp/string_port/string_crate/src
cp ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/string.rs \
   /tmp/string_port/string_crate/src/lib.rs
cp docs/string_port/Cargo.toml.template /tmp/string_port/string_crate/Cargo.toml

# Transpile (zero errors)
./target/release/rusty-cpp-transpiler \
    --crate /tmp/string_port/string_crate/Cargo.toml \
    --output-dir /tmp/string_port/cpp_out
```

Output: one `.cppm` file, ~3600 lines of input → ~3700 lines of C++.

## Blockers for Phase A2 (compile)

The transpiler emits 20 errors at first build, all of one shape:
unqualified module-path identifiers that aren't in scope.

| Identifier | Should resolve to | Status |
|------------|-------------------|--------|
| `rusty::ascii` | namespace `rusty::ascii` (does not exist) | missing |
| `std::Allocator` | `rusty::alloc::Allocator` | bad path |
| `::borrow::Cow` | `rusty::borrow::Cow` | missing namespace |
| `::collections::TryReserveError` | `rusty::collections::TryReserveError` | bad qual |
| `::str::CharIndices`, `::str::Chars`, etc. | `rusty::str::*` | namespace missing |
| `::vec::Vec` | `rusty::Vec` | namespace + alias |

Two distinct problems hide here:

1. **The transpiler isn't applying `rusty::` prefix for top-level
   modules from std.** For Vec we patched these case-by-case
   (V-A cluster). The fix lives in `post_transpile_patch.py` —
   add an analogue for the string surface.

2. **Hand-port collision.** `include/rusty/string.hpp` defines a
   stand-alone `rusty::String` class. A transpiled `String` would
   redeclare in the same namespace. Two paths:
     - Wrap the transpile output in `namespace string_port`
       (cheap; requires a `using rusty::String = string_port::String;`
       alias somewhere so user code stays the same).
     - Replace `rusty::String` with the transpiled version, retire
       the hand-port (better long-term but more work — the
       hand-port has API surface the transpiled module is missing,
       e.g. `String::from(const char*)`).

## Phase plan when work resumes

- [x] A1 — parse (0 errors)
- [ ] A2 — compile: namespace patches + decide on collision strategy
- [ ] B  — hand-port anything that's missing (`str::` types from
            `core/src/str/`)
- [ ] C  — smoke test: construct + push_str + format
- [ ] E1 — completeness vs hand-port
- [ ] E2 — bench vs `std::string`

Effort estimate (per playbook §2.8): half a day if we wrap in
`string_port::` namespace; one day if we replace the hand-port.

## Why pause here

The Vec port (Chapter 4 of the book) was the centerpiece of
Phase 1 and is complete with 16 operations, bench data, and a
written retrospective. String adds combinatorial complexity from
the str/ascii dependency on `core` (which has its own port
surface). Stopping here records a clean cut point.
