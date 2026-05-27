# core::str::Pattern port — Phase A1 (scaffolding)

The String port (docs/string_port/) is blocked on `str::Pattern +
str::Searcher` — a Rust trait family that doesn't transpile cleanly.
This directory holds the scaffolding for porting it.

## Reproducing

```bash
mkdir -p /tmp/core_str_port/core_str_crate/src
cp ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/str/pattern.rs \
   /tmp/core_str_port/core_str_crate/src/lib.rs
cp docs/core_str_port/Cargo.toml.template /tmp/core_str_port/core_str_crate/Cargo.toml

./target/release/rusty-cpp-transpiler \
    --crate /tmp/core_str_port/core_str_crate/Cargo.toml \
    --output-dir /tmp/core_str_port/cpp_out
```

Output: 1983-line `pattern.rs` → 20 build errors at first cmake.

## First-pass error catalogue

| Error shape | Count | Notes |
|-------------|-------|-------|
| `cmp` / `slice` unqualified namespace | ~2 | Same as Vec's V-A cluster |
| `CharIndices`/`Chars` partial in `rusty::str_runtime` | ~6 | rusty has the types but missing methods (`iter`, `next_back`) |
| Stray `char_` identifier, `this` outside member fn | ~5 | Rust raw-keyword emit bugs |
| Misc emit issues | ~7 | TBD; need full triage |

## Why pause here

Pattern.rs alone is 1983 lines, and the broader `core::str` surface
(mod.rs + iter.rs + traits.rs + validations.rs) totals ~7000 lines —
substantially more than Vec's ~2000. Realistic effort: 2–3 days of
iteration following the Vec playbook (clusters, patcher, tests).

This file documents the entry point so a future session can pick up
without re-discovering the scope. The pattern (no pun intended) is
clear: vendor → prep → transpile → catalogue errors → write patcher →
write smoke tests → close phase B (instantiation runtime). Tied to
String port's resumption in `docs/string_port/STATUS.md`.

## Smarter alternative

Rather than transpiling `core::str::pattern` faithfully, hand-port
just the surface that `String` actually uses (`find`, `starts_with`,
`contains`, `split`, `replace`). That subset is ~5 functions and
takes a substring or char as the haystack/needle — no full Pattern
trait machinery. ~½ day of focused work vs 2–3 days transpiling.

When this work resumes, evaluate that trade-off first. The
transpile-everything approach is "faithful but slow"; the hand-port-
surface approach is "pragmatic and unblocks String fastest".
