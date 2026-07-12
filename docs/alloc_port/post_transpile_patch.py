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


def _disambiguate_hoisted_helpers(t: str) -> str:
    """Rename duplicate `struct Guard`/`struct Dropper` that the transpiler
    hoists to class scope (Rust method-local structs with a shared name →
    C++ member-type redefinition). First occurrence keeps its name; each
    later one is suffixed `_2`, `_3`, … and its uses within the following
    method body are renamed to match. Indent-agnostic port of the
    vec_deque_port helper (single module uses deeper indentation)."""
    lines = t.splitlines(keepends=True)
    counts: dict = {}
    i = 0
    while i < len(lines):
        m = re.match(r"^(\s+)struct (Guard|Dropper) \{\s*$", lines[i])
        if not m:
            i += 1
            continue
        indent, name = m.group(1), m.group(2)
        counts[name] = counts.get(name, 0) + 1
        if counts[name] == 1:
            i += 1
            continue
        new_name = f"{name}_{counts[name]}"
        # struct end: matching `};` at same brace depth.
        depth = 0
        seen = False
        j = i
        while j < len(lines):
            for ch in lines[j]:
                if ch == "{":
                    depth += 1; seen = True
                elif ch == "}":
                    depth -= 1
            j += 1
            if seen and depth == 0:
                break
        struct_end = j
        # method body end: depth back to 0, or next hoisted struct.
        depth = 0
        seen = False
        method_end = struct_end
        for k in range(struct_end, len(lines)):
            if re.match(r"^\s+struct \w+ \{\s*$", lines[k]):
                method_end = k
                break
            for ch in lines[k]:
                if ch == "{":
                    depth += 1; seen = True
                elif ch == "}":
                    depth -= 1
            if seen and depth == 0:
                method_end = k + 1
                break
        for r in range(i, method_end):
            lines[r] = re.sub(rf"(?<![:\w]){name}\b", new_name, lines[r])
        i = method_end
    return "".join(lines)


