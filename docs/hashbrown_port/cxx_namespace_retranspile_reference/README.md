# hashbrown_port `--cxx-namespace` retranspile reference

These files are a **reference snapshot** of running the transpiler with
`--cxx-namespace rusty::port::collections::hashbrown` against
hashbrown-0.17.0. They are **not vendored into the build** — the live
vendored cppm files in `transpiled/hashbrown_port/` remain at the
original flat (un-namespaced) emit because the existing
`post_transpile_patch.py` was tuned for that shape, and the
deep-namespaced emit surfaces a fresh long-tail of issues that would
need separate patcher work (~5-10 days, similar to the original port).

## How this snapshot was produced

```bash
HB=~/.cargo/registry/src/index.crates.io-*/hashbrown-0.17.0
TGT=/tmp/hashbrown_port_retranspile
rm -rf $TGT && mkdir -p $TGT/hashbrown_crate/src
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
bash docs/hashbrown_port/prep.sh $TGT/hashbrown_crate/src
./target/release/rusty-cpp-transpiler \
    --crate $TGT/hashbrown_crate/Cargo.toml \
    --output-dir $TGT/cpp_out \
    --cxx-namespace rusty::port::collections::hashbrown
```

Result: **17 files transpiled, 0 parser errors.** All emitted content
post-`export module hashbrown_port.X;` is wrapped in
`namespace rusty::port::collections::hashbrown { ... }`. Confirmed
in this snapshot by `grep ^namespace map.cppm` etc.

## Why this isn't shipped yet

When this snapshot is compiled with the current patcher, the
following issue clusters appear (sample from the first build):

1. **`rusty::Vec<uint8_t>` in serde-de prelude** — `visit_byte_buf`
   uses `rusty::Vec` which isn't visible inside the wrap (the
   `rusty::Vec` alias lives in the `rusty` umbrella module, not in
   the GMF). Stub via the same `patch_visit_byte_buf` shape that
   `docs/binary_heap_port/post_transpile_patch.py` uses.

2. **`::cold_path()` global ref** — the wrap moves `cold_path` from
   global scope into the namespace. Same root cause as the previously
   reverted sed migration. Strip the `::` prefix.

3. **`rusty::ptr::slice_from_raw_parts_mut`** — symbol referenced via
   `rusty::ptr` lookup that may have been renamed or scoped
   differently in newer rusty headers. Compare against current
   `include/rusty/ptr.hpp`.

4. **`Result<X, AllocError>` vs `Result<X, rusty::Unit>` mismatch**
   in `AllocatorAdapter` — emit/header drift.

5. **Ambiguous `clone` call** in control.tag — multiple overloads
   reachable when name lookup widens after the wrap.

6. **Internal `::Group`, `::ScopeGuard`, `::BITMASK_*`, `::h1`,
   `::prev_pow2`, etc.** — same shape as the previously reverted sed
   migration. Strip `::` from the intra-module refs.

## What needs to happen to land this

1. **Update `post_transpile_patch.py`** to handle the new
   namespace-wrapped emit shape:
   - Add `patch_visit_byte_buf` (binary_heap_port shape).
   - Strip `::` prefix on intra-module function/type refs that the
     wrap moved out of global scope (`cold_path`, `Group`,
     `ScopeGuard`, `BITMASK_*`, `BitMaskWord`, `NonZeroBitMaskWord`,
     `h1`, `prev_pow2`, `bucket_mask_to_capacity`,
     `capacity_to_buckets`, `maximum_buckets_in`,
     `ensure_bucket_bytes_at_least_ctrl_align`, `offset_from`).
   - Hoist any `import` statements that ended up inside the namespace
     wrap out to module purview.
   - Rewrite `export using ::Name;` → `export using <wrap>::Name;`
     selectively (don't touch `using ::rusty::*`).

2. **Update consumer code**:
   - `tests/hashbrown_port_map_test.cpp` / `_set_test.cpp` to use
     `::rusty::port::collections::hashbrown::HashMap` / `HashSet`
     directly (or the `rusty::collections::*` alias).
   - `include/rusty/rusty.cppm` already has
     `rusty::collections::{HashMap,HashSet}` aliases — point them to
     the new deep path.

3. **Vendor + verify**:
   - Replace `transpiled/hashbrown_port/*.cppm` with the new shape.
   - Build, iterate on any remaining issues, smoke-test.

## Related

- `docs/binary_heap_port/STATUS.md` — full pipeline that already does
  the `--cxx-namespace` migration cleanly (BinaryHeap is the
  demonstrator).
- `transpiled/btree_port/` — also migrated to
  `rusty::port::collections::btree::*` (post-sed, not via
  `--cxx-namespace` due to btree's pre-existing nested namespace
  structure).
