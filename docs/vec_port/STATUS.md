# rustc-stdlib Vec Port — Status

First port after BTreeMap. Following the §2.3 / Chapter 2 phase
template (A → C → E, skipping D parity testing for now).

**Source**: `library/alloc/src/vec/` (16 files, 6,711 LOC) +
`library/alloc/src/raw_vec/` (1 file, 904 LOC).

**Target**: transpiled C++20 module `vec_port.vec` exposing
`vec_port::Vec<T, A>` (and supporting types: `IntoIter`, `Drain`,
`Splice`, `ExtractIf`).

**Comparison baseline**: the existing hand-written
`include/rusty/vec.hpp` (extensive; ~860 LOC of mostly hand-written
implementation). Both ship together — the hand-written version
provides the API surface annotation target, the transpiled version
provides exact rustc semantic parity.

## Source dependency graph

`Vec<T, A>` depends on:

| Path | Resolution in port | Status |
|---|---|---|
| `crate::raw_vec::RawVec` | Sibling module — vendored as `raw_vec/` | Need to port |
| `crate::alloc::{Allocator, Global, Layout, AllocError, handle_alloc_error}` | Existing `include/rusty/alloc.hpp` | Exists |
| `crate::boxed::Box` | Existing `include/rusty/box.hpp` | Exists |
| `crate::collections::TryReserveError` | Small struct, needs minimal port | Net-new |
| `crate::collections::VecDeque` | Treated as external (deferred until VecDeque port) | Deferred |
| `crate::fmt` | Existing `include/rusty/fmt.hpp` | Exists |
| `crate::borrow::{Cow, ToOwned}` | Treated as external for now | Deferred |
| `core::iter::*` | Existing C++ std iterators + `slice.hpp` adapters | Mostly exists |
| `core::slice` | Existing `include/rusty/slice.hpp` | Exists |
| `core::ptr::NonNull` | Existing `include/rusty/ptr.hpp` | Exists |
| `core::mem::ManuallyDrop` | Existing `include/rusty/mem.hpp` | Exists |

## Phase plan

Following the BTreeMap retrospective in §1.8 / playbook in §2.3.

### Phase A — Types compile

- [ ] **A1**: First transpile attempt. Catalogue all errors.
- [ ] **A2**: Per-cluster fixes. Apply Cluster A–E knowledge from
      BTreeMap port (likely template-arg recovery for `Vec`, `RawVec`,
      `IntoIter`).
- [ ] **A3**: Get `raw_vec.cppm` to compile.
- [ ] **A4**: Get `vec.cppm` to compile.
- [ ] **A5**: Get `into_iter.cppm`, `drain.cppm` compiling (or fold
      into vec.cppm if cyclic).

### Phase B — Hand-port unknowns

- [ ] **B1**: Stub-or-hand-port any methods the transpiler can't
      emit. Record in `post_transpile_patch.py`.

### Phase C — Link + smoke test

- [ ] **C1**: Write a smoke test that constructs a Vec, pushes a
      few elements, reads them back, drops. Link against transpiled
      module + rusty headers.

### Phase E — Completeness + bench

- [ ] **E1**: Cover the full public API of `vec.hpp` with
      side-by-side tests calling both the hand-written and
      transpiled versions on the same inputs.
- [ ] **E2**: Bench: insert N elements / iterate N elements /
      random access — versus `std::vector<T>` and native Rust
      `Vec<T>`.
- [ ] **E3**: Callgrind component comparison vs Rust BTreeMap
      style (§1.6).
- [ ] **E4**: Document in `rusty-std-book.md` Chapter 4.

## How to reproduce

```bash
# 1. Copy the stdlib vec + raw_vec subtrees.
mkdir -p /tmp/vec_port/vec_crate/src
cp -r ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/vec \
   /tmp/vec_port/vec_crate/src/vec
cp -r ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/alloc/src/raw_vec \
   /tmp/vec_port/vec_crate/src/raw_vec

# 2. Apply prep.sh hand-patches.
bash docs/vec_port/prep.sh \
  /tmp/vec_port/vec_crate/src/vec \
  /tmp/vec_port/vec_crate/src/raw_vec

# 3. Minimal Cargo crate skeleton.
cat > /tmp/vec_port/vec_crate/Cargo.toml <<EOF
[package]
name = "vec_port"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF
cat > /tmp/vec_port/vec_crate/src/lib.rs <<EOF
#![allow(unused)]
pub mod vec;
pub mod raw_vec;
EOF

# 4. Transpile.
cargo run --release -p rusty-cpp-transpiler -- \
  --crate /tmp/vec_port/vec_crate/Cargo.toml \
  --output-dir /tmp/vec_port/cpp_out

# 5. Apply post-transpile patches (once we have them).
# python3 docs/vec_port/post_transpile_patch.py /tmp/vec_port/cpp_out

# 6. Build.
# cd /tmp/vec_port/cpp_out && cmake -B build -S . -G Ninja \
#   -DCMAKE_CXX_FLAGS="-I<rusty-lib>/include -std=c++23" \
#   -DCMAKE_CXX_STANDARD=23 && cmake --build build
```

## Current state

**Active phase**: A1 — initial transpile attempt.

See `docs/rusty-std-book.md` Chapter 4 (forthcoming) for the running
retrospective.
