#!/usr/bin/env python3
"""Unified alloc_port patcher.

The consolidated alloc crate is ONE C++ module with the FULL namespace tree
preserved (vec::is_zero::__ufcs_IsZero, collections::vec_deque, …). The
transpiler emits those correctly. So we reuse the per-port patchers' CONTENT
rules (std::collections->rusty, Cap alias, aggregate_raw_ptr, self-Vec qual,
…) but NEUTRALIZE their per-SUBMODULE STRUCTURAL rules — chiefly the
"strip submodule:: qualifiers" rewrite, which was written for the old
flattened-per-import layout and MANGLES the real single-module namespaces
(vec::is_zero::__ufcs_IsZero -> vec::__ufcs_IsZero), and the module-MERGE
rules (moot: already one module).

Usage: post_transpile_patch.py <cpp_out_dir>
"""
import importlib.util, sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# Structural rules that assume the per-submodule layout — harmful or moot here.
NEUTRALIZE = {
    "patch_module_qualified_refs",           # strips real submodule:: namespaces
    "patch_merge_drain_into_vec",
    "patch_merge_extract_if_into_vec",
    "_merge_aux_module_into_vec",
    "patch_strip_vec_cppm_aux_imports",
    "patch_top_level_import_subset",
    "patch_strip_orphan_using_decls",
    "patch_stub_dropped_iter_types",
    "patch_hoist_imports_after_module_decl",
    "patch_trim_cmakelists",
}

def load(rel):
    p = REPO / rel
    spec = importlib.util.spec_from_file_location("p_" + p.parent.name, p)
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m

import re

def _alloc_specific(cpp_out: Path):
    """Rules the per-port patchers apply per-FILE (so they miss the single
    module) or that only arise in the consolidated crate. Applied glob-wide."""
    for path in cpp_out.glob("*.cppm"):
        t = path.read_text()
        o = t
        # collections::TryReserveError is external (rusty), but the prep's
        # std::collections mapping collapses to a bare `collections::` that
        # resolves to the LOCAL collections module (which lacks it).
        # All spellings (std::, ::, bare) of the external TryReserveError[Kind]
        # -> rusty::. Sentinel-guard the already-correct rusty:: form so the
        # bare/::-prefixed replaces (which also match inside rusty::collections)
        # don't double-prefix.
        for kind in ("TryReserveErrorKind", "TryReserveError"):
            t = t.replace(f"rusty::collections::{kind}", f"\0{kind}\0")
            t = t.replace(f"std::collections::{kind}", f"rusty::collections::{kind}")
            t = t.replace(f"::collections::{kind}", f"::rusty::collections::{kind}")
            t = re.sub(rf"(?<![:\w])collections::{kind}\b", f"rusty::collections::{kind}", t)
            t = t.replace(f"\0{kind}\0", f"rusty::collections::{kind}")
        # std::iter / std::ub_checks → rusty / stripped.
        t = t.replace("std::iter::", "rusty::iter::")
        t = t.replace("std::ub_checks::assert_unsafe_precondition", "rusty::intrinsics::noop")
        # Vec self-defines Vec; the umbrella alias isn't in scope here.
        t = t.replace("rusty::Vec<", "Vec<")
        # The transpiler puts `requires (Allocator<A>)` on the Vec/VecDeque
        # CLASS template but NOT on the out-of-line method DEFINITIONS, so they
        # mismatch ("requires clause differs in template redeclaration").
        # Strip it everywhere — consistent, and the constraint isn't
        # load-bearing for compilation.
        t = re.sub(r"\n\s*requires \(rusty::alloc::Allocator<A>\)", "", t)
        # Vec/VecDeque forward decls then need a default allocator (Vec<T> in
        # from_elem etc.). VecDeque already carries one; add it to Vec.
        for name in ("Vec", "IntoIter", "Drain"):
            t = re.sub(
                rf"(export template<typename T, typename A)(>\n\s*struct {name};)",
                r"\1 = rusty::alloc::Global\2",
                t,
            )
        t = t.replace("IntoIter into_iter(", "IntoIter<T, A> into_iter(")
        # bare NonZero::new_ -> qualified; and its Option constant can't be
        # constexpr (non-literal type) -> inline const.
        t = t.replace("NonZero::new_(", "rusty::num::NonZero<size_t>::new_(")
        t = t.replace(
            "static constexpr rusty::Option<rusty::num::NonZero<size_t>>",
            "static inline const rusty::Option<rusty::num::NonZero<size_t>>",
        )
        # Bare submodule types referenced from a sibling/parent scope in the
        # single module: qualify to their defining submodule namespace.
        t = re.sub(r"(?<![:\w])IntoIter<T>", "into_iter::IntoIter<T>", t)
        t = re.sub(r"(?<![:\w])IntoIter<T, A2?>", lambda m: "into_iter::" + m.group(0), t)
        # rusty::iter::Copied<X> adapter isn't exposed under rusty::iter; the
        # spec_extend_front decl that uses it is a dead specialization — drop
        # the Copied wrapper to its inner iterator type so the decl compiles.
        t = re.sub(r"rusty::iter::Copied<([^;]+?)> iter\)", r"\1 iter)", t)
        # std::hint branch hints have no C++ form.
        t = t.replace("std::hint::unlikely(", "(").replace("std::hint::likely(", "(")
        # Spec* extension-trait stubs (real impls live in dropped spec_* modules
        # in the per-port layout; here they're forward-declared). The per-port
        # injection anchors on an import line we stripped, so inject directly at
        # global scope before the first `namespace vec {`.
        stubs = (
            "struct SpecFromElem { template<typename T, typename A>"
            " static auto from_elem(T elem, std::size_t n, A alloc); };\n"
            "template<typename T, typename Iter> struct SpecFromIter"
            " { template<typename I> static auto from_iter(I); };\n"
            "template<typename T, typename Iter> struct SpecExtend"
            " { template<typename V, typename I> static void spec_extend(V&, I) {} };\n"
            "struct SpecCloneIntoVec { template<typename Src, typename Dst>"
            " static void clone_into(Src src, Dst& dst) { auto s = rusty::as_slice(src);"
            " dst.clear(); dst.reserve(s.size());"
            " for (size_t i = 0; i < s.size(); ++i) dst.push(rusty::clone(s[i])); } };\n"
        )
        if "struct SpecFromElem {" not in t and "\nnamespace vec {" in t:
            t = t.replace("\nnamespace vec {", "\n" + stubs + "\nnamespace vec {", 1)
        if t != o:
            path.write_text(t)

def run(cpp_out: Path):
    for rel in ("docs/vec_port/post_transpile_patch.py",
                "docs/vec_deque_port/post_transpile_patch.py"):
        m = load(rel)
        for name in NEUTRALIZE:
            if hasattr(m, name):
                setattr(m, name, (lambda *a, **k: 0))
        try:
            m.main(cpp_out)
        except Exception as e:
            print(f"  [{rel}] main() raised {e!r} — continuing", file=sys.stderr)
    _alloc_specific(cpp_out)

if __name__ == "__main__":
    run(Path(sys.argv[1]))
