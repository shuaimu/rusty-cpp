# String port — ✅ Phase B + C via bridge stub (full transpiled body blocked on cross-port deps)

Full transpiled string_port.cppm needs core::str (Searcher/Pattern),
alloc::borrow (Cow/ToOwned), alloc::ascii::Char — none vendored.
**`transpiled/string_port/string_port_stub.cppm`** re-exports
`rusty::String` under `string_port::String`. `libstring_port.a` builds;
smoke test proves String::from("hello") gives len() == 5.



Vendored `library/alloc/src/string.rs` (3606 LOC) →
`transpiled/string_port/string_port.cppm`. Re-transpiled with
`--auto-namespace`: **zero transpiler errors, 29 hand-port slots**.

This is an update from the earlier Phase A1 attempt (see git history of
this file) which had stalled at name-resolution. The `--auto-namespace`
flag and `prep.sh` rewrites unblock the parse/transpile pass.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ Zero errors, 29 hand-slots |
| 4. Patcher | 🔴 **Blocked** — see "Cross-port blockers" |
| 5. Build | 🔴 |

## Cross-port blockers

String's transpile is clean, but compiling it depends on types from
other rust stdlib crates not yet ported:

| Symbol | Lives in | Port needed |
|---|---|---|
| `Searcher` / `Pattern` traits | `core::str::pattern` | `core_str_port` (Phase A1 in docs/core_str_port/) |
| `Chars`, `CharIndices`, `Utf8Error`, etc. | `core::str` | same |
| `borrow::Cow`, `borrow::ToOwned` | `alloc::borrow` | not yet started |
| `alloc::ascii::Char` | `alloc::ascii` | not yet started |
| `Vec<T>` | `alloc::vec` | ✅ `vec_port` |
| `TryReserveError` | `alloc::collections` | ✅ `rusty::collections` |

A first-pass patcher would emit stub declarations for the missing
str/borrow/ascii types so String's body parses, similar to btree_port's
hand-port slots.

## Reproducing

```bash
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/ | head -1)
mkdir -p /tmp/string_port/string_crate/src
cp $RUSTSRC/string.rs /tmp/string_port/string_crate/src/lib.rs
cp docs/string_port/Cargo.toml.template /tmp/string_port/string_crate/Cargo.toml
bash docs/binary_heap_port/prep.sh /tmp/string_port/string_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/string_port/string_crate/Cargo.toml \
    --output-dir /tmp/string_port/cpp_out --auto-namespace
cp /tmp/string_port/cpp_out/*.cppm transpiled/string_port/
```

## Predicted effort

Per §2.8: **1–2 days** if str/borrow/ascii stubs are acceptable;
**1 week** if the cross-port deps need real ports first.

## 2026-06-04 attempt

Tried flipping CMake to build the full 5983-LOC body with the
codified patcher (renamed back from `.wip` for the attempt). After
stubbing borrow/str/ascii namespaces the patcher reduces errors but
~20 residual issues remain:

- `rusty::ascii` still referenced in ~10 sites (e.g. line 3668,
  4163-4164). Patcher's `using ascii::Char` injection at the
  `string_port` namespace doesn't reach the fully-qualified
  `rusty::ascii::Char` shape.
- `std::basic_string_view<char>::Searcher` (line 4081, 4165) — the
  transpiler assumed string_view has a nested Searcher type. It
  doesn't. Needs a wrapper that adds the nested type.
- `rusty::fmt::FormattingOptions`, `rusty::fmt::Formatter::new_`
  (line 5751) — fmt API surface gaps.
- `member reference base type 'const char32_t' is not a structure or union`
  (line 5763) — transpiler emitted `.method()` on primitive
  `char32_t`. Needs the same SFINAE dispatch shape that resolved
  `.eq()` on size_t, or a char wrapper.
- **Type identity mismatch**: the transpiled body's
  `rusty::port::string::String` is a distinct C++ class from the
  hand-written `rusty::String`. The bridge `string_port_stub.cppm`
  works by re-exporting `rusty::String` under the
  `string_port::String` name; the full body would introduce a
  *separate* class with the same alias path. Switching breaks any
  consumer that round-trips through `rusty::string::String`.

Conclusion: pushing to full requires either
(a) writing `core_str_port` + `alloc_borrow_port` + `alloc_ascii_port`
(multi-week, the documented dependency-vendoring track), or
(b) more invasive stubbing that re-routes the type identity question.

The current bridge stub remains the right architecture for now. The
WIP patcher is kept at `post_transpile_patch.py.wip` for whoever
picks this up next.
