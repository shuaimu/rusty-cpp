# HashMap port — Phase A1 only (probe)

This directory holds the scaffolding for the rustc
`std::collections::HashMap` port. Like
`docs/string_port/STATUS.md`, only Phase A1 (parse) succeeded;
deeper phases need foundational work first.

## Reproducing the parse attempt

```bash
mkdir -p /tmp/hashmap_port/hashmap_crate/src
cp ~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/collections/hash/map.rs \
   /tmp/hashmap_port/hashmap_crate/src/lib.rs
cp docs/hashmap_port/Cargo.toml.template /tmp/hashmap_port/hashmap_crate/Cargo.toml

./target/release/rusty-cpp-transpiler \
    --crate /tmp/hashmap_port/hashmap_crate/Cargo.toml \
    --output-dir /tmp/hashmap_port/cpp_out
```

Output: 3034-line `map.rs` → 22 build errors at first cmake.

## What blocks Phase A2

**Surface error cluster** — 22 errors of `using ::<module>::<Type>;`
where the modules don't exist in the rusty namespace (`borrow`,
`error`, `collections`, `fmt::Debug`, `hash::BuildHasher`,
`hash::Hash`, `iter::*`, etc.). These are mechanically patchable.

**Deeper blocker** — `std::collections::HashMap` is just a *thin
wrapper* around `hashbrown::HashMap`. The actual SwissTable
implementation lives in the `hashbrown` crate (not in `library/`).
Doing this port end-to-end requires:

1. **Vendor `hashbrown` separately.** It's its own crate with
   SIMD probing logic, control-byte arrays, group operations. The
   internal `RawTable<K, V>` is the real workhorse.
2. **Port `BuildHasher` + `Hash` trait infrastructure.** Rust's
   hasher API is a trait family with associated types; the C++
   port needs either CRTP equivalents or virtual-call wrappers.
3. **Then* port `std::HashMap` as a thin wrapper around the
   vendored hashbrown.

This is essentially the same structure as the BTreeMap port (a
data-structure crate with its own SwissTable analogue of the
node-based tree), but the predicted complexity is at least as
high — see book §3.8 #3 for the ~1-week estimate, still accurate.

## Phase plan when work resumes

- [x] A1 — parse std/collections/hash/map.rs (0 errors)
- [ ] **Prerequisite**: port `hashbrown` crate (its own
      A→E pipeline; estimated 5–7 days)
- [ ] A2 — compile: namespace patches + hashbrown link
- [ ] B/C/E — same shape as BTreeMap

## Why pause here

HashMap legitimately needs more groundwork than the original
§3.8 estimate captured — the "~1 week" assumed hashbrown was
already ported. With hashbrown as a sibling project, total is
closer to 2 weeks. Cleaner to defer than to start hashbrown as a
side-trip from the Vec/String/HashMap queue.

The right next move (per playbook §2.7 — "when to stop"): close
the Vec port retrospective, write the call to action for
hashbrown to land before HashMap can complete, then exit.
