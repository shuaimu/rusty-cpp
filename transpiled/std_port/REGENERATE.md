# Regenerating `std_port` (+ its `hashbrown` dep)

These two `.cppm` are vendored so the normal build never needs a rustc source
tree or `cargo-expand`. They are the transpiled Rust **std** `collections::hash`
slice — `HashMap`/`HashSet` over std's `RandomState` — plus the recursively
transpiled hashbrown 0.16.1 they sit on. This is the layering the 2026-07-27
directive asks for: translate from std, reach hashbrown only as std's dependency
(see the top of `docs/port_regen/STATUS.md`).

## THE SNAPSHOT IS OLDER THAN THE TRANSPILER

These two files are NOT reproducible from today's `docs/rusty/build.sh`: they
were taken with an earlier transpiler, and a fresh run emits ~170 extra prologue
lines (the `rusty::` runtime-helper preamble has grown since) that do not
currently compile — `hashbrown compile: 271 errors` versus zero for what is
vendored here. Re-vendoring is therefore its own piece of work, not a step you
take casually while fixing something else.

Consequence: a targeted fix in the transpiler's LOWERING may have to be applied
to these snapshots by hand. That is only legitimate when the generator really
does produce the same shape, so PROVE it the way the self-referential-const fix
did (2026-08-18): regenerate ONCE with the old transpiler and ONCE with the new
one into two work dirs, and check that transforming the old output reproduces
the new output byte-for-byte. Anything else is a hand-patch that the next
regeneration silently reverts.

Currently hand-applied on top of the snapshot:

  * **Self-referential associated consts are function-local-static accessors.**
    `Tag::EMPTY` / `Tag::DELETED` / `RawTableInner::NEW` (hashbrown) and
    `Error::{READ_EXACT_EOF,WRITE_ALL_EOF,INVALID_UTF8}` (std_port) used to be
    namespace-scope `inline const T Owner::NAME = expr;` definitions. Those are
    DYNAMICALLY initialized, and in a module purview a non-module TU that
    reaches them from its own static initializer runs first and reads a zeroed
    object — mako segfaulted in `RawTableInner::full_buckets_indices` off a null
    `ctrl`. They are now `static const T& NAME() { static const T … }`, with
    every `Owner::NAME` use site spelled `Owner::NAME()`. The transpiler emits
    exactly this shape; `tests/std_port_static_init_order_test.cpp` is the
    runtime guard.

## Regenerate

    bash docs/rusty/build.sh <work_dir>
    cp <work_dir>/out/std_port.cppm            transpiled/std_port/
    cp <work_dir>/out/hashbrown/hashbrown.cppm transpiled/std_port/hashbrown/

Then verify BOTH, because a clean `--precompile` proves very little here:

    bash docs/rusty/runtest.sh   <work_dir>   # must print RUNTIME PASS
    REUSE=1 bash docs/rusty/api_coverage.sh <work_dir>

`--precompile` skips template bodies, so a module that reports 0 errors can
still fail at instantiation — that exact trap took API coverage from 36/54 to
3/54 while the module itself compiled clean. `runtest.sh` instantiates with
concrete types; `api_coverage.sh` probes each member in its own TU.

## Two things that will bite a regeneration

**The wrap.** `docs/rusty/build.sh` passes `--crate-namespace-wrap`, which routes
the emission through `wrap_module_purview_in_crate_namespace` — the wrap that
also REQUALIFIES the crate's own qualified self-references. Without it the port
lands in the global namespace (`collections::hash::map::HashMap`, and the dep on
`::map`/`::set`/`::rustc_entry`), which cannot be re-exported from the umbrella.
Do not substitute `--cxx-namespace`: it is a blunt textual wrap with no
requalification and yields 272 errors in the dep alone.

**The patcher is wrap-sensitive.** The transpiler requalifies what IT emits and
never sees text `docs/rusty/post_transpile_patch.py` inserts or matches. Because
that patcher is plain string replacement, a stale anchor fails *silently* — it
simply does nothing, and the failure surfaces much later as an unrelated-looking
compile or instantiation error. Both patch functions therefore derive the prefix
once (`ns = "::std_port" if …`) and every crate-qualified rule, **anchors
included**, is an f-string on it; the hashbrown bridge additionally branches on
shape, since wrapped the dep already supplies `hash_map` and `hash_set` (the
latter as a namespace ALIAS, so re-declaring it is a hard error).

## Provenance

Generated from the rustc sysroot's `std/src/collections/hash/{map,set}.rs` plus
`std/src/hash/random.rs`, with hashbrown 0.16.1 vendored as a local path dep so
the transpiler descends into it. Pinned toolchain expectations and the prep
rewrites live in `docs/rusty/{prep.sh,build.sh}`.
