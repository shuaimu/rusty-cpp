#!/usr/bin/env python3
"""post-transpile patcher for the STANDALONE mono-module hashbrown.
Applies the hashbrown-specific text fixes that the transpiler can't
do generically (mirrors docs/hashbrown_port/post_transpile_patch.py,
collapsed for the single combined module)."""
import sys, pathlib
p = pathlib.Path(sys.argv[1])
text = p.read_text()
total = 0

def repl(old, new):
    global text, total
    c = text.count(old)
    if c:
        text = text.replace(old, new)
        total += c
        print(f"  +{c}  {old[:60]!r}")

# --- ScopeGuard dropfn: Rust FnMut(&mut T) -> void(T&); pass value, not &value ---
repl("(this->dropfn)(&this->value);", "(this->dropfn)(this->value);")

# --- crate-local `mod alloc` vs the std `alloc` crate ---------------------------
# The crate has `extern crate alloc as stdalloc;` AND its own `mod alloc`, so
# `use crate::alloc::{Global, do_alloc}` must stay LOCAL. The transpiler maps a
# root-level `alloc` segment to `std` (it cannot model the extern-crate rename),
# emitting `using std::Global;` / `using std::do_alloc;` which fail name lookup.
# The crate's own `namespace alloc` re-exports both (export using
# alloc::inner::Global; / ::do_alloc;), so retarget the `using`s at it.
repl("using std::Global;", "using ::alloc::Global;")
repl("using std::do_alloc;", "using ::alloc::do_alloc;")

# --- UFCS free-fn `equivalent` over-qualified with the `map::` module ----------
# `equivalent` is a trait method lowered to a free function reachable unqualified
# at the call site (the compiler itself suggests "did you mean simply
# 'equivalent'"); the emitter qualified it with the enclosing `map` module where
# no such member exists. Drop the qualifier.
repl("map::equivalent(", "equivalent(")

p.write_text(text)
print(f"patched {total} sites in {p.name}")
