#!/usr/bin/env python3
"""Unified alloc patcher.

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


def _scope_binary_heap(t: str) -> str:
    """The vec-scoped global rules (`IntoIter<T[,A]>` -> into_iter::IntoIter,
    `IntoIter into_iter(` -> `IntoIter<T, A> into_iter(`) are WRONG inside
    `namespace binary_heap`: its IntoIter is its own direct member (no
    into_iter child namespace) and BinaryHeap carries a `using IntoIter = …;`
    alias that shadows the template name. Revert both inside binary_heap
    regions only (brace-depth tracked)."""
    lines = t.split("\n")
    depth = 0
    bh_depth = None
    for i, line in enumerate(lines):
        if bh_depth is not None and depth >= bh_depth:
            line = re.sub(r"(?<![:\w])into_iter::IntoIter<", "IntoIter<", line)
            line = line.replace("IntoIter<T, A> into_iter(", "IntoIter into_iter(")
            lines[i] = line
        idx = line.find("namespace binary_heap")
        if idx != -1 and bh_depth is None:
            bh_depth = depth + line[:idx].count("{") + 1
        depth += line.count("{") - line.count("}")
        if bh_depth is not None and depth < bh_depth:
            bh_depth = None
    return "\n".join(lines)


def _patch_linked_list(t: str) -> str:
    """linked_list rules: (R1) the UFCS SpecExtend fwd-decl elides Rust's
    default allocator param — spell it; (R2) the emitter qualifies iterator
    types `into_iter::IntoIter` by the vec/vec_deque submodule convention, but
    linked_list is a single-FILE module (IntoIter is a direct member) — inside
    linked_list regions qualify absolutely instead (a bare `IntoIter<` would
    hit the member alias, a non-template name); (R3) stub assert_covariance
    (Rust dead code that exists only for rustc's lifetime-variance check; its
    emission force-instantiates LinkedList<string_view> through Box APIs
    nothing else needs)."""
    t = t.replace(
        "::collections::linked_list::LinkedList<T>",
        "::collections::linked_list::LinkedList<T, rusty::alloc::Global>",
    )
    # Node::into_element (Rust `fn into_element(self: Box<Self, A>) -> T`) was
    # emitted as a member TEMPLATE over the Box's allocator A — undeducible at
    # the `(*box).into_element()` call and unused in the body. De-template it.
    t = t.replace(
        "template<typename A>\n            T into_element() {",
        "T into_element() {",
    )
    out = []
    depth = 0
    ll_stack = []
    for line in t.splitlines(keepends=True):
        m = re.match(r"^\s*namespace (\w+) \{\s*$", line)
        if m and m.group(1) == "linked_list":
            ll_stack.append(depth)
        if ll_stack and not (m and m.group(1) == "linked_list"):
            line = re.sub(
                r"(?<![\w:])into_iter::IntoIter<",
                "::collections::linked_list::IntoIter<",
                line,
            )
            # Rust `&self.alloc` (by-ref allocator) lowered to a C++ POINTER,
            # but Box::new_in/from_raw_in take A by value. Global is a
            # stateless empty struct — pass a copy.
            line = line.replace("&this->alloc", "this->alloc")
            # Rust `Option<NonNull<Node<T>>>` is COPY: push_front_node/
            # push_back_node assign `node` into a link field in the match arm
            # AND then into head/tail — two uses of one value. The emission
            # std::move's BOTH, so the second gets a moved-out husk (None) and
            # the list silently corrupts (runtime-caught). Copy instead —
            # exactly Rust's Copy semantics.
            line = line.replace("std::move(node_shadow1)", "node_shadow1")
            # Rust `Box<Node<T>, &A>` (allocator-BY-REFERENCE Box) — with the
            # stateless Global it degenerates to Box<Node<T>>; and rusty::Box
            # has from_raw (no allocator-taking from_raw_in).
            line = line.replace("rusty::Box<Node<T>, const A&>", "rusty::Box<Node<T>>")
            line = line.replace(
                "::from_raw_in(rusty::as_ptr(node), this->alloc)",
                "::from_raw(rusty::as_ptr(node))",
            )
            # Cursor paths: the Box type arg was spelled from decltype(as_ptr(x))
            # = Node<T>* (a POINTER — Box-of-pointer is wrong); unwrap it and
            # drop the by-ref allocator arg.
            line = re.sub(
                r"rusty::Box<std::remove_cvref_t<decltype\(\(rusty::as_ptr\((\w+)\)\)\)>>"
                r"::from_raw_in\(rusty::as_ptr\(\1\), &this->list\.alloc\)",
                r"rusty::Box<std::remove_pointer_t<std::remove_cvref_t<decltype((rusty::as_ptr(\1)))>>>"
                r"::from_raw(rusty::as_ptr(\1))",
                line,
            )
        depth += line.count("{") - line.count("}")
        while ll_stack and depth <= ll_stack[-1]:
            ll_stack.pop()
        out.append(line)
    t = "".join(out)
    lines = t.splitlines(keepends=True)
    out, i = [], 0
    while i < len(lines):
        m = re.match(r"^(\s*)void assert_covariance\(\) \{\s*$", lines[i])
        if m:
            indent = m.group(1)
            j = i + 1
            while j < len(lines) and not lines[j].startswith(indent + "}"):
                j += 1
            out.append(indent + "void assert_covariance() {}\n")
            i = j + 1
            continue
        out.append(lines[i])
        i += 1
    return "".join(out)


def _replace_method_body(t: str, sig_marker: str, new_body_line: str) -> str:
    """Replace the balanced-brace body of the method whose signature LINE
    contains sig_marker (and an opening `{`) with `{ new_body_line }`.
    Line-walk, all occurrences."""
    lines = t.splitlines(keepends=True)
    out, i = [], 0
    while i < len(lines):
        line = lines[i]
        if sig_marker in line and "{" in line and not line.lstrip().startswith("//"):
            indent = re.match(r"\s*", line).group(0)
            head = line[: line.index("{") + 1]
            out.append(f"{head} {new_body_line} }}\n")
            depth = 0
            seen = False
            j = i
            while j < len(lines):
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1
                        seen = True
                    elif ch == "}":
                        depth -= 1
                j += 1
                if seen and depth == 0:
                    break
            i = j
            continue
        out.append(line)
        i += 1
    return "".join(out)


def _delete_method(t: str, sig_marker: str) -> str:
    """Delete the whole method (signature line through balanced close) whose
    signature line contains sig_marker. Line-walk, all occurrences."""
    lines = t.splitlines(keepends=True)
    out, i = [], 0
    while i < len(lines):
        line = lines[i]
        if sig_marker in line and "{" in line and not line.lstrip().startswith("//"):
            depth = 0
            seen = False
            j = i
            while j < len(lines):
                for ch in lines[j]:
                    if ch == "{":
                        depth += 1
                        seen = True
                    elif ch == "}":
                        depth -= 1
                j += 1
                if seen and depth == 0:
                    break
            i = j
            continue
        out.append(line)
        i += 1
    return "".join(out)


def _rc_rules(t: str) -> str:
    """rc.rs (Rc/Weak) submodule rules."""
    # `Global.allocate(...)` — a unit TYPE used with the dot operator.
    t = t.replace("rusty::alloc::Global.", "rusty::alloc::Global{}.")
    # to_rc_slice UFCS fwd-decls use a 1-arg Rc before the defaulted decl.
    t = t.replace(
        "::rc::Rc<std::span<const T>> to_rc_slice",
        "::rc::Rc<std::span<const T>, rusty::alloc::Global> to_rc_slice",
    )
    # Rust passes the fn item `<*mut u8>::cast` as allocate_for_layout's
    # mem_to_rc_inner callback; it emits as bare `::cast` (unparseable). The
    # DEF below re-types the pointer itself (correctly, with ITS OWN T), so
    # the callback arg is dead — pass a parseable dummy.
    t = t.replace(", ::cast)", ", 0)")
    t = t.replace(
        "auto inner = mem_to_rc_inner(rusty::as_ptr(ptr_shadow1.as_non_null_ptr()));",
        "auto inner = reinterpret_cast<std::add_pointer_t<RcInner<T>>>("
        "rusty::as_ptr(ptr_shadow1.as_non_null_ptr())); (void)mem_to_rc_inner;",
    )
    # `let x = if let Some(v) = e { v } else { return };` — the diverging else
    # embeds `return` as an EXPRESSION (`decltype((return))`, `.emplace(return)`).
    # Rewrite to an early-return. (Transpiler bug, also hit by sync.rs.)
    t = re.sub(
        r"std::optional<std::remove_cvref_t<decltype\(\(return\)\)>> (\w+);\n"
        r"(\s*)\{\n"
        r"\s*auto&& (\w+) = ([^\n]+);\n"
        r"\s*if \(\3\.is_some\(\)\) \{\n"
        r"\s*auto (\w+) = \3\.unwrap\(\);\n"
        r"\s*\1\.emplace\(\5\);\n"
        r"\s*\} else \{ \1\.emplace\(return\); \}\n"
        r"\s*\}\n"
        r"\s*const auto (\w+) = std::move\(\1\)\.value\(\);",
        r"auto&& \3 = \4;\n"
        r"\2if (!\3.is_some()) { return; }\n"
        r"\2const auto \6 = \3.unwrap();",
        t,
    )
    # dyn-Any downcast (Rust `Rc<dyn Any>::downcast`) — no C++ dyn-Any model;
    # abort-stub (signature kept so the class parses).
    t = _replace_method_body(
        t, "rusty::Result<Rc<T, A>, Rc<T, A>> downcast() const {", "std::abort();"
    )
    t = _replace_method_body(t, "Rc<T, A> downcast_unchecked() const {", "std::abort();")
    # From<&str> for Rc<str> — the body routes through a span-of-span
    # mis-emission and collides with another from(string_view) overload;
    # str-Rc isn't part of the port surface. Delete.
    t = _delete_method(t, "static Rc<std::string_view> from(std::string_view v) {")
    # is_dangling: `(ptr as *const ()).addr() == usize::MAX` — addr() on a
    # cast raw pointer; spell the address read directly.
    t = t.replace(
        "(reinterpret_cast<const rusty::Unit*>(ptr))->addr()",
        "reinterpret_cast<std::uintptr_t>(ptr)",
    )
    # make_mut: local shadows the free fn (`auto size_of_val = size_of_val(…)`
    # — deduced type in its own initializer).
    t = t.replace(
        "auto size_of_val = size_of_val(",
        "auto size_of_val_v = size_of_val(",
    )
    t = t.replace(
        "reinterpret_cast<uint8_t*>(in_progress.data_ptr()), std::move(size_of_val));",
        "reinterpret_cast<uint8_t*>(in_progress.data_ptr()), std::move(size_of_val_v));",
    )
    # UniqueRc::drop: `drop_in_place((*this).deref_mut())` — no deref_mut
    # member; the target is the RcInner's value field.
    t = t.replace(
        "drop_in_place((*this).deref_mut());",
        "drop_in_place(&(*rusty::as_ptr(this->ptr)).value);",
    )
    # drop_slow builds Rust's `Weak<T, &A>` (allocator BY REFERENCE); with the
    # stateless Global pass a copy (the by-value ctor).
    t = t.replace(
        "Weak<T, A>(this->ptr, &this->alloc)",
        "Weak<T, A>(this->ptr, this->alloc)",
    )
    # `Rc::from_inner(Box::leak(b).into())` — the NonNull-from-ref `.into()`
    # is dropped, leaving a raw RcInner<T>*; from_inner takes NonNull. Wrap.
    t = re.sub(
        r"from_inner\(\((rusty::Box<RcInner<T>>::[^;]*)\)\.leak\(\)\);",
        r"from_inner(rusty::ptr::NonNull<RcInner<T>>::new_unchecked((\1).leak()));",
        t,
    )
    return t


def _arc_rules(t: str) -> str:
    """sync.rs (Arc/Weak) rules — mirrors the rc rules where the family
    recurs; arc-only items below. (The module namespace may be renamed
    `sync_mod` by the conflict-rename machinery — cover both spellings.)"""
    for ns in ("sync", "sync_mod"):
        t = t.replace(
            f"::{ns}::Arc<std::span<const T>> to_arc_slice",
            f"::{ns}::Arc<std::span<const T>, rusty::alloc::Global> to_arc_slice",
        )
    # Rust `hint::spin_loop()` — a CPU pause hint; safe to elide.
    t = t.replace("std::hint::spin_loop();", "/* spin_loop hint */;")
    # panicking::panic_display -> the existing panic_fmt(std::string) entry.
    t = t.replace(
        "rusty::panicking::panic_display(",
        "rusty::panicking::panic_fmt(std::string(",
    )
    t = t.replace(
        "rusty::panicking::panic_fmt(std::string(INTERNAL_OVERFLOW_ERROR);",
        "rusty::panicking::panic_fmt(std::string(INTERNAL_OVERFLOW_ERROR));",
    )
    # dyn-Any downcast + nightly error::Request provide — no C++ model.
    t = _replace_method_body(
        t, "rusty::Result<Arc<T, A>, Arc<T, A>> downcast() const {", "std::abort();"
    )
    t = _replace_method_body(t, "Arc<T, A> downcast_unchecked() const {", "std::abort();")
    t = _delete_method(t, "void provide(rusty::error::Request& req) const {")
    # From<&str> for Arc<str> — same span-of-span mis-emission as Rc's.
    t = _delete_method(t, "static Arc<std::string_view> from(std::string_view v) {")
    # Arc's drop_slow / from_inner leak-wrap mirror the rc shapes with
    # ArcInner — reuse by textual analogy.
    t = re.sub(
        r"from_inner\(\((rusty::Box<ArcInner<T>>::[^;]*)\)\.leak\(\)\);",
        r"from_inner(rusty::ptr::NonNull<ArcInner<T>>::new_unchecked((\1).leak()));",
        t,
    )
    t = t.replace(
        "auto inner = mem_to_arc_inner(rusty::as_ptr(ptr_shadow1.as_non_null_ptr()));",
        "auto inner = reinterpret_cast<std::add_pointer_t<ArcInner<T>>>("
        "rusty::as_ptr(ptr_shadow1.as_non_null_ptr())); (void)mem_to_arc_inner;",
    )
    # Arc::new_: `let x = Box::new(ArcInner{..})` emitted as a CONST local,
    # but leak() mutates; and from_inner takes NonNull. Make the local
    # mutable + wrap.
    t = t.replace(
        "const auto x = rusty::Box<ArcInner<T>>::new_(",
        "auto x = rusty::Box<ArcInner<T>>::new_(",
    )
    t = t.replace(
        "return Arc<T, A>::from_inner((std::move(x)).leak());",
        "return Arc<T, A>::from_inner("
        "rusty::ptr::NonNull<ArcInner<T>>::new_unchecked((std::move(x)).leak()));",
    )
    # `ptr::addr_eq(p, &STATIC_INNER_SLICE.inner)` — the address-of on the
    # static's field was dropped.
    t = t.replace(
        "addr_eq(rusty::as_ptr(this->ptr), STATIC_INNER_SLICE.inner)",
        "addr_eq(rusty::as_ptr(this->ptr), &STATIC_INNER_SLICE.inner)",
    )
    # is_dangling's param was emitted as `std::add_pointer_t<std::add_const_t
    # <T>>` — a NON-DEDUCED context, so bare calls can't infer T. Spell it
    # deducibly.
    t = t.replace(
        "is_dangling(std::add_pointer_t<std::add_const_t<T>> ptr)",
        "is_dangling(const T* ptr)",
    )
    return t


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
        # LINE-BOUNDED ([^\n]*, no DOTALL): with DOTALL the non-greedy spans
        # matched ACROSS functions once new submodules added more ptr::add
        # sites, deleting a brace pair mid-Vec (boxed probe: a 698-error
        # structural cascade). The rule's shape is strictly 4 consecutive
        # lines — encode that.
        t = re.sub(
            r"\n( +)\{\n"
            r"( +auto ptr_shadow1 = rusty::ptr::add\([^\n]*\n"
            r" +auto ret = rusty::ptr::read\(ptr_shadow1\);\n"
            r" +rusty::ptr::copy\([^\n]*\n)"
            r" +\}\n"
            r"( +this->set_len)",
            r"\n\1\2\3",
            t,
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
        # rc.rs (Rc/Weak) + sync.rs (Arc) rules.
        t = _rc_rules(t)
        t = _arc_rules(t)
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
        # Vec::operator[]/index_mut lower Rust's `Index::index` to `.index()` on
        # the dereffed `*this`, but that derefs to std::span which has no
        # .index(). Route to `as_slice(*this)[i]` (vendored vec_port does the
        # same; its rule is anchored to 4-space indent so it misses the single
        # module — the body expr is indent-independent, replace directly).
        t = t.replace(
            "(rusty::detail::deref_if_pointer_like((*this))).index(std::move(index))",
            "rusty::as_slice((*this))[static_cast<size_t>(index)]",
        )
        t = t.replace(
            "(rusty::detail::deref_if_pointer_like((*this))).index_mut(std::move(index))",
            "rusty::as_mut_slice((*this))[static_cast<size_t>(index)]",
        )
        # --- instantiation-time fixes (surface when Vec/VecDeque are actually
        # used with a concrete T; the BMI precompile skips these template
        # bodies). These mirror vec_port/vec_deque_port's file-specific rules. ---
        # Vec::clone: `std::span::to_vec_in` doesn't exist; do with_capacity_in
        # + a clone loop (vendored form).
        t = t.replace(
            "return std::span<const T>::to_vec_in("
            "rusty::detail::deref_if_pointer_like((*this)), std::move(alloc));",
            "auto out = Vec<T, A>::with_capacity_in(this->len_field, std::move(alloc)); "
            "auto src = rusty::as_slice(*this); "
            "for (size_t i = 0; i < src.size(); ++i) { out.push(rusty::clone(src[i])); } "
            "return out;",
        )
        # `T::IS_ZST` / `::IS_ZST` are ill-formed for primitive T (`int::IS_ZST`)
        # in EVERY position — `if (…)`, `… ? a : b` ternaries, bare exprs. Rust's
        # ZST check is `size_of::<T>()==0`; C++ has no zero-size types so
        # `(sizeof(T)==0)` is always false — the exact vendored vec_deque_port
        # rule. Do the general rewrite first, then make the `if`-statement form
        # `if constexpr` so the (dead) ZST branch body isn't even instantiated.
        t = re.sub(r"(?<![A-Za-z0-9_:])::IS_ZST\b", "(sizeof(T) == 0)", t)
        t = re.sub(r"(?<![A-Za-z0-9_])T::IS_ZST\b", "(sizeof(T) == 0)", t)
        t = t.replace("if ((sizeof(T) == 0)) {", "if constexpr ((sizeof(T) == 0)) {")
        # ── Raw slice pointer (`*mut [T]`/`*const [T]`) is a Rust FAT pointer; it
        # must lower to std::span<T> (a VALUE), not `std::span<T>*`. The
        # transpiler mismodels it as `std::add_pointer_t<std::span<T>>` on the
        # method RETURN type (buffer_range/as_raw_mut_slice/Drain::as_slices) and
        # then callers deref `*this->buffer_range(...)`. This single bug blocks
        # ALL VecDeque construction (~VecDeque→as_mut_slices→buffer_range).
        # Fix: span value return + drop the deref (do NOT stub ~VecDeque like
        # vec_deque_port — stubbing leaks non-trivial elements).
        t = t.replace("std::add_pointer_t<std::span<T>>", "std::span<T>")
        t = re.sub(r"\*this->buffer_range\(", "this->buffer_range(", t)
        t = re.sub(r"\*this->as_raw_mut_slice\(", "this->as_raw_mut_slice(", t)
        # `expr as *mut [T]` also surfaces as `const_cast<…span<T>*…>(slice_from(x))`
        # (truncate/clear's drop_back/drop_front). slice_from returns a span
        # value; drop_in_place accepts a span. Strip the const_cast<span*> wrap.
        t = re.sub(
            r"const_cast<std::remove_const_t<std::remove_pointer_t<std::remove_cvref_t<"
            r"decltype\(\(rusty::slice_from\(([^)]*)\)\)\)>>>\*>\(rusty::slice_from\(\1\)\)",
            r"rusty::slice_from(\1)",
            t,
        )
        # UFCS-migration regression: inherent `.get()`/`.get_mut()` on VecDeque
        # (a non-contiguous type) routed through the `rusty::get` slice helper,
        # which mis-types the deque as its own element. Call the member directly.
        t = t.replace("rusty::get((*this), ", "this->get(")
        t = t.replace("rusty::get_mut((*this), ", "this->get_mut(")
        # Vec has no begin()/end() members, so range-for and the winnow
        # slice-`contains` dispatcher don't fire (contains silently returns
        # false). Inject them (verbatim vec_port), anchored before as_mut_ptr.
        t = t.replace(
            "constexpr std::add_pointer_t<T> as_mut_ptr() {",
            "T* begin() { return this->as_mut_ptr(); }\n"
            "        T* end() { return this->as_mut_ptr() + this->len_field; }\n"
            "        const T* begin() const { return this->as_ptr(); }\n"
            "        const T* end() const { return this->as_ptr() + this->len_field; }\n"
            "        constexpr std::add_pointer_t<T> as_mut_ptr() {",
        )
        # Vec::from([T;N]): the owned by-value From is the intended one; the
        # `std::array<T,N>&` reference overload only collides under C++ ref
        # binding (3-overload ambiguity). Delete it.
        t = re.sub(
            r"\n\s*template<size_t N>\n\s*static Vec<T> from\(std::array<T, N>& s\) \{\n"
            r"\s*return Vec<T, A>::from\(rusty::as_mut_slice\(s\)\);\n\s*\}\n",
            "\n",
            t,
        )
        # RawVecInner::shrink_unchecked passes the raw NonNull<u8> from
        # alloc.shrink() straight into set_ptr_and_cap, whose param is the fat
        # NonNullSlice<u8> with an EXPLICIT NonNull ctor → copy-init ill-formed.
        # Direct-init via a functional cast fires the ctor (len_=0 is unused by
        # set_ptr_and_cap, which only takes ptr.cast()). ptr_shadow2 is unique
        # to shrink_unchecked.
        t = t.replace(
            "this->set_ptr_and_cap(std::move(ptr_shadow2), std::move(cap));",
            "this->set_ptr_and_cap(rusty::ptr::NonNullSlice<uint8_t>(std::move(ptr_shadow2)), std::move(cap));",
        )
        # ── ManuallyDrop `me` receivers not dereferenced (recurring across
        # into_raw_parts_with_alloc AND into_iter; the same bodies use `(*me)`
        # correctly two lines away — inconsistent emission). `me` is always a
        # `manually_drop_new(...)` local in these bodies, so generalize:
        t = t.replace("rusty::len(me)", "rusty::len((*me))")
        t = t.replace("auto capacity = me.capacity();", "auto capacity = (*me).capacity();")
        # allocator() returns a const ref; ptr::read wants a pointer → move it
        # out via const_cast (vendored vec_port form). Covers both the
        # `auto alloc = …;` and `manually_drop_new(ptr::read(me.allocator()))` forms.
        t = t.replace(
            "rusty::ptr::read(me.allocator())",
            "std::move(const_cast<A&>((*me).allocator()))",
        )
        # Vec::from(VecDeque) len (the allocator/capacity derefs live in the
        # existing Vec::from(VecDeque) rule block below).
        t = t.replace(
            "auto len = rusty::len(other_shadow1);",
            "auto len = rusty::len((*other_shadow1));",
        )
        # ── Raw-pointer methods emitted as C++ member calls on a bare T*
        # (Rust `*mut T`::add/cast). buf is `.buf.ptr()` (a T*): `buf.add(n)`
        # -> rusty::ptr::add(buf, n). from_raw_parts_in casts T* -> uint8_t* for
        # the type-erased RawVecInner: `ptr.cast()` -> reinterpret_cast.
        t = t.replace(
            "rusty::ptr::copy(buf.add(std::move(rusty::detail::deref_if_pointer((*other_shadow1)).head)), std::move(buf), std::move(len));",
            "rusty::ptr::copy(rusty::ptr::add(buf, std::move(rusty::detail::deref_if_pointer((*other_shadow1)).head)), std::move(buf), std::move(len));",
        )
        t = t.replace(
            "const auto ptr_shadow1 = ptr.cast();",
            "const auto ptr_shadow1 = reinterpret_cast<uint8_t*>(ptr);",
        )
        # The (dead) ZST arms of `(sizeof(T)==0) ? … : …` RUNTIME ternaries in
        # Vec::into_iter/IntoIter::size_hint call raw-pointer methods
        # (`begin->wrapping_byte_add`, `p->addr()`) on a bare `const T*`, which
        # must still COMPILE. Lower them to pointer arithmetic on the raw ptr.
        t = t.replace(
            "begin->wrapping_byte_add(rusty::len((*me)))",
            "reinterpret_cast<std::add_pointer_t<std::add_const_t<T>>>("
            "reinterpret_cast<const char*>(begin) + rusty::len((*me)))",
        )
        # Vec::into_iter builds the result with a DESIGNATED initializer, but
        # IntoIter has user-declared ctors (non-aggregate). Use its positional
        # ctor (same field order). buf is a trivially-copyable NonNull so the
        # double std::move is a pointer copy (both buf and ptr get it).
        t = t.replace(
            "return IntoIter{.buf = std::move(buf), .phantom = rusty::PhantomData<T>{}, "
            ".cap = std::move(cap), .alloc = std::move(alloc), .ptr = std::move(buf), "
            ".end = std::move(end)};",
            "return IntoIter(std::move(buf), rusty::PhantomData<T>{}, std::move(cap), "
            "std::move(alloc), std::move(buf), std::move(end));",
        )
        # IntoIter::size_hint's `exact` is a convoluted `(sizeof==0)? … : …`
        # with a `*ptr.method()` precedence bug in the else arm. `end - ptr` (in
        # elements) is the exact remaining count; compute it by byte-diff/size.
        t = re.sub(
            r"auto exact = \(\(sizeof\(T\) == 0\) \?.*?\);\n(\s*return std::make_tuple\(std::move\(exact\))",
            r"auto exact = (reinterpret_cast<std::size_t>(this->end) - "
            r"reinterpret_cast<std::size_t>(rusty::as_ptr(this->ptr))) / (sizeof(T) == 0 ? 1 : sizeof(T));\n\1",
            t,
        )
        # slice_ranges: `slice_ext::range(...)` returns std::pair<size_t,size_t>
        # (slice.hpp:2400), so `.start`/`.end` must be `.first`/`.second`.
        t = t.replace(
            "rusty::detail::deref_if_pointer(_let_pat.start)",
            "rusty::detail::deref_if_pointer(_let_pat.first)",
        )
        t = t.replace(
            "rusty::detail::deref_if_pointer(_let_pat.end)",
            "rusty::detail::deref_if_pointer(_let_pat.second)",
        )
        # RawVec::non_null: `.cast()` yields a CastProxy that implicitly
        # converts to NonNull<T>; `.as_non_null_ptr()` forces it to NonNull<u8>
        # (wrong return type). Strip it (vendored raw_vec rule).
        t = t.replace(
            "this->ptr_field.cast().as_non_null_ptr()",
            "this->ptr_field.cast()",
        )
        # Vec::from(VecDeque): buf access derefs the ManuallyDrop but capacity()/
        # allocator() were emitted without the deref.
        t = t.replace(
            "auto cap = other_shadow1.capacity();",
            "auto cap = rusty::detail::deref_if_pointer((*other_shadow1)).capacity();",
        )
        t = t.replace(
            "auto alloc = rusty::ptr::read(other_shadow1.allocator());",
            "auto alloc = std::move(const_cast<A&>("
            "rusty::detail::deref_if_pointer((*other_shadow1)).allocator()));",
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
            # SpecFromElem must DECLARE a concrete return type (a deduced
            # return can't be used before the definition, and callers appear
            # mid-module); the real body is appended at end-of-module where
            # vec::Vec is complete.
            "namespace vec { export template<typename T, typename A> struct Vec; }\n"
            "struct SpecFromElem { template<typename T, typename A>"
            " static vec::Vec<T, A> from_elem(T elem, std::size_t n, A alloc); };\n"
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
            # from_elem's real body (vec![x; n]): out-of-line at end-of-module,
            # where vec::Vec is complete. Semantics = Rust's SpecFromElem
            # default: with_capacity + n cloned pushes.
            t += (
                "\n// vec![x; n] support (SpecFromElem::from_elem out-of-line body)\n"
                "template<typename T, typename A>\n"
                "vec::Vec<T, A> SpecFromElem::from_elem(T elem, std::size_t n, A alloc) {\n"
                "    auto v = vec::Vec<T, A>::with_capacity_in(n, std::move(alloc));\n"
                "    for (std::size_t i = 0; i < n; ++i) v.push(rusty::clone(elem));\n"
                "    return v;\n"
                "}\n"
            )
        # ── borrow (Cow/ToOwned) submodule ──
        # vec-side impls (From<Vec<T>>/PartialEq<Vec<U>> for Cow<[T]>, from
        # vec/cow.rs + vec/partial_eq.rs) are spliced into borrow::Cow's class
        # body but keep the `Vec` spelling that's valid only inside namespace
        # vec — qualify. (The operator== rule is indent-anchored: 8 spaces =
        # Cow's body; vec::Vec's own copy at 12 spaces is untouched.)
        t = t.replace(
            "static Cow<std::span<const T>> from(Vec<T> v) {",
            "static Cow<std::span<const T>> from(vec::Vec<T> v) {",
        )
        t = t.replace(
            "static Cow<std::span<const T>> from(const Vec<T>& v) {",
            "static Cow<std::span<const T>> from(const vec::Vec<T>& v) {",
        )
        t = t.replace(
            "\n        bool operator==(const Vec<U, A>& other) const {",
            "\n        bool operator==(const vec::Vec<U, A>& other) const {",
        )
        # ToOwnedTraits' primary template hard-requires B::Owned, but foreign B
        # (str -> std::string_view, [T] -> std::span<const T>) have no member
        # Owned; Rust's blanket `impl<T: Clone> ToOwned for T` makes identity
        # the fallback. SFINAE the primary + add the two foreign specializations.
        t = t.replace(
            "template <class B> struct ToOwnedTraits { using Owned = typename B::Owned; };\n"
            "    template <class S> struct ToOwnedTraits<S*> { using Owned = typename ToOwnedTraits<S>::Owned; };\n"
            "    template <class S> struct ToOwnedTraits<S&> { using Owned = typename ToOwnedTraits<S>::Owned; };",
            "template <class B, class = void> struct ToOwnedTraits { using Owned = std::remove_cvref_t<B>; };\n"
            "    template <class B> struct ToOwnedTraits<B, std::void_t<typename B::Owned>> { using Owned = typename B::Owned; };\n"
            "    template <class S> struct ToOwnedTraits<S*, void> { using Owned = typename ToOwnedTraits<S>::Owned; };\n"
            "    template <class S> struct ToOwnedTraits<S&, void> { using Owned = typename ToOwnedTraits<S>::Owned; };\n"
            "    template <> struct ToOwnedTraits<std::string_view, void> { using Owned = rusty::String; };\n"
            "    template <class T> struct ToOwnedTraits<std::span<const T>, void> { using Owned = vec::Vec<T>; };",
        )
        # ── binary_heap: usize::BITS leaked verbatim (assoc-const on primitive).
        t = t.replace("usize::BITS", "(8u * sizeof(size_t))")
        # binary_heap instantiation fixes (surface with concrete T):
        # Rust raw-pointer args emitted as lvalues — restore the address-of.
        t = t.replace(
            "auto elt = rusty::ptr::read(data[std::move(pos)]);",
            "auto elt = rusty::ptr::read(&data[std::move(pos)]);",
        )
        t = t.replace(
            "rusty::detail::deref_if_pointer_like(this->elt)), this->data[std::move(pos)], 1);",
            "rusty::detail::deref_if_pointer_like(this->elt)), &this->data[std::move(pos)], 1);",
        )
        # mem::swap takes T&, T& (Rust &mut lowered to pointer — strip).
        t = t.replace(
            "rusty::mem::swap(&item, &this->data[static_cast<size_t>(0)]);",
            "rusty::mem::swap(item, this->data[static_cast<size_t>(0)]);",
        )
        # Hole::element returns const T& but elt is ManuallyDrop<T> — deref.
        t = t.replace(
            "const T& element() const {\n                return this->elt;",
            "const T& element() const {\n                return *this->elt;",
        )
        # linked_list: Node::into_element(self: Box<Self>) — deref_if_pointer_like
        # does not peel rusty::Box; deref explicitly (Box::operator*).
        t = t.replace(
            "rusty::detail::deref_if_pointer_like(std::forward<decltype(_v)>(_v)).into_element()",
            "(*std::forward<decltype(_v)>(_v)).into_element()",
        )
        # ── region-scoped reverts/qualifications for the single-file submodules
        # (must run AFTER the vec-scoped global IntoIter rules above).
        t = _scope_binary_heap(t)
        t = _patch_linked_list(t)
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
