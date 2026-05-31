# Cell / RefCell port — Phase C (smoke test passing)

Vendored `library/core/src/cell.rs` (2737 LOC) →
`transpiled/cell_port/cell_port.cppm`. Patcher pipeline lands 8
classes of post-transpile edits + new Rust-trait stubs in
`include/rusty/{ops,marker,fmt,pin,panic}.hpp` to unblock the cross-
crate `using` imports.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ |
| 4. Patcher | ✅ 8 patches in `post_transpile_patch.py` |
| 5. Build | ✅ `libcell_port.a` (Clang 19 + C++23) |
| 6. Smoke test | ✅ `tests/cell_port_module_test.cpp` — imports module, builds `BorrowError`/`BorrowMutError`, formats via `operator<<` |

## Trait stubs introduced

Rust's `core::ops::Deref`, `core::marker::PhantomData`,
`core::fmt::Debug` etc. don't have direct C++ analogues — they're
trait declarations referenced via `using rusty::ops::Deref;` etc. in
the transpiled module. Phase B added empty-marker stub types in:

| Header | New types |
|---|---|
| `include/rusty/ops.hpp` (new file) | `Deref`/`DerefMut`/`DerefPure`/`CoerceUnsized`/`DispatchFromDyn`/`Drop`/`Fn`/`FnMut`/`FnOnce`/`Add`/`Sub`/`Mul`/`Div`/`Neg`/`Not`/`BitAnd`/`BitOr`/`BitXor`/`Shl`/`Shr`/`Rem` (+ all `*Assign` variants), `Index`/`IndexMut`, range markers, `Try`/`FromResidual` |
| `include/rusty/marker.hpp` (extended) | `marker::Copy`/`Sized`/`Send`/`Sync`/`Unpin`/`Destruct`/`Unsize`/`PhantomPinned` |
| `include/rusty/fmt.hpp` (extended) | `Debug`/`Display`/`Binary`/`Octal`/`LowerHex`/`UpperHex`/`LowerExp`/`UpperExp`/`Pointer` |
| `include/rusty/pin.hpp` (new file) | `pin::PinCoerceUnsized`/`PinDerefMut` |
| `include/rusty/panic.hpp` (extended) | `panic::Location` (with static `caller()`), `panic::const_panic` |

`rusty::pin::Pin<T>` is intentionally **not** stubbed — the transpiled
code emits its own `template<typename T> using Pin = T;` alias inside
its own `pin` namespace (which auto-namespace mode lands in
`rusty::pin` due to the surrounding `namespace rusty {` wrap), and a
duplicate definition would clash.

## Patcher patches (8)

1. **Namespace `using ::ns::X;` → `using rusty::ns::X;`** for cmp/fmt/marker/mem/ops/ptr/iter/hash/panic/pin. Also rewrites `::panic::Location` type references in declarations.
2. **`BorrowCounter` alias reorder** — alias was emitted after its first use; pulled it up to the module-purview head.
3. **Drop `::` global qualifier on intra-module helpers** — calls like `::panic_already_borrowed(...)` rewritten to unqualified lookup inside the `cell_port` namespace.
4. **Qualify bare `ptr::FN(...)` helpers** — `ptr::replace`/`ptr::eq`/etc. → `rusty::ptr::FN(...)`.
5. **Fix empty `write!` macro stub lines** — transpiler emits `auto res = /* write!(f, ...) */;`; the comment-as-RHS is a parse error. Replaced with a comment marker since the next statement always recovers via `rusty::write_fmt`.
6. **`borrow_field.get_mut() = UNUSED` → `.set(UNUSED)`** — assignment-through-rvalue-reference fails because `rusty::Cell::get_mut()` is const-qualified; `.set()` is the right surface.
7. **Stub `assert_coerce_unsized` signature** — function was a Rust-only `CoerceUnsized` trait check whose signature instantiated `rusty::UnsafeCell<const int32_t&>`, malformed in our hand-written UnsafeCell. Signature rewritten to take no parameters; body becomes a no-op.
8. **`Option<&Location>` → `Option<Location>`** — our `rusty::Some(x)` helper decays references, so it builds an `Option<Location>` value which can't convert to `Option<const Location&>`. Since `Location` is a one-byte empty marker, by-value and by-ref are observationally identical.
9. **Drop misplaced `import cell_port.lazy/once;`** — transpiler emitted these past the module preamble (ill-formed). The lazy/once submodules also weren't vendored, so the `using lazy::LazyCell;` re-exports were also commented out.

## Known limitations of the port

- No `LazyCell` / `OnceCell` (vendor `library/core/src/cell/lazy.rs`
  and `library/core/src/cell/once.rs` as separate `cell_port.lazy` /
  `cell_port.once` submodules to surface these).
- The smoke test only exercises `BorrowError` / `BorrowMutError`
  construction + formatting. Full `RefCell<T>` usage requires
  surfacing `rusty::Cell<T>::new_(...)` and `rusty::UnsafeCell<T>::new_(...)`
  constructors that the transpiled methods need; the existing
  hand-written `rusty/cell.hpp` and `rusty/unsafe_cell.hpp` are
  scoped differently. Extending those is follow-up work.
- `rusty::panic::Location::caller()` returns a static empty
  Location; rustc would record real `file:line:column`. For
  diagnostic correctness this would need `std::source_location`.

## Reproducing

See §6.9 in the rusty-std-book.

```bash
# from .claude/worktrees/rusty-lib/
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/core/src/ | head -1)
mkdir -p /tmp/cell_port/cell_crate/src
cp $RUSTSRC/cell.rs /tmp/cell_port/cell_crate/src/lib.rs
cp docs/cell_port/Cargo.toml.template /tmp/cell_port/cell_crate/Cargo.toml
bash docs/cell_port/prep.sh /tmp/cell_port/cell_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/cell_port/cell_crate/Cargo.toml \
    --output-dir /tmp/cell_port/cpp_out --auto-namespace
cp /tmp/cell_port/cpp_out/*.cppm transpiled/cell_port/
python3 docs/cell_port/post_transpile_patch.py transpiled/cell_port/
cmake --build build_cell --target cell_port_module_test.out
./build_cell/cell_port_module_test.out
```
