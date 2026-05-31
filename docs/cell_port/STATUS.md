# Cell / RefCell port — Phase A2 partial (namespace patches applied)

Vendored `library/core/src/cell.rs` (2737 LOC) →
`transpiled/cell_port/cell_port.cppm`. Phase A2 patches landed
(transpiler emit had bare `using ::cmp::Foo;` for cross-crate
imports; rewrote to `using rusty::cmp::Foo;` for cmp/fmt/marker/mem/
ops/ptr/iter/hash. Also reordered `using BorrowCounter = ptrdiff_t;`
to come before its first use).

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ |
| 4. Patcher | 🟡 **Partial.** Namespace `using ::ns::` → `using rusty::ns::` rewrites applied + `BorrowCounter` alias reorder. Hits next-layer issue: Rust trait names (`Debug`, `Display`, `CoerceUnsized`, `Deref`, `DerefMut`, `Destruct`, `PhantomData`, `Unsize`, etc.) don't exist as C++ types in `rusty::fmt::`, `rusty::ops::`, `rusty::marker::`. |
| 5. Build | 🔴 Blocked on the trait-stub work above. |

## Remaining Phase B work

The `using rusty::ops::Deref;` etc. fail because Rust's `Deref` etc.
are trait declarations that don't have a C++ analogue. They're
referenced for SFINAE / variance markers but never invoked at runtime.

Two paths to close Phase B:
1. **Stub the trait names** in `rusty/ops.hpp` / `rusty/marker.hpp` /
   `rusty/fmt.hpp` as empty marker types. Each is ~5 lines.
2. **Comment out the `using` declarations** in cell_port that pull in
   undefined Rust traits (the body of cell_port doesn't actually use
   them — they're decorative from the rustc source's `use ops::*;`).

Path 2 is faster; Path 1 is cleaner long-term and would unblock
similar issues in rc_port / arc_port (they reference the same trait
names from `core::ops::*`).

Predicted effort: **2-3 days** for full Phase B (covering Path 1 +
the per-trait stub declarations + cell_port's own internal hand-ports).

## Reproducing

See §6.9 in the rusty-std-book.
