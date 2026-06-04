# borrow_port — Phase A1 (transpile clean) + Phase A2 BLOCKED on trait emit

`library/alloc/src/borrow.rs` (524 LOC source) → `borrow_port.cppm.wip`
(3870 LOC C++). Transpiles cleanly (0 errors), 4 hand-port slots.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/alloc/src/borrow.rs` |
| 2. Prep | ✅ `prep.sh` normalises `core::` → `std::`, `crate::` → `std::` |
| 3. Transpile | ✅ Zero errors, 4 hand-slots |
| 4. Compile | 🔴 13 errors in 5 categories — Phase A2 blocked |
| 5. Build | 🔴 |

## What `borrow_port` exports

- `borrow::Cow<'a, B>` — clone-on-write wrapper (enum: Borrowed/Owned)
- `borrow::ToOwned` — trait for "produce an owned version of borrowed data"
- Re-exports `core::borrow::{Borrow, BorrowMut}` (small wrappers around `&T` / `&mut T`)
- `Add` / `AddAssign` impls for `Cow<'a, str>`

## 13 compile errors, by category

After `clang++ -std=c++23 -fmodules -c borrow_port.cppm`:

| # | Site | Error | Tractability |
|---:|---|---|---|
| 1 | L341 (GMF prelude) | `no template named 'Vec' in namespace 'rusty'` | ✅ Patcher: stub `visit_byte_buf` (binary_heap_tests_port pattern) |
| 3-5 | L3743 | `no member named 'rusty_ext' in global namespace` | ✅ Patcher: namespace qualification |
| 8-13 | L3845-3850 | Orphan-impl methods of `T` at namespace scope | ✅ Patcher: `#if 0` block (rc_port pattern) |
| 2 | L3713 | `typename B::Owned` — `&str` (=`string_view`) has no nested `Owned` typedef | 🔴 **Trait machinery gap** |
| 6 | L3787 | `ToOwned::Owned::default_()` — uses `ToOwned` (a trait) as a class | 🔴 **Trait machinery gap** |

## The trait machinery gap

`Cow_Owned<B>` is emitted as `struct Cow_Owned<B> { typename B::Owned _0; };`.
This assumes every type `B` that goes into a `Cow` carries a nested
`Owned` typedef — exactly the associated type from Rust's `ToOwned`
trait. In Rust the compiler resolves `B::Owned` via the impl table.
In our emit, the compiler can't.

`Cow<B>::default_()` similarly emits `ToOwned::Owned::default_()` —
using `ToOwned` as if it were a class with a nested `Owned` type
member. This is a different shape of the same bug: the transpiler
hasn't decided how to render associated-type access through a trait
bound.

These are the same transpiler-side limits that block parts of
`btree_port`, `vec_port`, etc. when they hit trait-heavy code paths —
but borrow's surface IS predominantly trait/associated-type
machinery, so the limit shows up everywhere instead of in isolated
hot spots.

## Paths forward

There are three credible routes, each with different costs and
different "purity" trade-offs against the project's transpilation
philosophy:

**A. Patcher + ToOwnedTraits adapter** (~1 day, partial transpile)
Add a `template <typename B> struct ToOwnedTraits { using Owned = B; }`
helper in `rusty::borrow`, with explicit specialisations for the
well-known shapes (`string_view → string`, `[]T → Vec<T>`, …).
Patcher rewrites `typename B::Owned` → `typename
ToOwnedTraits<B>::Owned`. Rewrites `ToOwned::Owned::default_()` →
`typename ToOwnedTraits<B>::Owned{}`. Lossy: only specialised
B's actually work; other B's fall through to `Owned = B`, which is
incorrect for any `?Sized` type but matches what consumers want
~90% of the time.

**B. Hand-port** (~1 week, no transpile)
Same as `rusty::String` in `include/rusty/string.hpp`. Write a
~150-LOC hand-written `borrow.hpp` defining `borrow::Cow<B>` +
`borrow::ToOwned` concept directly. Drop the transpiled .wip.
Honest, but breaks the "transpile rustc" philosophy of the project.

**C. Fix the transpiler** (~weeks, real transpile)
Teach the transpiler to emit associated-type access through a
`ToOwnedTraits`-style template-specialisation indirection
automatically, instead of `typename B::Owned`. This is the path that
generalises — `core::str`, `core::ascii`, and many future
trait-heavy ports would benefit. But it's substantial codegen work.

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — `core::` → `std::`, `crate::` → `std::` normalisation
- `transpiled/borrow_port/borrow_port.cppm.wip` — clean transpile,
  parked until Phase A2 path is chosen
