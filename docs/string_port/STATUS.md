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

First pass: 20 errors of one shape — unqualified module-path
identifiers. I patched those with stubs to see what's underneath:

| Identifier | Patch applied |
|------------|---------------|
| `using std::Allocator;` | commented out |
| `::borrow::Cow` / `::borrow::ToOwned` | stubbed |
| `::collections::TryReserveError` | aliased to `rusty::collections::TryReserveError` |
| `::str::Chars`, `CharIndices`, `Utf8Error`, `FromStr`, `from_utf8_unchecked_mut`, `from_boxed_utf8_unchecked` | stubbed (no rusty::str namespace) |
| `::vec::Vec` | aliased to `rusty::Vec` |
| `rusty::ascii::Char` | replaced with `uint8_t` |

Second pass: 20 *different* errors now. The shape of remaining
blockers is heavier:

| Error | Why hard |
|-------|----------|
| `rusty::Vec` requires template args | trivial fix |
| `Searcher` not in `std::basic_string_view` | needs Pattern/Searcher trait infrastructure that has no C++ analogue |
| unqualified `collections::` references | one more pass of namespace rewrites |
| `std::str` doesn't exist | str/Chars/CharIndices need ports of their own (from `core/src/str/`) |
| `this` outside non-static member function | transpiler emit bug |
| `const` qualifier on non-member function | transpiler emit bug |
| class member redeclared | template specialization mishap |

Two distinct problem clusters underneath:

1. **The transpiler isn't applying `rusty::` prefix for top-level
   modules.** Patchable case-by-case (Vec played this game in
   V-A cluster). Maybe ~40 occurrences in string_port; mechanical.

2. **The str/Pattern dependency tree.** `String` calls into
   `str::Pattern::Searcher`, which is its own trait surface (also
   under `core/src/str/pattern.rs`). Porting String fully would
   chain to porting `core::str` — that's a project of its own.
   The transpiler can't synthesize Pattern/Searcher; would need
   either hand-port stubs or a separate `core_str_port` crate.

3. **Hand-port collision.** `include/rusty/string.hpp` already
   defines `rusty::String`. The transpile would need to live in
   `string_port::String` then alias.

## Phase plan when work resumes

- [x] A1 — parse (0 errors)
- [ ] A2 — compile: namespace patches + decide on collision strategy
- [ ] B  — hand-port anything that's missing (`str::` types from
            `core/src/str/`)
- [ ] C  — smoke test: construct + push_str + format
- [ ] E1 — completeness vs hand-port
- [ ] E2 — bench vs `std::string`

Effort estimate (per playbook §2.8): revised after second pass —
this is **2–3 days minimum**, not half a day. The Pattern/Searcher
dependency was hidden underneath the namespace errors. A "core
str port" prerequisite has to land first; that's its own Tier 1
project. The original §3.8 estimate of ½ day for String was
optimistic — it assumed Pattern/Searcher were already available.

## Why pause here

The Vec port (Chapter 4 of the book) was the centerpiece of
Phase 1 and is complete with 16 operations, bench data, and a
written retrospective. String adds combinatorial complexity from
the str/ascii dependency on `core` (which has its own port
surface). Stopping here records a clean cut point.
