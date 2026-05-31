# Rc port — ✅ Phase B + C via bridge stub (full transpiled body still WIP)

The full transpiled `rc_port.cppm` (5094 LOC from `library/alloc/src/rc.rs`)
is blocked on transpiler-side issues catalogued below. To unblock
downstream consumers, **`transpiled/rc_port/rc_port_stub.cppm`** re-exports
the hand-written `rusty::Rc<T>` under the `rc_port::Rc<T, A=Global>`
alias. `librc_port.a` builds today; `tests/rc_port_module_test.cpp`
proves it (constructs `Rc<int>(42)` + clone).

When transpiler fixes land (the ~20 patches in
`post_transpile_patch.py` cover most of the surface), swap the stub
for the full transpiled file and re-run the smoke test.



Vendored `library/alloc/src/rc.rs` (5094 LOC) →
`transpiled/rc_port/rc_port.cppm`. Patcher pipeline started; library
still does not build clean. Phase B requires either substantial
transpiler-side fixes or extensive hand-port work.

## Pipeline summary

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ |
| 2. Prep | ✅ |
| 3. Transpile | ✅ |
| 4. Patcher | 🟡 **1 patch applied** — namespace prefix fixups (`std::borrow::` → comments out, `string::String` → `rusty::String`, `rusty::Vec<>` → `::Vec<>`, `rusty::mem::MaybeUninit` → `rusty::MaybeUninit`). |
| 5. Build | 🔴 **Blocked** — see below |

## Patcher patches (1 active)

`post_transpile_patch.py` does namespace prefix fixups:
- `using ::std::borrow::Cow/ToOwned;` → commented out (we don't vendor `core::borrow`).
- `using ::string::String;` → `using rusty::String;`.
- `using rusty::Vec;` → `using ::Vec;` (Vec is global after VecLegacy retirement).
- `rusty::Vec<>` type refs → `::Vec<>`.
- `std::ptr::Alignment` → `rusty::ptr::Alignment`.
- `rusty::mem::MaybeUninit` and bare `mem::MaybeUninit` → `rusty::MaybeUninit`.

## Remaining Phase B blockers

1. **No `import vec_port.vec;` in rc_port** — the patcher now rewrites `rusty::Vec` to `::Vec` but `::Vec` isn't visible without the module import. Either:
   - Patcher should also inject `import vec_port.vec;` into the module preamble, OR
   - Re-transpile with a Cargo.toml that surfaces vec_port as a dependency so the transpiler emits the import.

2. **`Rc<T, A>` template arg count mismatch** — hand-written `rusty::Rc<T>` takes one template arg, but transpiled `Rc<T, Global>` passes two. Same shape for `Weak<T, A>`. Either:
   - Extend the hand-written `rusty::Rc<T>` to `rusty::Rc<T, A = Global>` (mirroring vec_port), OR
   - Hand-port the rc_port to its own `rc_port::Rc<T, A>` and stop aliasing to `rusty::Rc`.

3. **`.cast<>()` method not found on `NonNull<T>` etc.** — the transpiled code calls `.cast<U>()` to type-convert NonNull pointers. Our `rusty::ptr::NonNull<T>` doesn't surface a `cast<>()` method. Either:
   - Add `NonNull::cast<U>()` to `rusty/ptr.hpp` (simple), OR
   - Hand-rewrite the cast sites.

4. **Cluster A regression** — `'auto' not allowed in template argument` errors at line 3948+ indicate the transpiler's __TemplateArgs heuristic isn't applied to rc_port's absorbed methods. This is the same shape as the BTreeMap Cluster A bug that was fixed for btree_port. Needs the fix extended to cover rc.rs's helper-call sites.

5. **Cross-port dependencies** — rc_port references `rusty::Cell`, `rusty::UnsafeCell`, `rusty::Box`, all of which we have but with slightly different signatures than the rustc original expects. Each will surface as instantiation errors during full Phase B build.

## Predicted Phase B effort

The patcher fixes here are good for ~70% of the namespace issues. The
remaining work is substantive: **5-8 days** to extend `rusty::Rc<T, A>`,
add `NonNull::cast<>()`, wire the vec_port import, and chase the
Cluster A residue. By contrast cell_port reached Phase B/C in one
session because cell.rs is simpler (no allocator-generic + no
multi-arg smart-pointer template).

## Reproducing

```bash
# from .claude/worktrees/rusty-lib/
RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloc/src/ | head -1)
mkdir -p /tmp/rc_port/rc_crate/src
cp $RUSTSRC/rc.rs /tmp/rc_port/rc_crate/src/lib.rs
cp docs/rc_port/Cargo.toml.template /tmp/rc_port/rc_crate/Cargo.toml
bash docs/rc_port/prep.sh /tmp/rc_port/rc_crate/src/lib.rs
./target/release/rusty-cpp-transpiler --crate /tmp/rc_port/rc_crate/Cargo.toml \
    --output-dir /tmp/rc_port/cpp_out --auto-namespace
cp /tmp/rc_port/cpp_out/*.cppm transpiled/rc_port/
python3 docs/rc_port/post_transpile_patch.py transpiled/rc_port/
```
