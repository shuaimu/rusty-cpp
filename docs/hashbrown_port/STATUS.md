# hashbrown port — Status

Following the BTreeMap / Vec port playbook (book §2.3): vendor the
rustc-stdlib source, prep it for the transpiler, transpile, then
peel patches until it builds + smoke-tests.

**Source**: `hashbrown-0.17.0` crate (~21K LOC, 17 .rs files
excluding `external_trait_impls/{serde,rayon}` and tests).
**Target**: transpiled C++20 module `hashbrown_port` exposing
`hashbrown_port::HashMap<K, V>`, `HashSet<T>`, `HashTable<T>`.

## Phase plan

### Phase A — types compile

- [x] **A1** First transpile pass. Vendor + prep.sh + transpiler.
      **17 / 17 files transpile cleanly, 0 parser errors.**
      4 / 17 modules fail compile at cmake (catalogued below).
- [~] **A2** Per-module compile fixes (in progress).
      **16 / 17 modules compile**, only `raw.cppm` remains.
      Total patches: ~11 in `post_transpile_patch.py`.
      Cluster fixes landed:
      - `control.tag`: const-qualify member methods; stub Tag::fmt
      - `hasher`: replace body with FNV-1a stub (drops foldhash dep)
      - `alloc`: inner::Global → std::malloc/free; AllocatorAdapter
        maps `rusty::alloc::AllocError` → `std::tuple<>{}`
      - `control.group.generic`: hand-rolled (~150 LOC) replacement
        with `group_internal::{Tag,BitMask}` to avoid cross-module
        redefinition conflicts
      - `control.group`: drop `generic::` qualifier
      - `control.bitmask`: move misplaced import; strip `group::`
        qualifier; Rust integer-trait methods → __builtin_ctzll etc.;
        neutralize ARM cfg!() dead branch
      - `control` (parent): strip `bitmask::`/`group::`/`tag::`
        qualifiers; drop unexported `TagSliceExt` re-export
      - `raw`: partial — bare `TryReserveError` → `rusty::collections::
        TryReserveError`. Substantial work remaining:
        - `::TryReserveError` (leading-`::`) needs the same rewrite
        - Rust enum variant constructors (`TryReserveError_CapacityOverflow{}`,
          `TryReserveError_AllocError{...}`) — semantic gap, need
          either matching tagged-struct emits in rusty or a stub layer
        - Misplaced `import` lines
        - `std::AllocError`/`std::Allocator` → `rusty::alloc::*`
        - `handle_alloc_error` returns void but signature wants Result
- [ ] **A3** Once raw compiles, cascading errors in map/set/table/
      raw_entry/rustc_entry will surface. (Most likely the same
      classes of issues; the patcher framework is now in place.)

### Phase B — hand-port unknowns

- [ ] **B1** Methods the transpiler can't emit cleanly (record in
      `post_transpile_patch.py`).

### Phase C — link + smoke test

- [ ] **C1** Smoke test: insert N (k, v) pairs, read back, drop.

### Phase E — completeness + bench

- [ ] **E1** Full API parity tests (4-way bench shape).
- [ ] **E2** vs `std::unordered_map` + native Rust `HashMap`.

## A1 outcome (this session)

```bash
bash docs/hashbrown_port/prep.sh /tmp/hashbrown_port/hashbrown_crate/src
cargo run --release -p rusty-cpp-transpiler -- \
  --crate /tmp/hashbrown_port/hashbrown_crate/Cargo.toml \
  --output-dir /tmp/hashbrown_port/cpp_out
# → Done: 17 files transpiled, 0 errors
```

**prep.sh** handles three things the transpiler can't:
1. **cfg_if! collapse** — forced the generic (no-SIMD) group impl by
   replacing `control/group/mod.rs` with a 4-line non-cfg_if version.
   sse2/neon/lsx files deleted.
2. **Nightly-gated items stripped** — any `#[cfg(feature = "nightly")]`-
   prefixed item (impl block, fn, use) is removed entirely via a
   Python brace-matching pass. Handles `#[may_dangle]` Drop impls,
   `TrivialClone` specializations, etc. Also strips `default_fn! { ... }`
   macro wrapping by unwrapping the contents.
3. **`Equivalent` trait inlining** — `pub use equivalent::Equivalent;`
   is rewritten to a hand-rolled definition in the crate so we don't
   need a cross-crate lookup.

## A1 compile-error catalogue (Phase A2 work)

4 / 17 modules currently fail compile:

| Module | First-wave error pattern |
|---|---|
| `hashbrown_port.control.group.generic` | `BitMaskWord`/`Tag`/`BitMask` not resolved (cross-module import failure); `u64` undeclared (transpiler emit shape) |
| `hashbrown_port.control.tag` | (cascading on group.generic) |
| `hashbrown_port.alloc` | `allocator_api2` external crate refs; `rusty::ptr::slice_from_raw_parts_mut` missing; `AllocError` vs `std::tuple<>` mismatch |
| `hashbrown_port.hasher` | (cascading; specific errors not yet inspected) |

The downstream modules (raw, table, map, set, raw_entry, rustc_entry,
util, scopeguard, etc.) presumably depend on these four landing
first.

## Reproducing the pipeline

```bash
# 1. Set up vendor crate (Cargo.toml + src/ from hashbrown-0.17.0):
HB=~/.cargo/registry/src/index.crates.io-*/hashbrown-0.17.0
TGT=/tmp/hashbrown_port
rm -rf $TGT
mkdir -p $TGT/hashbrown_crate/src
cp -r $HB/src/* $TGT/hashbrown_crate/src/
rm -rf $TGT/hashbrown_crate/src/external_trait_impls
find $TGT/hashbrown_crate/src -name "tests*" -type d -exec rm -rf {} +
find $TGT/hashbrown_crate/src -name "tests.rs" -delete
cat > $TGT/hashbrown_crate/Cargo.toml <<EOF
[package]
name = "hashbrown_port"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF

# 2. Apply prep.sh:
bash docs/hashbrown_port/prep.sh $TGT/hashbrown_crate/src

# 3. Transpile:
cargo run --release -p rusty-cpp-transpiler -- \
  --crate $TGT/hashbrown_crate/Cargo.toml \
  --output-dir $TGT/cpp_out

# 4. Build (will surface A2 errors):
cd $TGT/cpp_out
CXXFLAGS="-I<rusty-lib>/include" cmake -B build -S . -G Ninja \
  -DCMAKE_CXX_COMPILER=clang++-19 -DCMAKE_CXX_STANDARD=23
cmake --build build
```

## Estimated remaining budget

Per book §3.8 and the earlier scoping doc: ~5-10 days of patcher
iteration, similar shape to BTreeMap (which took ~5 days). The SIMD
group impls (sse2/neon/lsx) are deferred for now — generic path
only — which trades ~2x perf for staying within scope.