def _stub_next_chunk(t: str) -> str:
    """`Iterator::next_chunk` (nightly) returns `Result<[T;N], array::IntoIter>`
    but there is no `rusty::array::IntoIter`, so the return type is ill-formed.
    An ill-formed member declaration poisons the whole class — clang then
    reports later members (clone/next_back) as "not a member" and the
    clone-delegating copy-ctor fails. next_chunk is unexercised; stub it to a
    valid signature (matches vendored vec_port)."""
    lines = t.splitlines(keepends=True)
    out = []
    i = 0
    while i < len(lines):
        if re.search(r"\bnext_chunk\(\)\s*\{", lines[i]) and "array::IntoIter" in lines[i]:
            indent = re.match(r"\s*", lines[i]).group(0)
            out.append(f"{indent}auto next_chunk() {{ std::abort(); }}\n")
            depth = 0
            seen = False
            j = i
            while j < len(lines):
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1; seen = True
                    elif ch == "}":
                        depth -= 1
                j += 1
                if seen and depth == 0:
                    break
            i = j
            continue
        out.append(lines[i])
        i += 1
    return "".join(out)


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
        # `using std::ub_checks;` (the module import survived even though the
        # only member use was already rewritten) — nothing to import, strip it.
        t = re.sub(r"\n\s*using std::ub_checks;", "", t)
        # Stray cross-crate ref to the OLD per-port module surface; in the
        # single crate it's the sibling submodule.
        t = t.replace("rusty::port::vec::IntoIter", "vec::into_iter::IntoIter")
        # `let (src, dst, len) = if … else …;` tuple-destructure: the transpiler
        # emits the branch assignments but drops the hoisted binding decl.
        # join_head_and_tail_wrapping is the one site; inject the decl (all
        # three are physical indices → size_t, matching the vendored port).
        t = t.replace(
            "const auto join_head_and_tail_wrapping = [](auto& source_deque, "
            "size_t drain_len, size_t head_len, size_t tail_len) {\n",
            "const auto join_head_and_tail_wrapping = [](auto& source_deque, "
            "size_t drain_len, size_t head_len, size_t tail_len) {\n"
            "                                size_t src, dst, len;\n",
        )
        # Late-init `let ret; unsafe { ret = ptr::read(..); .. }` — the transpiler
        # scopes `ret` inside the inner unsafe block, which closes before
        # `return Some(ret)`. Dissolve the inner block so ret survives (it's
        # move-initialized → can't be default-declared and hoisted).
        t = re.sub(
            r"\n( +)\{\n"
            r"( +auto ptr_shadow1 = rusty::ptr::add\(.*?\n"
            r" +auto ret = rusty::ptr::read\(ptr_shadow1\);\n"
            r" +rusty::ptr::copy\(.*?\n)"
            r" +\}\n"
            r"( +this->set_len)",
            r"\n\1\2\3",
            t,
            flags=re.DOTALL,
        )
        # `if const { size_of::<SRC>()==0 || … }` compile-time fences were
        # elided to `(void)0`, which isn't bool-convertible. They guard ZST /
        # debug-assert paths the port never exercises — make the guard false so
        # the branch is dead.
        t = t.replace(
            "/* const-block elided (Rust 2024 compile-time fence) */ (void)0",
            "false",
        )
        # `const { … }` value blocks emitted as an argument (const-eval seed)
        # also collapse to `(void)0`; already covered by the replace above.
        # `rusty::alloc::Global` is a (unit) TYPE; using it as a value needs an
        # instance.
        t = t.replace("= rusty::alloc::Global;", "= rusty::alloc::Global{};")
        # `::IS_ZST` lost its `T::` type qualifier (bare-global ZST probe).
        t = t.replace("rusty::detail::rust_not(::IS_ZST)", "rusty::detail::rust_not(T::IS_ZST)")
        # spec_extend_front(Copied<…> | Rev<Copied<…>>) — Rust perf
        # specializations with no core::iter::Copied C++ analog; the generic
        # template-I overload covers all callers. Delete both the UFCS
        # forward-decls (end in `;`) and the class-method defs (have a body).
        # (Vendored vec_deque_port deletes these too.)
        t = re.sub(
            r"\n\s*export template<[^\n]*>\n\s*void spec_extend_front\([^;{]*Copied[^;{]*\);",
            "",
            t,
        )
        _vd = load("docs/vec_deque_port/post_transpile_patch.py")
        t = _vd._stub_copied_spec_extend_front(t)
        # The stubber deletes the decl body but leaves the `export template<…>`
        # head dangling ("expected unqualified-id"). Drop the orphan head.
        t = re.sub(
            r"\n\s*export template<[^\n]*>\n(\s*// patcher: spec_extend_front<Copied)",
            r"\n\1",
            t,
        )
        # The remaining spec_extend_front(Drain<…>) forward-decls use a bare
        # `Drain` (the class-method def already qualifies it). Qualify to the
        # defining submodule.
        t = t.replace(
            "self_, Drain<T, A2>",
            "self_, collections::vec_deque::drain::Drain<T, A2>",
        )
        t = t.replace(
            "std::declval<Drain<T, A2>>",
            "std::declval<collections::vec_deque::drain::Drain<T, A2>>",
        )
        # Rename the hoisted Guard/Dropper collisions.
        t = _disambiguate_hoisted_helpers(t)
        # Stub next_chunk (ill-formed rusty::array::IntoIter return type poisons
        # the enclosing IntoIter class → cascades to clone/next_back "not a
        # member").
        t = _stub_next_chunk(t)
        # `usize::abs_diff` has no rusty analog and wasn't lowered; inline it
        # (both operands are size_t → the std::move is a no-op copy).
        t = re.sub(
            r"(\w+)\.abs_diff\(std::move\((\w+)\)\)",
            r"(std::max<size_t>(\1, \2) - std::min<size_t>(\1, \2))",
            t,
        )
        # `is_zero(Option<…bool…>)` uses Rust niche layout: Option<bool> is 1
        # byte there, so it transmutes to u8 and compares. rusty::Option<bool>
        # is 2 bytes → the transmute size-assert fires. is_zero only gates a
        # memset-zero fast path, so returning false (element-wise fill) is
        # always correct. Rewrite just the Option-of-bool bodies (same-size
        # is_zero(u8)/(i8)/… transmutes are fine, left untouched).
        t = re.sub(
            r"(export bool is_zero\(const rusty::Option<[^;{]*?bool[^;{]*?>& self_\) \{\n)"
            r" +using Self[^\n]*\n"
            r" +const uint8_t raw = rusty::mem::transmute[^\n]*\n"
            r" +return rusty::detail::deref_if_pointer_like\(raw\)[^\n]*\n"
            r"( +\})",
            r"\1                return false;\n\2",
            t,
        )
        # `impl PartialEq<Vec/[U]> for Cow<[T]>` — but Cow<[T]> collapsed to the
        # prelude's str/bytes `rusty::Cow` (a std::variant), so the param type is
        # CONCRETE. That makes `slice_full(self_)` a non-dependent expression,
        # instantiated at template-DEFINITION time (two-phase lookup) → array.hpp
        # asserts (no len() for the Cow variant) even though nothing calls it.
        # These slice-Cow comparisons can't be expressed against a str/bytes Cow;
        # abort (loud if the unexercised path is ever hit, per vendored practice).
        t = re.sub(
            r"(bool (?:eq|ne)\(const rusty::Cow& self_,[^{]*\{)\n"
            r"\s*using Self[^\n]*\n"
            r"\s*return [^\n]*slice_full\(self_\)[^\n]*\n"
            r"(\s*\})",
            r"\1 std::abort(); \2",
            t,
        )
        # write_iter_wrapping (uncalled abbreviated-template helper): `size_t::
        # ByRefSized` is a non-dependent name that fails at parse. ByRefSized is
        # just a zero-cost re-borrow; `iter.take(…)` is dependent (unchecked
        # unless instantiated) and the correct intent.
        t = t.replace(
            "size_t::ByRefSized(&iter).take(",
            "iter.take(",
        )
        # VecDeque::resize used core::iter::repeat_n (no rusty::iter analog).
        # Emit a real loop so resize actually works (vendored abort-stubs it).
        t = t.replace(
            "this->extend(rusty::iter::repeat_n(std::move(value), std::move(extra)));",
            "for (size_t _ri = 0; _ri < rusty::detail::deref_if_pointer_like(extra); ++_ri) "
            "{ this->push_back(rusty::clone(value)); }",
        )
        # `super::Vec` (into_iter's Default) — parent module is `vec`.
        t = t.replace(
            "super::Vec<T, rusty::alloc::Global>",
            "vec::Vec<T, rusty::alloc::Global>",
        )
        # VecDeque::from([T;N]): the transpiler used the array-length param N as
        # the type argument (`VecDeque<N>`); it's the deque's own T, A.
        t = t.replace(
            "auto deq = VecDeque<N>::with_capacity(N);",
            "auto deq = VecDeque<T, A>::with_capacity(N);",
        )
        # usize::unchecked_sub not lowered (like abs_diff) — plain subtraction.
        t = t.replace(
            "rusty::field_end(initialized).unchecked_sub(std::move(rusty::field_start(initialized)))",
            "(rusty::field_end(initialized) - rusty::field_start(initialized))",
        )
        # in-place-collect: PhantomData<Src>/RawVec<Src, A> reference type names
        # the transpiler didn't thread through — `Src` is `I::Src`, and the
        # drop's `A` is the (only) Global allocator.
        t = t.replace("rusty::PhantomData<Src>{}", "rusty::PhantomData<typename I::Src>{}")
        t = t.replace(
            "raw_vec::RawVec<Src, A>::from_nonnull_in",
            "raw_vec::RawVec<Src, rusty::alloc::Global>::from_nonnull_in",
        )
        # handle_error: the TryReserveErrorKind match mis-emitted a unit-variant
        # arm as a `const auto& Enum::Variant = _m;` binding. Both arms are
        # infallible-reserve OOM handlers that abort, so collapse to abort.
        t = re.sub(
            r"return \[&\]\(\) -> void \{ auto&& _m = e\.kind\(\);.*?"
            r"rusty::collections::TryReserveErrorKind::CapacityOverflow = _m;.*?"
            r"rusty::intrinsics::unreachable\(\); \}\(\); \}\(\);",
            "(void)e; std::abort();",
            t,
            flags=re.DOTALL,
        )
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
