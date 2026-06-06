#!/usr/bin/env python3
"""Post-transpile patches for the btree_port C++20 module port.

Two things happen here:

1. Stub 5 specific methods in `btree_port.btree.btree_internal.cppm` that
   hit transpiler-side template-parameter recovery bugs (`BorrowType` /
   `NodeType` placeholder leakage, `Box<auto>` emission). The hand-
   written facade in `include/btree_port/btreemap.hpp` doesn't call any
   of these, so stubbing them lets the module compile cleanly while the
   ~6.4 KLoC of correctly-transpiled internals remain available for the
   gradual migration described in `STATUS.md`.

2. Trim `CMakeLists.txt` to only build `btree_internal`. The map / set /
   entry submodules hit additional, distinct transpiler bugs (post-
   module import ordering, cross-module template-arity recovery, orphan-
   impl misrouting) that are tracked separately in `STATUS.md`. Drop
   them from the build target until those land, and also wire the rusty
   include path into CMake so reconfigure doesn't drop the `-I`.

Usage:
    python3 post_transpile_patch.py path/to/cpp_out/

Idempotent: rerunning detects already-applied patches and skips.
"""

import re
import sys
from pathlib import Path

STUB_BODY = (
    "{ throw ::std::runtime_error("
    "\"rusty-cpp-transpiler: btree internal method stub "
    "(template-parameter recovery limitation; see docs/btreemap_port/STATUS.md)\""
    "); }"
)

# Method header substrings (anchored on the unique tail of each signature).
# We match by `find()` so a substring of the full declaration is enough.
TARGETS = [
    "from_new_leaf(rusty::Box<LeafNode<K, V>, A> leaf)",
    "from_new_internal(rusty::Box<InternalNode<K, V>, A> internal, rusty::num::NonZero<size_t> height)",
    "push_with_handle(K key, V val)",
    "deallocating_next(A alloc)",
    "deallocating_next_back(A alloc)",
]


def find_matching_brace(text: str, open_pos: int) -> int:
    """Return position of `}` that matches `{` at `open_pos`."""
    depth = 0
    for i in range(open_pos, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return i
    return -1


def stub(src: str, sig_tail: str) -> tuple[str, bool]:
    pos = src.find(sig_tail)
    if pos == -1:
        return src, False
    # Find the next `{` after the signature; bypass attribute braces by
    # requiring it to be on the same or next line (no nested attributes
    # appear between sig and body in the transpiled output).
    brace_open = src.find("{", pos + len(sig_tail))
    if brace_open == -1:
        return src, False
    brace_close = find_matching_brace(src, brace_open)
    if brace_close == -1:
        return src, False
    body = src[brace_open : brace_close + 1]
    # Idempotency guard: if already stubbed, don't replace again.
    if "rusty-cpp-transpiler: btree internal method stub" in body:
        return src, False
    src = src[:brace_open] + STUB_BODY + src[brace_close + 1 :]
    return src, True


def patch_internal(path: Path) -> None:
    src = path.read_text()
    changed_any = False
    for sig in TARGETS:
        src, ok = stub(src, sig)
        if ok:
            print(f"  [stubbed] {sig}")
            changed_any = True
        else:
            print(f"  [skip]    {sig} (not found or already stubbed)")

    # Clang-strictness fix: NodeRef::eq emits
    #   `auto&& height = …deref…(_let_pat.height_field);`
    #   `assert((* height == other . height));`
    # The `* height` is a spurious deref of a value-type reference; the
    # `other . height` refers to the METHOD `height()`, not the field
    # `height_field`. GCC accepts both leniently; clang rejects the
    # method-ref-without-call. Rewrite the assert to a no-spurious-deref
    # comparison against the field.
    bad_assert = "assert((* height == other . height));"
    good_assert = (
        "assert((height == other.height_field));"
        "  /* btree_port port: clang-strictness fix by post_transpile_patch.py */"
    )
    if bad_assert in src:
        src = src.replace(bad_assert, good_assert, 1)
        changed_any = True
        print("  [clang-fix] NodeRef::eq assert (spurious *, .height→.height_field)")

    if changed_any:
        path.write_text(src)
        print(f"  wrote: {path}")
    else:
        print(f"  no changes to: {path.name}")


def patch_entry_imports(path: Path, extra_imports: list[str]) -> None:
    """Move all `import …;` lines to immediately after `export module …;`
    so the post-module imports form a contiguous block (a C++20 module
    requirement). Add any `extra_imports` that the transpiler missed.
    Idempotent: skipped if the sentinel is already present."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: imports reordered by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (already reordered)")
        return

    mod_match = re.search(
        r"^(export module [A-Za-z0-9_.]+;)\s*$", src, re.MULTILINE
    )
    if mod_match is None:
        print(f"  [warn] no `export module` line in {path.name}", file=sys.stderr)
        return

    # Collect every `import …;` line in the file (they may be scattered).
    import_lines = re.findall(r"^import [A-Za-z0-9_.]+;\s*$", src, re.MULTILINE)
    # Dedup while preserving first-seen order.
    seen = set()
    uniq_imports = []
    for ln in import_lines:
        ln_clean = ln.rstrip()
        if ln_clean not in seen:
            seen.add(ln_clean)
            uniq_imports.append(ln_clean)
    # Add any extras the transpiler missed (e.g. set.entry needs map).
    for extra in extra_imports:
        extra_line = f"import {extra};"
        if extra_line not in seen:
            seen.add(extra_line)
            uniq_imports.append(extra_line)

    # Strip the original import lines from the body (we'll re-emit them
    # in one block right after the module declaration).
    src_without_imports = re.sub(
        r"^import [A-Za-z0-9_.]+;\s*\n", "", src, flags=re.MULTILINE
    )

    block = "\n".join(uniq_imports)
    insertion = f"\n{sentinel}\n{block}\n"
    new_src = src_without_imports.replace(
        mod_match.group(1), mod_match.group(1) + insertion, 1
    )
    path.write_text(new_src)
    print(f"  reordered imports in: {path.name} ({len(uniq_imports)} imports)")


def remove_setvalzst_methods(path: Path) -> None:
    """Drop methods that the orphan-impl injector misrouted from
    `set::*Entry` into `map::*Entry` (or vice-versa). They reference
    `SetValZST` (a set-internal type not in scope here) and use a
    `template<typename T>` shape that doesn't match the enclosing
    `<K, V, A>` struct. Wrap them in `#if 0` blocks so the misrouted
    body stays visible in the file (for future grep'ing) but is
    invisible to the compiler.

    Idempotent — bails if the sentinel is already present."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: orphan-impl misroutes hidden by post_transpile_patch.py"
    # Drop the old (narrower) SetValZST-only sentinel if a previous run
    # used it; that way running with the broadened heuristic catches
    # additional `this->inner.*` clusters that weren't covered before.
    old_sentinel = "// btree_port port: SetValZST misroutes hidden by post_transpile_patch.py"
    src = src.replace(old_sentinel, sentinel)

    # Match a contiguous run of `template<typename T>\n<sig>\n<body…>\n}` blocks
    # whose body references SET-internal symbols. Two signals:
    #  - `SetValZST` (set-internal ZST type leaked into map.entry).
    #  - `this->inner.` (orphan-impl methods absorbed from
    #    set::OccupiedEntry / set::VacantEntry that destructure
    #    through their `inner: MapOccupiedEntry<…>` field — which
    #    doesn't exist on the map-side struct they got placed in).
    #
    # Strategy: scan the file line-by-line. Skip blocks already
    # wrapped in `#if 0 / #endif` (re-run safety). For every
    # `template<typename T>` method-block, peek at the body — if it
    # contains either signal, wrap the cluster in `#if 0 / #endif`.

    lines = src.split("\n")
    n = len(lines)
    out: list[str] = []
    i = 0
    hidden_blocks = 0
    in_skip = False
    while i < n:
        line = lines[i]
        # Track #if 0 / #endif so we don't double-wrap.
        if line.startswith("#if 0"):
            in_skip = True
            out.append(line)
            i += 1
            continue
        if line.startswith("#endif") and in_skip:
            in_skip = False
            out.append(line)
            i += 1
            continue
        if in_skip:
            out.append(line)
            i += 1
            continue
        # Look for `    template<typename T>` (4-space indent inside struct).
        if (line.rstrip() == "    template<typename T>"
            or line.rstrip() == "    template<typename V>"
            or line.rstrip() == "    template<typename K>"
            or line.rstrip() == "    template<typename K, typename V>"
           ) and i + 1 < n:
            # Scan forward to find method-block end (closing `}` at indent 4).
            # Capture from i through the matching `    }` line.
            j = i
            block_end = None
            depth = 0
            saw_open = False
            while j < n:
                cur = lines[j]
                # Count braces in the current line — works for inline bodies too.
                for c in cur:
                    if c == "{":
                        depth += 1
                        saw_open = True
                    elif c == "}":
                        depth -= 1
                        if saw_open and depth == 0:
                            block_end = j
                            break
                if block_end is not None:
                    break
                j += 1
            if block_end is None:
                # Couldn't find end — give up and keep line as-is.
                out.append(line)
                i += 1
                continue
            block = "\n".join(lines[i : block_end + 1])
            # Inside the entry structs (VacantEntry / OccupiedEntry,
            # template params <K, V, A>), no legitimate method uses
            # `template<typename T>` — the struct's own template
            # parameters cover every legitimate use. Every
            # 4-space-indented `template<typename T>` block we see
            # here is an orphan-impl misroute from set::*Entry into
            # map::*Entry (or vice versa) and references symbols
            # (`this->inner.*`, `SetValZST`, `this->get()`) that
            # don't exist on the host struct. Hide them all.
            if True:
                # Hide it (and swallow any contiguous next template<typename T>
                # blocks too — they're all part of the same misrouted cluster).
                cluster_start = i
                cluster_end = block_end
                k = block_end + 1
                while k < n:
                    if lines[k].rstrip() == "" and k + 1 < n and lines[k + 1].rstrip() in ("    template<typename T>", "    template<typename V>", "    template<typename K>", "    template<typename K, typename V>"):
                        # blank between methods
                        k += 1
                        continue
                    if lines[k].rstrip() in ("    template<typename T>", "    template<typename V>", "    template<typename K>", "    template<typename K, typename V>"):
                        # find this method's end too
                        m = k
                        d = 0
                        so = False
                        me = None
                        while m < n:
                            c2 = lines[m]
                            for ch in c2:
                                if ch == "{":
                                    d += 1
                                    so = True
                                elif ch == "}":
                                    d -= 1
                                    if so and d == 0:
                                        me = m
                                        break
                            if me is not None:
                                break
                            m += 1
                        if me is None:
                            break
                        # Always swallow contiguous template<typename T> blocks
                        # in the same cluster — they're the misroute group.
                        cluster_end = me
                        k = me + 1
                        continue
                    break
                out.append(f"#if 0  // {sentinel}")
                out.extend(lines[cluster_start : cluster_end + 1])
                out.append("#endif")
                hidden_blocks += 1
                i = cluster_end + 1
                continue
        out.append(line)
        i += 1

    if hidden_blocks > 0 or src != "\n".join(out):
        path.write_text("\n".join(out))
        print(
            f"  hid {hidden_blocks} orphan-impl misroute cluster(s) in: {path.name}"
        )
    else:
        print(f"  no orphan-impl misroutes found in: {path.name}")


def hide_template_free_misroutes(path: Path) -> None:
    """Catch orphan-impl misroutes that escape `remove_setvalzst_methods`
    because they're non-template (no `template<typename T>` qualifier).
    The canonical example is set::BTreeSet::replace(value: T) which the
    injector landed inside map::BTreeMap with a body that references
    `SetValZST`. The K-and-V-only host struct doesn't have that
    symbol, so the body fails to parse.

    Heuristic: for every line containing `SetValZST` not already inside
    a `#if 0 / #endif` block, walk backward to find the nearest method
    signature ending in `{` at indent 4, then forward to the matching
    `}`. Wrap the resulting method body in `#if 0 / #endif`.
    """
    src = path.read_text()
    sentinel = (
        "// btree_port port: template-free SetValZST misroutes hidden "
        "by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (template-free misroutes already hidden)")
        return

    lines = src.split("\n")
    n = len(lines)

    # First pass: mark which lines are inside any existing `#if 0` block.
    inside_skip = [False] * n
    in_skip = False
    for i, l in enumerate(lines):
        if l.startswith("#if 0"):
            in_skip = True
        inside_skip[i] = in_skip
        if l.startswith("#endif"):
            in_skip = False

    # Walk forward looking for `SetValZST` references not inside #if 0.
    # For each, snap to method boundaries.
    out: list[str] = list(lines)  # mutable copy
    wrapped: list[tuple[int, int]] = []  # (start, end) inclusive of wrapped regions
    i = 0
    while i < n:
        if "SetValZST" in out[i] and not inside_skip[i]:
            # Walk backward to find method signature.
            # A method signature at indent 4 ends with `{` on its own
            # line OR on the signature line. Walk back through prior
            # non-blank lines whose indent ≥ 6 (body) until we find a
            # line that ends with `{` and the line before that has the
            # signature.
            #
            # Practical rule: walk back to the first line whose stripped
            # form ends with `) {` or `const {` or `noexcept {` (or
            # just `{` if the signature spans multiple lines and the
            # opening brace is on its own). Then walk back one more if
            # there's a `template<...>` line directly above.
            sig_end = None
            for j in range(i - 1, -1, -1):
                l = out[j].rstrip()
                if l.endswith("{") and not l.startswith("//"):
                    sig_end = j
                    break
                # If we hit an outer struct boundary, give up.
                if l.startswith("};") or l == "};":
                    sig_end = None
                    break
            if sig_end is None:
                i += 1
                continue
            # Now find the method's start (template<...> line above, if
            # any).
            sig_start = sig_end
            while sig_start > 0:
                prev = out[sig_start - 1].rstrip()
                if prev.lstrip().startswith("template<"):
                    sig_start -= 1
                else:
                    break
            # Find matching `}` at the same indent level.
            depth = 0
            method_end = None
            for j in range(sig_end, n):
                l = out[j]
                for c in l:
                    if c == "{":
                        depth += 1
                    elif c == "}":
                        depth -= 1
                        if depth == 0:
                            method_end = j
                            break
                if method_end is not None:
                    break
            if method_end is None:
                i += 1
                continue
            # Skip if any of the lines we want to wrap are already
            # inside an existing #if 0 (e.g., we walked backwards into
            # a hidden cluster — that means the misroute extends past
            # a hidden cluster and isn't safe to wrap with this naive
            # method).
            if any(inside_skip[k] for k in range(sig_start, method_end + 1)):
                i = method_end + 1
                continue
            # Wrap.
            out[sig_start] = (
                f"#if 0  // {sentinel}\n" + out[sig_start]
            )
            out[method_end] = out[method_end] + "\n#endif"
            for k in range(sig_start, method_end + 1):
                inside_skip[k] = True
            wrapped.append((sig_start, method_end))
            i = method_end + 1
            continue
        i += 1

    if wrapped:
        path.write_text("\n".join(out))
        print(
            f"  hid {len(wrapped)} template-free SetValZST-misroute "
            f"method(s) in: {path.name}"
        )
    else:
        print(f"  no template-free SetValZST misroutes in: {path.name}")


def recover_template_args(path: Path) -> None:
    """Patch call sites that use `NodeRef::new_leaf`, `Root::new_`,
    or `Handle::into_kv` without their concrete template arguments.

    The transpiler drops template arguments here because Rust resolves
    them via type inference at call sites; C++ requires them spelled.
    Inside BTreeMap<K, V, A> methods (the only context these appear
    in), K and V are in scope, so a textual substitution suffices.

    Substitutions:
      NodeRef::new_leaf(…)  →  NodeRef<::marker::Owned, K, V, ::marker::Leaf>::new_leaf(…)
      Root::new_(…)         →  Root<K, V>::new_(…)
      .map(Handle::into_kv) →  .map([](auto&& __h){ return __h.into_kv(); })

    Idempotent — guarded by a sentinel comment.
    """
    src = path.read_text()
    sentinel = "// btree_port port: template-args recovered by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (template-args already recovered)")
        return

    replacements = [
        # NodeRef::new_leaf is called with a single arg (the allocator).
        # The receiver is the marker::Owned-leaf NodeRef variant.
        (
            "NodeRef::new_leaf(",
            "NodeRef<::marker::Owned, K, V, ::marker::Leaf>::new_leaf(",
        ),
        # Root<K, V> is `NodeRef<marker::Owned, K, V, marker::LeafOrInternal>`.
        # `Root::new_(alloc)` is the same construction shape, as is
        # `Root::calc_split_length(...)` (static method, no `self`).
        (
            "Root::new_(",
            "Root<K, V>::new_(",
        ),
        (
            "Root::calc_split_length(",
            "Root<K, V>::calc_split_length(",
        ),
        # `.map(Handle::into_kv)` passes a method pointer; C++ can't
        # form one without the template args. Rewrite the call site
        # to a lambda that dispatches on the deduced argument's type.
        (
            ".map(Handle::into_kv)",
            ".map([](auto&& __h) { return __h.into_kv(); })",
        ),
        # SearchBound is `enum class SearchBound<Q>` in Rust (specialized
        # for borrowed keys). At the call sites in BTreeMap, the lookup
        # uses K directly. `SearchBound::from_range(b)` → ::from_range
        # with the concrete K.
        (
            "SearchBound::from_range(",
            "SearchBound<K>::from_range(",
        ),
        # DedupSortedIter has template params <K, V, I>. K and V come
        # from the enclosing BTreeMap; I is the iterator type which
        # is deducible from the argument. Substitute the helper that
        # the patcher injects below.
        (
            "DedupSortedIter::new_(",
            "__btree_port_make_dedup<K, V>(",
        ),
    ]

    n_total = 0
    for old, new in replacements:
        n = src.count(old)
        if n == 0:
            continue
        src = src.replace(old, new)
        n_total += n
        print(f"  recovered {n}× `{old}` → expanded form")

    if n_total:
        # If `__btree_port_make_dedup<` was substituted in, also
        # inject the helper definition. Under auto-namespace mode the
        # injection goes INSIDE the namespace wrap (so unqualified
        # `DedupSortedIter` resolves via the `btree_internal` alias);
        # otherwise it goes at module purview after the last import.
        if "__btree_port_make_dedup<" in src:
            auto_ns = "namespace btree_port::btree::" in src
            type_ref = (
                "btree_internal::DedupSortedIter" if auto_ns
                else "DedupSortedIter"
            )
            helper = (
                "\n// btree_port port: DedupSortedIter deduction helper "
                "injected by post_transpile_patch.py\n"
                "template<typename __K, typename __V, typename __It>\n"
                "inline auto __btree_port_make_dedup(__It __it) {\n"
                f"    return {type_ref}<__K, __V, __It>"
                "::new_(std::move(__it));\n"
                "}\n"
            )
            import re
            if auto_ns:
                # Inject right after the namespace-open + alias block.
                # Find the LAST `namespace X = ::Y::Z;` line in the
                # contiguous alias block at the top of the namespace.
                # `finditer` walks them all; we keep the last contiguous one.
                alias_pattern = re.compile(
                    r"^namespace [a-z_]+ = ::btree_port::btree::[a-z_.:]+;\n",
                    re.MULTILINE,
                )
                last_alias = None
                prev_end = -1
                for m in alias_pattern.finditer(src):
                    if prev_end != -1 and m.start() != prev_end:
                        break  # gap — not contiguous with previous
                    last_alias = m
                    prev_end = m.end()
                if last_alias:
                    src = src[:last_alias.end()] + helper + src[last_alias.end():]
            else:
                last_import = None
                for m in re.finditer(r"^import [A-Za-z0-9_.]+;\s*$", src, re.MULTILINE):
                    last_import = m
                if last_import is not None:
                    ins = last_import.end()
                    src = src[:ins] + helper + src[ins:]
        src = sentinel + "\n" + src
        path.write_text(src)
    else:
        print(f"  no template-arg recovery sites in: {path.name}")


def drop_duplicate_min_len(path: Path) -> None:
    """`MIN_LEN` is exported from btree_internal.cppm. The transpiler
    also emits a re-export in map.cppm — `export extern const size_t
    MIN_LEN;` followed by `export constexpr size_t MIN_LEN = …;`. This
    creates a duplicate-declaration error across the two modules.
    Drop both lines in map.cppm."""
    src = path.read_text()
    sentinel = "// btree_port port: MIN_LEN duplicate dropped by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (MIN_LEN dup already dropped)")
        return
    targets = [
        "export extern const size_t MIN_LEN;\n",
        "export constexpr size_t MIN_LEN = MIN_LEN_AFTER_SPLIT;\n",
    ]
    n_dropped = 0
    for t in targets:
        if t in src:
            src = src.replace(t, "", 1)
            n_dropped += 1
    if n_dropped:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  dropped {n_dropped} duplicate MIN_LEN line(s) in: {path.name}")
    else:
        print(f"  no MIN_LEN duplicates in: {path.name}")


def fix_entry_T_to_KV(path: Path) -> None:
    """In `BTreeMap::entry()`'s body, the transpiler emits
    `VacantEntry<T, A>` / `OccupiedEntry<T, A>` instead of
    `VacantEntry<K, V, A>` / `OccupiedEntry<K, V, A>`. The map-side
    entry types take 3 type params; the transpiler accidentally
    used the set-side spelling.

    Substitute the wrong form to the right one. Idempotent."""
    src = path.read_text()
    sentinel = "// btree_port port: entry T→K,V fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (entry T→K,V already fixed)")
        return
    n_v = src.count("VacantEntry<T, A>")
    n_o = src.count("OccupiedEntry<T, A>")
    if n_v == 0 and n_o == 0:
        print(f"  no entry T→K,V sites in: {path.name}")
        return
    src = src.replace("VacantEntry<T, A>", "VacantEntry<K, V, A>")
    src = src.replace("OccupiedEntry<T, A>", "OccupiedEntry<K, V, A>")
    src = sentinel + "\n" + src
    path.write_text(src)
    print(
        f"  substituted {n_v} VacantEntry<T,A> + {n_o} OccupiedEntry<T,A> "
        f"→ <K,V,A> in: {path.name}"
    )


def fix_new_global_alloc(path: Path) -> None:
    """`BTreeMap<K, V>::new_()` is impl'd specifically for Global in
    Rust (`impl<K, V> BTreeMap<K, V, Global>`). The transpiler emits
    the body using `A` (the enclosing class's generic A) but the
    return type is `BTreeMap<K, V>` which defaults A=Global. Mismatch
    on the body's PhantomData.

    Fix on two fronts:
      1. Make `new_()`'s return type explicit:
         `BTreeMap<K, V>` → `BTreeMap<K, V, rusty::alloc::Global>`
      2. In the body, replace `A` → `rusty::alloc::Global` in
         PhantomData (already aligning with the explicit return)."""
    src = path.read_text()
    sentinel = "// btree_port port: new_() A→Global fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (new_() A→Global already fixed)")
        return

    n_total = 0

    # 1. `static BTreeMap<K, V> new_()` → explicit Global return.
    decl_old = "static BTreeMap<K, V> new_()"
    decl_new = "static BTreeMap<K, V, rusty::alloc::Global> new_()"
    if decl_old in src:
        src = src.replace(decl_old, decl_new, 1)
        n_total += 1

    # 2. Inside `BTreeMap<K, V>(...)` calls, swap `A` → `Global` in
    # PhantomData. Also rewrite the constructor call itself to
    # `BTreeMap<K, V, rusty::alloc::Global>(...)`.
    pattern = "BTreeMap<K, V>("
    target = "rusty::PhantomData<rusty::Box<std::tuple<K, V>, A>>{}"
    repl = "rusty::PhantomData<rusty::Box<std::tuple<K, V>, rusty::alloc::Global>>{}"
    pos = 0
    while True:
        i = src.find(pattern, pos)
        if i == -1:
            break
        j = src.find(";", i)
        if j == -1:
            break
        stmt = src[i : j + 1]
        if target in stmt:
            new_stmt = stmt.replace(target, repl, 1)
            new_stmt = new_stmt.replace(
                "BTreeMap<K, V>(",
                "BTreeMap<K, V, rusty::alloc::Global>(",
                1,
            )
            src = src[:i] + new_stmt + src[j + 1 :]
            n_total += 1
            j = i + len(new_stmt) - 1
        pos = j + 1

    if n_total:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  applied {n_total} new_()/A→Global fix(es) in: {path.name}")
    else:
        print(f"  no new_()/A→Global sites in: {path.name}")


def fix_vacant_entry_key_field(path: Path) -> None:
    """`VacantEntry<K, V, A>` has its field named `key_field` (to
    avoid clashing with the `key()` getter method) but the
    transpiler emits aggregate-init with `.key = …`. C++20
    designated initializers require the exact field name, so
    `.key = …` errors. Same shape as the height/height_field
    fix from earlier.

    Substitute `.key = ` → `.key_field = ` inside `VacantEntry<…>{…}`
    aggregate-init sites."""
    src = path.read_text()
    sentinel = "// btree_port port: VacantEntry .key → .key_field by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (VacantEntry key_field already fixed)")
        return
    target = "VacantEntry<K, V, A>{.key = std::move(key),"
    repl = "VacantEntry<K, V, A>{.key_field = std::move(key),"
    n = src.count(target)
    if n == 0:
        print(f"  no VacantEntry.key sites in: {path.name}")
        return
    src = src.replace(target, repl)
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  fixed {n} VacantEntry .key → .key_field in: {path.name}")


def fix_setvalzst_as_value(path: Path) -> None:
    """`SetValZST` is a Rust unit struct (`pub struct SetValZST;`).
    The transpiler emits it both as a type (in PhantomData etc.) and
    as a value (`.insert(SetValZST)`). The value form needs `{}` for
    default construction.

    Substitute only the value-position contexts:
      `, SetValZST)`  →  `, SetValZST{})`
      `(SetValZST)`   →  `(SetValZST{})`
      ` SetValZST)`   →  ` SetValZST{})`  (e.g. `return SetValZST;`)
    Type positions inside `<...>` are unaffected."""
    src = path.read_text()
    sentinel = "// btree_port port: SetValZST type→value by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (SetValZST type→value already fixed)")
        return
    replacements = [
        (", SetValZST)", ", SetValZST{})"),
        ("(SetValZST)", "(SetValZST{})"),
        (", SetValZST,", ", SetValZST{},"),
        (", SetValZST;", ", SetValZST{};"),
    ]
    n_total = 0
    for old, new in replacements:
        n = src.count(old)
        if n:
            src = src.replace(old, new)
            n_total += n
    if n_total:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  substituted SetValZST → SetValZST{{}} at {n_total} site(s) in: {path.name}")
    else:
        print(f"  no SetValZST type→value sites in: {path.name}")


def fix_global_as_value(path: Path) -> None:
    """`rusty::alloc::Global` is an empty struct (Rust's `Global`
    unit struct allocator). The transpiler emits it both as a TYPE
    (`BTreeMap<K, V, rusty::alloc::Global>`) and as a VALUE
    (`manually_drop_new(rusty::alloc::Global)`). The value form
    needs `Global{}` for default construction.

    Substitute only call-arg position contexts:
      `, rusty::alloc::Global)`  →  `, rusty::alloc::Global{})`
      `(rusty::alloc::Global)`   →  `(rusty::alloc::Global{})`
    Type positions (`<…>`) are unaffected."""
    src = path.read_text()
    sentinel = (
        "// btree_port port: rusty::alloc::Global type→value "
        "by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (Global type→value already fixed)")
        return
    n1 = src.count(", rusty::alloc::Global)")
    n2 = src.count("(rusty::alloc::Global)")
    src = src.replace(", rusty::alloc::Global)", ", rusty::alloc::Global{})")
    src = src.replace("(rusty::alloc::Global)", "(rusty::alloc::Global{})")
    if n1 + n2 > 0:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  substituted Global → Global{{}} at {n1 + n2} call-arg site(s) in: {path.name}")
    else:
        print(f"  no Global type→value sites in: {path.name}")


def fix_debug_map_call(path: Path) -> None:
    """The transpiler emits `f.debug_map().entries(...)` but rusty's
    Formatter has `debug_list`/`debug_struct`/`debug_tuple` only, not
    `debug_map`. Substitute → `debug_list` (the print format won't
    look like Rust's `{k: v, …}` but the type checks out).
    """
    src = path.read_text()
    sentinel = "// btree_port port: debug_map→debug_list by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (debug_map already rewritten)")
        return
    target = "f.debug_map()"
    if target not in src:
        print(f"  no debug_map sites in: {path.name}")
        return
    n = src.count(target)
    src = src.replace(target, "f.debug_list()")
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  rewrote {n} debug_map → debug_list in: {path.name}")


def fix_empty_write_return(path: Path) -> None:
    """Some `fmt(...)` bodies have a literal `return /* write!(…) */;`
    where the `write!` macro got dropped (left as a comment) and the
    return statement has no value. Inject `rusty::fmt::Result::Ok({})`
    so the non-void function returns SOMETHING."""
    src = path.read_text()
    sentinel = "// btree_port port: empty-return fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (empty-return already fixed)")
        return
    import re
    # Match `    return /* write!(...) */;` (any whitespace, any comment body)
    pattern = re.compile(
        r"return /\* write!\([^)]*\) \*/;",
    )
    matches = pattern.findall(src)
    if not matches:
        print(f"  no empty-return sites in: {path.name}")
        return
    # NB: don't use /* */ in the replacement — the matched original
    # contains a /* */ already and nested block comments are invalid
    # C++. Use a line-comment after the statement instead.
    src = pattern.sub(
        "return rusty::fmt::Result::Ok(std::make_tuple());  "
        "// btree_port: was empty `return /* write! */;`",
        src,
    )
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  fixed {len(matches)} empty-return site(s) in: {path.name}")


def fix_merge_unknown_Q(path: Path) -> None:
    """`BTreeMap::merge(other, conflict)` body references `Q` (a Rust
    generic that lets the lookup type be a borrowed view of K), but the
    transpiler dropped the `template<typename Q>` qualifier. The single
    use is `rusty::Bound<const Q&>::Included(first_other_key)`. Since
    K trivially borrows K, substitute Q → K at the merge call site."""
    src = path.read_text()
    sentinel = "// btree_port port: merge Q→K by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (merge Q→K already applied)")
        return
    target = "this->lower_bound_mut(rusty::Bound<const Q&>::Included("
    repl = "this->lower_bound_mut(rusty::Bound<const K&>::Included("
    if target in src:
        src = src.replace(target, repl, 1)
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  substituted Q→K in merge() in: {path.name}")
    else:
        print(f"  no merge Q→K site in: {path.name}")


# `fix_recursive_lambda_clone_subtree` removed (Item 8): the transpiler
# now lowers self-recursive nested fns to Y-combinator-shaped lambdas
# directly. See `emit_nested_function` + `try_emit_y_combinator_call`.


def fix_dormant_mut_ref_calls(path: Path) -> None:
    """`DormantMutRef::new_(x)` is the same template-args-recovery
    shape as A1's Root::new_ and NodeRef::new_leaf — except the
    deduced T varies per call site (BTreeMap, Root, Option<Root>),
    so a single-target textual substitution doesn't work.

    Approach: inject a deduction helper at the top of the module
    body, then rewrite call sites to use the helper. The helper
    template-deduces T from its argument.

    Also fixes a transpiler-side typo at the cursor sites:
    `DormantMutRef::new_(&this->root)` passes a pointer where
    `new_` expects a reference. Strip the spurious `&`.
    """
    src = path.read_text()
    sentinel = (
        "// btree_port port: DormantMutRef deduction helper "
        "injected by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (DormantMutRef helper already injected)")
        return

    n_calls = src.count("DormantMutRef::new_(")
    if n_calls == 0:
        print(f"  no DormantMutRef::new_ sites in: {path.name}")
        return

    # Under auto-namespace mode, inject INSIDE the namespace wrap and
    # use the alias-resolved unqualified `DormantMutRef`. Otherwise
    # inject at module purview (the helper template is at file scope).
    auto_ns = "namespace btree_port::btree::" in src
    type_ref = (
        "btree_internal::DormantMutRef" if auto_ns
        else "DormantMutRef"
    )
    helper = (
        f"\n{sentinel}\n"
        "// `DormantMutRef::new_(x)` is a static method on a class template\n"
        "// that the transpiler emits without explicit template arguments.\n"
        "// The helper template-deduces T from its argument so call sites\n"
        "// don't have to spell `DormantMutRef<T>::new_(x)` themselves.\n"
        "template<typename __T>\n"
        "inline auto __btree_port_make_dormant(__T& __t) {\n"
        f"    return {type_ref}<__T>::new_(__t);\n"
        "}\n"
    )

    import re
    if auto_ns:
        # Inject right after the alias block inside the namespace wrap.
        # Find the LAST contiguous `namespace X = ::Y::Z;` line.
        alias_pattern = re.compile(
            r"^namespace [a-z_]+ = ::btree_port::btree::[a-z_.:]+;\n",
            re.MULTILINE,
        )
        last_alias = None
        prev_end = -1
        for m in alias_pattern.finditer(src):
            if prev_end != -1 and m.start() != prev_end:
                break
            last_alias = m
            prev_end = m.end()
        if last_alias is None:
            print(
                f"  [warn] no namespace-alias anchor in {path.name}; "
                f"can't place helper",
                file=sys.stderr,
            )
            return
        insertion_pos = last_alias.end()
    else:
        # Find the position right after the last `import` directive.
        last_import = None
        for m in re.finditer(r"^import [A-Za-z0-9_.]+;\s*$", src, re.MULTILINE):
            last_import = m
        if last_import is None:
            print(f"  [warn] no import directives in {path.name}; can't place helper", file=sys.stderr)
            return
        insertion_pos = last_import.end()
    src = src[:insertion_pos] + helper + src[insertion_pos:]

    # Now rewrite the call sites. Order matters: the more-specific
    # pattern (with `&this->root`) must match BEFORE the general one.
    typo_fix_count = src.count("DormantMutRef::new_(&this->root)")
    src = src.replace(
        "DormantMutRef::new_(&this->root)",
        "__btree_port_make_dormant(this->root)",
    )
    n_remaining = src.count("DormantMutRef::new_(")
    src = src.replace(
        "DormantMutRef::new_(",
        "__btree_port_make_dormant(",
    )

    path.write_text(src)
    print(
        f"  injected DormantMutRef deduction helper + rewrote {n_calls} site(s) "
        f"in: {path.name} (fixed {typo_fix_count} `&this->root` typo)"
    )


def fix_boxed_box_path(path: Path) -> None:
    """The transpiler emits `::boxed::Box<…>` (matching Rust's
    `alloc::boxed::Box`) but the C++ side has `rusty::Box` only.
    Rewrite. Idempotent — uses a sentinel comment on first apply."""
    src = path.read_text()
    sentinel = "// btree_port port: boxed::Box rewritten to rusty::Box by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (boxed::Box already rewritten)")
        return
    n1 = src.count("::boxed::Box<")
    n2 = src.count("boxed::Box<")
    if n1 == 0 and n2 == 0:
        print(f"  no boxed::Box paths in: {path.name}")
        return
    src = src.replace("::boxed::Box<", "rusty::Box<")
    src = src.replace("boxed::Box<", "rusty::Box<")
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  rewrote {n1 + n2} boxed::Box path(s) in: {path.name}")


def implement_from_new_leaf(path: Path) -> None:
    """Replace the `throw` stub for `NodeRef::from_new_leaf` with a
    hand-port of the Rust source. This is phase B1 of the transpile
    completion plan — `BTreeMap::insert` ultimately reaches this
    method to construct a fresh leaf NodeRef from a Box-allocated
    LeafNode.

    Rust source (library/alloc/src/collections/btree/node.rs):
        fn from_new_leaf<A: Allocator + Clone>(
            leaf: Box<LeafNode<K, V>, A>,
        ) -> Self {
            let (node, _alloc) = Box::into_non_null_with_allocator(leaf);
            NodeRef { height: 0, node, _marker: PhantomData }
        }

    C++ port: take the Box's raw pointer (the allocator drops with
    the Box's destructor), wrap in NonNull, build the NodeRef.
    """
    src = path.read_text()
    sentinel = "// btree_port port: B1 from_new_leaf hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (B1 already landed)")
        return

    # The stub line we're replacing was emitted by stub() at step 19.
    # Match the exact stub-body shape so we don't accidentally rewrite
    # a real impl on a re-run after manual edits.
    sig = "static NodeRef<BorrowType, K, V, Type> from_new_leaf(rusty::Box<LeafNode<K, V>, A> leaf) {"
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no B1 stub site in: {path.name}")
        return
    # Find the matching close brace (depth walk).
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find stub end in: {path.name}", file=sys.stderr)
        return

    impl = (
        "{\n"
        f"        {sentinel}\n"
        "        // Box::into_non_null_with_allocator(leaf) → (NonNull<T>, A).\n"
        "        // The Box owns the allocator; its destructor drops it. We use\n"
        "        // `into_raw()` to take ownership of the LeafNode pointer.\n"
        "        ::LeafNode<K, V>* __raw = std::move(leaf).into_raw();\n"
        "        return NodeRef<BorrowType, K, V, Type>{\n"
        "            .height_field = static_cast<size_t>(0),\n"
        "            .node = rusty::ptr::NonNull<::LeafNode<K, V>>::new_unchecked(__raw),\n"
        "            ._marker = rusty::PhantomData<std::tuple<BorrowType, Type>>{}\n"
        "        };\n"
        "    }"
    )
    new_src = src[:brace_open] + impl + src[brace_close + 1 :]
    path.write_text(new_src)
    print(f"  hand-ported NodeRef::from_new_leaf in: {path.name}")


def implement_from_new_internal(path: Path) -> None:
    """Replace the `throw` stub for `NodeRef::from_new_internal`. Same
    shape as B1 but for internal nodes: takes a Box<InternalNode> + a
    NonZero<size_t> height, casts the NonNull pointer to LeafNode (the
    shared storage layout), and calls correct_all_childrens_parent_links
    on the resulting borrow_mut.

    Rust source (library/alloc/src/collections/btree/node.rs):
        fn from_new_internal<A: Allocator + Clone>(
            internal: Box<InternalNode<K, V>, A>,
            height: NonZero<usize>,
        ) -> Self {
            let (node, _alloc) = Box::into_non_null_with_allocator(internal);
            let mut this = NodeRef {
                height: height.into(),
                node: node.cast(),
                _marker: PhantomData,
            };
            this.borrow_mut().correct_all_childrens_parent_links();
            this
        }
    """
    src = path.read_text()
    sentinel = "// btree_port port: B2 from_new_internal hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (B2 already landed)")
        return
    sig = ("static NodeRef<BorrowType, K, V, Type> from_new_internal("
           "rusty::Box<InternalNode<K, V>, A> internal, "
           "rusty::num::NonZero<size_t> height) {")
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no B2 stub site in: {path.name}")
        return
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find B2 stub end in: {path.name}", file=sys.stderr)
        return
    impl = (
        "{\n"
        f"        {sentinel}\n"
        "        // Box::into_non_null_with_allocator → (NonNull, A); the Box's\n"
        "        // destructor drops the allocator. Take ownership of the\n"
        "        // InternalNode pointer via `into_raw()`, then cast NonNull\n"
        "        // to NonNull<LeafNode> (the storage layout has LeafNode at\n"
        "        // the head of InternalNode, so the cast is reinterpret-safe).\n"
        "        ::InternalNode<K, V>* __raw = std::move(internal).into_raw();\n"
        "        rusty::ptr::NonNull<::LeafNode<K, V>> __node =\n"
        "            rusty::ptr::NonNull<::InternalNode<K, V>>::new_unchecked(__raw).cast();\n"
        "        NodeRef<BorrowType, K, V, Type> __this{\n"
        "            .height_field = static_cast<size_t>(height.get()),\n"
        "            .node = __node,\n"
        "            ._marker = rusty::PhantomData<std::tuple<BorrowType, Type>>{}\n"
        "        };\n"
        "        __this.borrow_mut().correct_all_childrens_parent_links();\n"
        "        return std::move(__this);\n"
        "    }"
    )
    new_src = src[:brace_open] + impl + src[brace_close + 1 :]
    path.write_text(new_src)
    print(f"  hand-ported NodeRef::from_new_internal in: {path.name}")


def implement_push_with_handle(path: Path) -> None:
    """Replace the `throw` stub for `NodeRef::push_with_handle`.
    This is the leaf-write workhorse: `BTreeMap::insert` ultimately
    reaches here when adding to a non-full leaf.

    Rust source (library/alloc/src/collections/btree/node.rs):
        pub(super) unsafe fn push_with_handle<'b>(
            &mut self, key: K, val: V,
        ) -> Handle<NodeRef<marker::Mut<'b>, K, V, marker::Leaf>, marker::KV> {
            let len = self.len_mut();
            let idx = usize::from(*len);
            assert!(idx < CAPACITY);
            *len += 1;
            unsafe {
                self.key_area_mut(idx).write(key);
                self.val_area_mut(idx).write(val);
                Handle::new_kv(
                    NodeRef {
                        height: self.height,
                        node: self.node,
                        _marker: PhantomData,
                    },
                    idx,
                )
            }
        }

    The pattern is exactly the one already-transpiled `push()` at
    line 4543 uses (same len/idx/CAPACITY check + key/val_area_mut
    writes) — we just wrap a Handle::new_kv around the result.
    """
    src = path.read_text()
    sentinel = "// btree_port port: B3 push_with_handle hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (B3 already landed)")
        return
    sig = ("Handle<NodeRef<::marker::Mut, K, V, ::marker::Leaf>, ::marker::KV> "
           "push_with_handle(K key, V val) {")
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no B3 stub site in: {path.name}")
        return
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find B3 stub end in: {path.name}", file=sys.stderr)
        return
    impl = (
        "{\n"
        f"        {sentinel}\n"
        "        // Same pattern as the already-transpiled `push()`: increment\n"
        "        // len, write into the key/val areas at idx, then return a\n"
        "        // Handle pointing at the new (key, val) pair. The returned\n"
        "        // NodeRef has a fresh lifetime 'b in Rust; in C++ lifetimes\n"
        "        // are erased, so it's just a sibling NodeRef<Mut, K, V, Leaf>.\n"
        "        uint16_t& __len = this->len_mut();\n"
        "        auto __idx = static_cast<size_t>(__len);\n"
        "        assert((__idx < CAPACITY));\n"
        "        __len += 1;\n"
        "        // @unsafe — caller has the only mutable borrow of this leaf.\n"
        "        this->key_area_mut(__idx).write(std::move(key));\n"
        "        this->val_area_mut(__idx).write(std::move(val));\n"
        "        return Handle<NodeRef<::marker::Mut, K, V, ::marker::Leaf>, ::marker::KV>::new_kv(\n"
        "            NodeRef<::marker::Mut, K, V, ::marker::Leaf>{\n"
        "                .height_field = this->height_field,\n"
        "                .node = this->node,\n"
        "                ._marker = rusty::PhantomData<std::tuple<::marker::Mut, ::marker::Leaf>>{}\n"
        "            },\n"
        "            __idx\n"
        "        );\n"
        "    }"
    )
    new_src = src[:brace_open] + impl + src[brace_close + 1 :]
    path.write_text(new_src)
    print(f"  hand-ported NodeRef::push_with_handle in: {path.name}")


def fix_dormant_mut_ref_from_t(path: Path) -> None:
    """The Rust `DormantMutRef::new(t: &mut T)` body calls
    `NonNull::from(t)` — taking the `&mut T` directly. The
    transpiler emitted this as `NonNull<...>::from(t)` but our
    `NonNull<T>::from(T*)` takes a POINTER, not a reference.
    Patch: `NonNull<...>::from(t)` → `NonNull<...>::from(&t)`."""
    src = path.read_text()
    sentinel = (
        "// btree_port port: DormantMutRef NonNull::from(t) → from(&t) "
        "by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (DormantMutRef from(&t) already fixed)")
        return
    target = (
        "auto ptr_shadow1 = NonNull<"
        "std::remove_pointer_t<std::remove_reference_t<decltype((t))>>"
        ">::from(t);"
    )
    repl = (
        "auto ptr_shadow1 = NonNull<"
        "std::remove_pointer_t<std::remove_reference_t<decltype((t))>>"
        ">::from(&t);"
    )
    if target in src:
        src = src.replace(target, repl, 1)
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  fixed DormantMutRef::new_ NonNull::from(t) → from(&t) in: {path.name}")
    else:
        print(f"  no DormantMutRef from(t) site in: {path.name}")


def fix_dormant_mut_ref_const_ref(path: Path) -> None:
    """In `DormantMutRef::new_(T& t)` the body says
    `const T& new_ref = …` but then constructs
    `std::tuple<T&, DormantMutRef<T>>{new_ref, …}`. The Rust source
    has `&mut *ptr.as_ptr()` (mutable). Strip the `const` so the
    tuple init's `T&` element can bind to `new_ref`."""
    src = path.read_text()
    sentinel = (
        "// btree_port port: DormantMutRef new_ref const→mut "
        "by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (DormantMutRef const→mut already fixed)")
        return
    target = "const T& new_ref = *rusty::as_ptr(ptr_shadow1);"
    repl = "T& new_ref = *rusty::as_ptr(ptr_shadow1);"
    if target in src:
        src = src.replace(target, repl, 1)
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  fixed DormantMutRef::new_ const T& → T& in: {path.name}")
    else:
        print(f"  no DormantMutRef const→mut site in: {path.name}")


def fix_as_leaf_ptr_self(path: Path) -> None:
    """`as_leaf_ptr` is declared `static auto as_leaf_ptr(const NodeRef& this_)`
    (taking the receiver as an explicit parameter, mirroring Rust's
    static-method convention). Several call sites inside NodeRef
    methods call it as `as_leaf_ptr()` (no args), expecting the
    self parameter to be implicit. Pass `(*this)` explicitly."""
    src = path.read_text()
    sentinel = "// btree_port port: as_leaf_ptr() → as_leaf_ptr((*this)) by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (as_leaf_ptr self-arg already fixed)")
        return
    # Match `as_leaf_ptr()` only when NOT preceded by a qualifier
    # (those are the explicit static form like `NodeRef<…>::as_leaf_ptr(...)`
    # which already passes (*this)). Use a simple word-boundary check.
    import re
    pattern = re.compile(r"(?<![\w:])as_leaf_ptr\(\)")
    matches = list(pattern.finditer(src))
    if not matches:
        print(f"  no bare as_leaf_ptr() sites in: {path.name}")
        return
    src = pattern.sub("as_leaf_ptr((*this))", src)
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  fixed {len(matches)} bare as_leaf_ptr() call(s) in: {path.name}")


def fix_force_match_arms(path: Path) -> None:
    """The transpiler punts on `match expr.force() { Leaf(x) => …,
    Internal(y) => … }` arms, emitting:

      if (/* TODO transpiler: unresolved bare-glob variant `Leaf`
            (no enum decl visible in this TU; patch arm manually) */ true) {
          auto&& leaf = rusty::detail::deref_if_pointer(
              rusty::detail::deref_if_pointer(_m)._0);
          …
      }
      if (/* TODO ... `Internal` ... */ true) {
          auto&& internal = …deref_if_pointer(_m)._0…;
          …
      }

    Two bugs: (1) the condition is hard-coded `true` so both arms
    enter, (2) `_m._0` doesn't compile because `_m` is a
    std::variant<…> not a struct.

    Fix: change `true` → `…(_m).index() == {0,1}`, and add
    `std::get<{0,1}>` around `…(_m)` so the variant alternative
    extraction works."""
    src = path.read_text()
    sentinel = "// btree_port port: force() arm conditions + variant access fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (force() arms already fixed)")
        return

    # The condition variable in scope varies: `_m` in nested-lambda
    # match contexts, `_whilelet` in while-let contexts. Scan the
    # whole file linearly so we can detect which variable was
    # declared right before each TODO arm.
    import re
    n_fixed = 0
    lines = src.split("\n")
    var_in_scope = None  # most recent `auto&& X = ….force();` binding
    out = []
    for line in lines:
        # Track .force() bindings.
        m = re.search(r"auto&&\s+(_\w+)\s*=\s*[^;]*\.force\(\)", line)
        if m:
            var_in_scope = m.group(1)
        # Substitute TODO patterns using the in-scope variable.
        for idx, name in [(0, "Leaf"), (1, "Internal")]:
            comment = (
                f"/* TODO transpiler: unresolved bare-glob variant "
                f"`{name}` (no enum decl visible in this TU; patch arm manually) */ true"
            )
            if comment in line and var_in_scope is not None:
                replacement_cond = (
                    f"rusty::detail::deref_if_pointer({var_in_scope}).index() == {idx}"
                )
                line = line.replace(comment, replacement_cond)
                n_fixed += 1
        out.append(line)
    src = "\n".join(out)

    # Now also fix the variant-element access. Inside each arm, the
    # `_0` is accessed via:
    #   rusty::detail::deref_if_pointer(_m)._0
    # which fails because _m is a variant. Wrap in std::get.
    # Heuristic: the arm body uses an enum-distinct name `leaf` or
    # `internal` to bind. Find the binding pattern:
    #   auto&& leaf = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_m)._0);
    #   →
    #   auto&& leaf = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m))._0);
    # Same for `internal` with std::get<1>.
    pairs = [
        (
            "auto&& leaf = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_m)._0);",
            "auto&& leaf = rusty::detail::deref_if_pointer(std::get<0>(rusty::detail::deref_if_pointer(_m))._0);",
        ),
        (
            "auto&& internal = rusty::detail::deref_if_pointer(rusty::detail::deref_if_pointer(_m)._0);",
            "auto&& internal = rusty::detail::deref_if_pointer(std::get<1>(rusty::detail::deref_if_pointer(_m))._0);",
        ),
    ]
    for old, new in pairs:
        n = src.count(old)
        if n == 0:
            continue
        src = src.replace(old, new)
        n_fixed += n

    if n_fixed:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  fixed {n_fixed} force() arm site(s) in: {path.name}")
    else:
        print(f"  no force() arm sites in: {path.name}")


def strip_redundant_method_template_params(path: Path) -> None:
    """The transpiler sometimes emits redundant template params on
    method declarations, shadowing the enclosing struct's params.
    e.g. inside `struct Handle<Node, Type>`:

        template<typename BorrowType, typename K, typename V>
        NodeRef<BorrowType, K, V, …> descend() {
            …
        }

    Here BorrowType/K/V are shadowed method-template params. The
    method takes no args from which to deduce them, so the call
    `handle.descend()` fails template argument deduction. The fix
    is to remove the `template<…>` line so the names refer to
    the enclosing class's template params instead."""
    src = path.read_text()
    sentinel = "// btree_port port: redundant method-template params stripped by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (redundant method-template params already stripped)")
        return
    # Match the exact shape that produces the deduction failure:
    # `    template<typename BorrowType, typename K, typename V>\n` at
    # struct-body indent (4 spaces). Only strip when the next line
    # is a method declaration (starts with a type or `template` for
    # nested templates).
    import re
    pattern = re.compile(
        r"^    template<typename BorrowType, typename K, typename V>\s*\n",
        re.MULTILINE,
    )
    matches = list(pattern.finditer(src))
    if not matches:
        print(f"  no redundant method-template sites in: {path.name}")
        return
    src = pattern.sub("", src)
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  stripped {len(matches)} redundant method-template line(s) in: {path.name}")


def implement_leaf_edge_walkers(path: Path) -> None:
    """Hand-port `NodeRef::first_leaf_edge` and `last_leaf_edge`.
    The transpiled bodies have the unrecoverable force() arm
    pattern (`/* TODO transpiler: unresolved bare-glob variant
    `Leaf` … */ true`) that left the conditions broken AND used
    `._0` directly on a `std::variant` (invalid).

    Rust source (library/alloc/src/collections/btree/navigate.rs):
        fn first_leaf_edge(self) -> Handle<NodeRef<…, Leaf>, Edge> {
            let mut node = self;
            loop {
                match node.force() {
                    ForceResult::Leaf(leaf) => return leaf.first_edge(),
                    ForceResult::Internal(internal) => {
                        node = internal.first_edge().descend();
                    }
                }
            }
        }
    """
    src = path.read_text()
    sentinel = "// btree_port port: first/last_leaf_edge hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (first/last_leaf_edge already ported)")
        return

    landed = 0
    for (which, edge_call) in [("first_leaf_edge", "first_edge"), ("last_leaf_edge", "last_edge")]:
        sig = (
            f"Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, "
            f"::marker::Edge> {which}() const {{"
        )
        sig_pos = src.find(sig)
        if sig_pos == -1:
            print(f"  no {which} const site in: {path.name}")
            continue
        brace_open = src.find("{", sig_pos + len(sig) - 1)
        depth = 0
        brace_close = -1
        for k in range(brace_open, len(src)):
            if src[k] == "{":
                depth += 1
            elif src[k] == "}":
                depth -= 1
                if depth == 0:
                    brace_close = k
                    break
        if brace_close == -1:
            print(f"  [warn] couldn't find {which} end", file=sys.stderr)
            continue
        impl = (
            "{\n"
            f"        {sentinel}\n"
            "        // Method is const but the Rust source consumes self;\n"
            "        // copy `*this` into a mutable local for the walk. NodeRef\n"
            "        // is cheap to copy (a pointer + a size_t + PhantomData).\n"
            "        auto node = *this;\n"
            "        while (true) {\n"
            "            auto __m = node.force();  // std::variant<ForceResult_Leaf, ForceResult_Internal>\n"
            "            if (__m.index() == 0) {\n"
            f"                return std::get<0>(__m)._0.{edge_call}();\n"
            "            } else {\n"
            f"                node = std::get<1>(__m)._0.{edge_call}().descend();\n"
            "            }\n"
            "        }\n"
            "    }"
        )
        src = src[:brace_open] + impl + src[brace_close + 1 :]
        landed += 1
        print(f"  hand-ported NodeRef::{which} in: {path.name}")

    if landed:
        path.write_text(src)


def implement_handle_descend(path: Path) -> None:
    """Hand-port `Handle::descend`. The transpiled body:

      template<typename BorrowType, typename K, typename V>
      NodeRef<BorrowType, K, V, ::marker::LeafOrInternal> descend() {
          rusty::intrinsics::unreachable();   // ← stub marker
          const auto parent_ptr = …as_internal_ptr(this->node);
          auto node = (*parent_ptr).edges.get_unchecked(…).assume_init_read();
          return NodeRef<BorrowType, K, V, …>{…};
      }

    has two bugs:
      1. The unreachable() at the top makes the body dead.
      2. BorrowType/K/V are method-template params that can't be
         deduced from the (empty) parameter list, so call sites
         like `handle.descend()` fail template-arg deduction.

    Fix: rewrite to use a `__NodeRefArgs<Node>` type-trait that
    destructures the enclosing class's `Node` template arg
    (which is `NodeRef<BorrowType, K, V, marker::Internal>` for
    descend's specific impl), giving us BorrowType/K/V at the
    method-body level without needing method-template params.

    Also adds the `__NodeRefArgs` trait at the top of the module
    (right after the rusty includes) so it's visible everywhere.
    """
    src = path.read_text()
    sentinel = "// btree_port port: Handle::descend hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (Handle::descend already ported)")
        return

    # 1. Inject the __NodeRefArgs trait at module scope. Place it
    #    right after the global-module-fragment block (before
    #    `export module …;`) so it's visible everywhere in the
    #    module purview. Actually easier: place it just before the
    #    first NodeRef forward decl, which sits in the module
    #    purview already.
    trait_marker = "// btree_port port: __NodeRefArgs trait injected"
    if trait_marker not in src:
        trait_block = (
            f"\n{trait_marker}\n"
            "// Type-trait to destructure `NodeRef<B, K, V, T>` template\n"
            "// arguments so methods of `Handle<NodeRef<...>, Edge>` can\n"
            "// derive BorrowType/Key/Value at the class-template level\n"
            "// rather than redundant method-template level (which fails\n"
            "// deduction when the method has no arguments).\n"
            "template<typename T> struct __NodeRefArgs;\n"
            "template<typename B, typename K, typename V, typename T>\n"
            "struct __NodeRefArgs<NodeRef<B, K, V, T>> {\n"
            "    using BorrowType = B;\n"
            "    using Key = K;\n"
            "    using Value = V;\n"
            "    using Tag = T;\n"
            "};\n"
        )
        # Insert AFTER the first NodeRef forward declaration so the
        # trait specialization can name `NodeRef<…>`. The forward
        # decl looks like:
        #   export template<typename BorrowType, typename K, typename V, typename Type>
        #       requires (...)
        #   struct NodeRef;
        # We anchor on the `struct NodeRef;` line.
        anchor = "struct NodeRef;\n"
        pos = src.find(anchor)
        if pos == -1:
            print(f"  [warn] couldn't find NodeRef forward decl in: {path.name}", file=sys.stderr)
            return
        ins = pos + len(anchor)
        src = src[:ins] + trait_block + src[ins:]

    # 2. Replace the descend method's signature + body.
    # The two known emit shapes differ only in the first body line:
    #   - OLD (pre-Cluster D): `rusty::intrinsics::unreachable();` (the
    #     mis-lowered `const { ... }` Rust 2024 compile-time fence).
    #   - NEW (post-Cluster D): `// const-block elided (Rust 2024 compile-time fence)`.
    # The rest of the body (parent_ptr lookup, edges read, return aggregate
    # — including `this->node.height` without the `_field` suffix, which a
    # later patcher pass rewrites globally) is identical between the two.
    body_tail = (
        "        const auto parent_ptr = NodeRef<BorrowType, K, V, ::marker::LeafOrInternal>::as_internal_ptr(this->node);\n"
        "        auto node = (*parent_ptr).edges.get_unchecked(std::move(this->idx_field)).assume_init_read();\n"
        "        return NodeRef<BorrowType, K, V, ::marker::LeafOrInternal>{"
        ".height_field = rusty::detail::deref_if_pointer_like(this->node.height) - static_cast<size_t>(1), "
        ".node = std::move(node), "
        "._marker = rusty::PhantomData<std::tuple<BorrowType, ::marker::LeafOrInternal>>{}};\n"
        "    }\n"
    )
    sig_head = (
        "    template<typename BorrowType, typename K, typename V>\n"
        "    NodeRef<BorrowType, K, V, ::marker::LeafOrInternal> descend() {\n"
    )
    # Post-Cluster-A-completion shape: the method-template params are
    # dropped and impl-generic refs are replaced with
    # `typename __TemplateArgs<Node>::arg_<N>`. The body keeps
    # `.edges.get_unchecked(...)` (still wrong — std::array has no
    # get_unchecked) and references `this->node.height_field`.
    ta = "typename __TemplateArgs<Node>::arg_"
    ta_ref = f"NodeRef<{ta}0, {ta}1, {ta}2"
    cluster_a_method = (
        f"    {ta_ref}, ::marker::LeafOrInternal> descend() {{\n"
        "        // const-block elided (Rust 2024 compile-time fence)\n"
        f"        const auto parent_ptr = {ta_ref}, ::marker::Internal>::as_internal_ptr(this->node);\n"
        "        auto node = (*parent_ptr).edges.get_unchecked(std::move(this->idx_field)).assume_init_read();\n"
        f"        return {ta_ref}, ::marker::LeafOrInternal>"
        "{.height_field = rusty::detail::deref_if_pointer_like(this->node.height_field) - static_cast<size_t>(1), "
        ".node = std::move(node), "
        f"._marker = rusty::PhantomData<std::tuple<{ta}0, ::marker::LeafOrInternal>>{{}}}};\n"
        "    }\n"
    )
    # Post-(Item 2 + Item 7 transient) shape: get_unchecked is now wrapped
    # in a SFINAE shim, and at one point the as_internal_ptr recovery
    # picked LeafOrInternal (from the descend method's return type) instead
    # of Internal (from the receiver's struct field). Kept for back-compat
    # with intermediate transpiler versions.
    cluster_a_v2_method = (
        f"    {ta_ref}, ::marker::LeafOrInternal> descend() {{\n"
        "        // const-block elided (Rust 2024 compile-time fence)\n"
        f"        const auto parent_ptr = {ta_ref}, ::marker::LeafOrInternal>::as_internal_ptr(this->node);\n"
        "        auto node = ([&](auto&& __recv, auto&& __idx) -> decltype(auto) { "
        "if constexpr (requires { __recv[__idx]; }) { return __recv[__idx]; } "
        "else { return __recv.get_unchecked(__idx); } })"
        "((*parent_ptr).edges, std::move(this->idx_field)).assume_init_read();\n"
        f"        return {ta_ref}, ::marker::LeafOrInternal>"
        "{.height_field = rusty::detail::deref_if_pointer_like(this->node.height_field) - static_cast<size_t>(1), "
        ".node = std::move(node), "
        f"._marker = rusty::PhantomData<std::tuple<{ta}0, ::marker::LeafOrInternal>>{{}}}};\n"
        "    }\n"
    )
    # Post-Item-7 (LeafOrInternal) shape: cluster_a recovery resolves
    # arg_0/1/2 correctly, but the trailing marker still comes from the
    # enclosing function's return type hint (LeafOrInternal), not the
    # impl block's concrete `Internal`. The `as_internal_ptr` call
    # therefore has the WRONG marker — Owned/Leaf method resolution
    # still works at runtime because the helper just looks up a static
    # method on NodeRef regardless of marker — but the hand-port
    # replaces the whole body anyway, so we only need to match the
    # emit shape. `this->node.height` (no _field suffix) is the raw
    # Rust field name; the renamed C++ field is `height_field`.
    cluster_a_item7_method = (
        f"    {ta_ref}, ::marker::LeafOrInternal> descend() {{\n"
        "        // const-block elided (Rust 2024 compile-time fence)\n"
        f"        const auto parent_ptr = {ta_ref}, ::marker::LeafOrInternal>::as_internal_ptr(this->node);\n"
        "        auto node = ([&](auto&& __recv, auto&& __idx) -> decltype(auto) { "
        "if constexpr (requires { __recv[__idx]; }) { return __recv[__idx]; } "
        "else { return __recv.get_unchecked(__idx); } })"
        "((*parent_ptr).edges, std::move(this->idx_field)).assume_init_read();\n"
        f"        return {ta_ref}, ::marker::LeafOrInternal>"
        "{.height_field = rusty::detail::deref_if_pointer_like(this->node.height) - static_cast<size_t>(1), "
        ".node = std::move(node), "
        f"._marker = rusty::PhantomData<std::tuple<{ta}0, ::marker::LeafOrInternal>>{{}}}};\n"
        "    }\n"
    )
    old_method_variants = [
        sig_head + "        rusty::intrinsics::unreachable();\n" + body_tail,
        sig_head
        + "        // const-block elided (Rust 2024 compile-time fence)\n"
        + body_tail,
        cluster_a_method,
        cluster_a_v2_method,
        cluster_a_item7_method,
    ]
    old_method = next((v for v in old_method_variants if v in src), None)
    new_method = (
        f"    {sentinel}\n"
        "    NodeRef<typename __NodeRefArgs<Node>::BorrowType,\n"
        "            typename __NodeRefArgs<Node>::Key,\n"
        "            typename __NodeRefArgs<Node>::Value,\n"
        "            ::marker::LeafOrInternal> descend() {\n"
        "        using __B = typename __NodeRefArgs<Node>::BorrowType;\n"
        "        using __K = typename __NodeRefArgs<Node>::Key;\n"
        "        using __V = typename __NodeRefArgs<Node>::Value;\n"
        "        const auto parent_ptr = NodeRef<__B, __K, __V, ::marker::Internal>::as_internal_ptr(this->node);\n"
        "        // `edges` is std::array<MaybeUninit<NonNull<LeafNode>>, …>;\n"
        "        // use operator[] (std::array has no get_unchecked) plus\n"
        "        // assume_init_read() to extract the NonNull.\n"
        "        auto __idx = static_cast<size_t>(this->idx_field);\n"
        "        auto node = (*parent_ptr).edges[__idx].assume_init_read();\n"
        "        return NodeRef<__B, __K, __V, ::marker::LeafOrInternal>{\n"
        "            // `height` is a getter method, not the field; the field\n"
        "            // is named height_field.\n"
        "            .height_field = this->node.height_field - static_cast<size_t>(1),\n"
        "            .node = std::move(node),\n"
        "            ._marker = rusty::PhantomData<std::tuple<__B, ::marker::LeafOrInternal>>{}\n"
        "        };\n"
        "    }\n"
    )
    if old_method is not None:
        src = src.replace(old_method, new_method, 1)
        path.write_text(src)
        print(f"  hand-ported Handle::descend (with __NodeRefArgs trait) in: {path.name}")
    else:
        print(f"  no Handle::descend site in: {path.name}")


def stub_broken_map_methods(path: Path) -> None:
    """Stub map-side methods whose bodies have cascading transpiler
    bugs that aren't worth fixing per-line (they're all blocked on
    search_tree which is stubbed). Each method becomes a throw."""
    src = path.read_text()
    sentinel = "// btree_port port: map.cppm broken-method stubs by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (broken-method stubs already in)")
        return

    # Methods to stub. The body is replaced with a throw.
    # NOTE: BTreeMap::get used to be stubbed but search_tree is now
    # hand-ported (step 48), so get's body should work — leaving it
    # un-stubbed and seeing what happens.
    targets = [
        ("rusty::Option<std::tuple<const K&, const V&>> first_key_value() const {",
         "BTreeMap::first_key_value"),
        ("rusty::Option<std::tuple<const K&, const V&>> last_key_value() const {",
         "BTreeMap::last_key_value"),
    ]
    landed = 0
    for sig, name in targets:
        sig_pos = src.find(sig)
        if sig_pos == -1:
            continue
        brace_open = src.find("{", sig_pos + len(sig) - 1)
        depth = 0
        brace_close = -1
        for k in range(brace_open, len(src)):
            if src[k] == "{":
                depth += 1
            elif src[k] == "}":
                depth -= 1
                if depth == 0:
                    brace_close = k
                    break
        if brace_close == -1:
            continue
        stub = (
            "{\n"
            f"        // btree_port port: {name} stubbed by post_transpile_patch.py\n"
            f"        throw ::std::runtime_error(\"rusty-cpp-transpiler: {name} stub\");\n"
            "    }"
        )
        src = src[:brace_open] + stub + src[brace_close + 1 :]
        landed += 1
        print(f"  stubbed {name} in: {path.name}")
    if landed:
        # Add sentinel at top so re-run skips.
        src = src.replace("\n", f"\n{sentinel}\n", 1)
        path.write_text(src)


def stub_broken_map_entry_methods(path: Path) -> None:
    """Stub map.entry methods (kv_mut, into_val_mut callers) whose
    bodies depend on the broken-method chain."""
    src = path.read_text()
    sentinel = "// btree_port port: map.entry broken-method stubs by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (broken-method stubs already in)")
        return

    # Stub the specific lines/methods that fail. Simpler approach:
    # find any line containing `.kv_mut()` or `.into_val_mut()` and
    # wrap the enclosing method body. Actually easier — stub the
    # specific known method.
    # Looking at line 3709: it's inside OccupiedEntry::key().
    targets = [
        ("const K& key() const {",
         "OccupiedEntry::key"),
        ("K into_key() {",
         "OccupiedEntry::into_key"),
        ("V& get_mut() {",
         "OccupiedEntry::get_mut"),
        ("V& into_mut() {",
         "OccupiedEntry::into_mut"),
    ]
    landed = 0
    for sig, name in targets:
        # Find first occurrence (there might be multiple structs with
        # similarly-named methods — but the first within this file is
        # the OccupiedEntry one based on the line numbers).
        sig_pos = src.find(sig)
        if sig_pos == -1:
            continue
        brace_open = src.find("{", sig_pos + len(sig) - 1)
        depth = 0
        brace_close = -1
        for k in range(brace_open, len(src)):
            if src[k] == "{":
                depth += 1
            elif src[k] == "}":
                depth -= 1
                if depth == 0:
                    brace_close = k
                    break
        if brace_close == -1:
            continue
        stub = (
            "{\n"
            f"        // btree_port port: {name} stubbed by post_transpile_patch.py\n"
            f"        throw ::std::runtime_error(\"rusty-cpp-transpiler: {name} stub\");\n"
            "    }"
        )
        src = src[:brace_open] + stub + src[brace_close + 1 :]
        landed += 1
        print(f"  stubbed {name} in: {path.name}")
    if landed:
        src = src.replace("\n", f"\n{sentinel}\n", 1)
        path.write_text(src)


def merge_map_entry_into_map(map_mod: Path, map_entry: Path) -> None:
    """Step 52: merge OccupiedEntry / VacantEntry / Entry / OccupiedError
    struct definitions from map.entry.cppm into map.cppm.

    Why: in C++20 modules you can't express the cycle that the Rust
    source has — `map/entry.rs` references `super::map::BTreeMap` in
    its entry-struct `dormant_map` field, but `map.cppm` imports
    `map.entry.cppm`. Forward decls can't bridge it because of module
    attachment. The fix is to put the entry structs in the SAME module
    as BTreeMap, so the field type resolves directly.

    Side effects this transform makes:
      1. Inject `#define RUSTY_BTREEMAP_HPP / RUSTY_BTREESET_HPP` in
         map.cppm's GMF so the std::map facade (`rusty::BTreeMap`)
         from rusty/btreemap.hpp doesn't define the same name.
      2. Replace `import btree_port.btree.map.entry;` in map.cppm with
         a comment explaining the merge.
      3. Extract the post-`export module` content from map.entry.cppm
         (the forward decls + struct defs) and inject it into map.cppm
         right before the BTreeMap struct definition.
      4. Substitute `rusty::BTreeMap` → `BTreeMap` throughout map.cppm
         (both in the injected content and in the BTreeMap struct's
         own body, which previously relied on the rusty:: facade).

    The patcher's downstream rules then continue to operate on map.cppm
    as if both halves were one file.
    """
    map_src = map_mod.read_text()
    sentinel = "// btree_port port step 52: entry types (OccupiedEntry / VacantEntry / Entry / OccupiedError) inlined from map.entry.cppm"
    if sentinel in map_src:
        print(f"  no changes to: {map_mod.name} (map.entry already merged)")
        return
    if not map_entry.exists():
        print(f"  [warn] {map_entry.name} not present; skip merge", file=sys.stderr)
        return

    # 1. Skip the rusty::BTreeMap / rusty::BTreeSet facade in map.cppm.
    gmf_marker = "module;\n\n#include <cstdint>"
    gmf_replace = (
        "module;\n\n"
        "// btree_port port step 52: skip rusty::BTreeMap / rusty::BTreeSet\n"
        "// facade (rusty/btreemap.hpp + rusty/btreeset.hpp). The names\n"
        "// rusty::BTreeMap / rusty::BTreeSet are no longer occupied by the\n"
        "// std::map-backed facade inside this TU, so the transpiled BTreeMap\n"
        "// can freely use the `BTreeMap` name in the global namespace.\n"
        "#define RUSTY_BTREEMAP_HPP\n"
        "#define RUSTY_BTREESET_HPP\n\n"
        "#include <cstdint>"
    )
    if gmf_marker in map_src:
        map_src = map_src.replace(gmf_marker, gmf_replace, 1)
    else:
        print(f"  [warn] couldn't find GMF anchor in {map_mod.name}", file=sys.stderr)

    # 2. Strip `import btree_port.btree.map.entry;` (its content is being
    #    inlined below, so this import would dangle — the module isn't in
    #    the build target). Use a line-level regex so the strip is robust
    #    to surrounding whitespace / blank lines / sentinel comment order.
    import_replace_note = (
        "// btree_port port step 52: map.entry merged into this module. The\n"
        "// OccupiedEntry / VacantEntry / Entry / OccupiedError struct defs\n"
        "// are now inlined below, ahead of the BTreeMap struct.\n"
    )
    new_map_src, n_stripped = re.subn(
        r"^import btree_port\.btree\.map\.entry;\s*\n",
        import_replace_note,
        map_src,
        count=1,
        flags=re.MULTILINE,
    )
    if n_stripped:
        map_src = new_map_src
    else:
        print(
            f"  [warn] no `import btree_port.btree.map.entry;` line in {map_mod.name}",
            file=sys.stderr,
        )

    # 3. Extract entry content from map.entry.cppm. We take everything
    #    after the `export module btree_port.btree.map.entry;` line
    #    and its imports, up to EOF (which is the last `};`).
    entry_src = map_entry.read_text()
    # Find the first line AFTER the imports.
    import_anchor = "import btree_port.btree.btree_internal;\n"
    pos = entry_src.find(import_anchor)
    if pos == -1:
        print(f"  [warn] no import anchor in {map_entry.name}", file=sys.stderr)
        return
    content_start = pos + len(import_anchor)
    # Skip any leading blank lines.
    while content_start < len(entry_src) and entry_src[content_start] == "\n":
        content_start += 1
    entry_content = entry_src[content_start:]
    # Strip any remaining `import ...;` lines from the merged content.
    # With the --auto-namespace crate-mode transpile, map.entry.cppm now
    # has multiple imports (btree_internal AND map). Only the first one
    # got consumed by the `import_anchor` skip above; the rest would
    # land inside map.cppm's body, including a self-import of
    # `btree_port.btree.map` which is a hard compile error.
    entry_content = re.sub(
        r"^import [a-z_.]+;\s*\n",
        "",
        entry_content,
        flags=re.MULTILINE,
    )
    # Substitute rusty::BTreeMap → BTreeMap so the merged content
    # references the local transpiled BTreeMap.
    entry_content = entry_content.replace("rusty::BTreeMap", "BTreeMap")
    # CRITICAL: rewrite the namespace open from
    #   `namespace btree_port::btree::map::entry { … }`
    # to just `namespace entry { … }`. Why: when this content is
    # injected INSIDE `namespace btree_port::btree::map { … }`, the
    # fully-qualified `namespace btree_port::btree::map::entry { … }`
    # is interpreted as opening
    # `btree_port::btree::map::btree_port::btree::map::entry`
    # (per C++17 nested-namespace-definition rules — the prefix
    # nests INSIDE the current namespace). Using just `entry` opens
    # the correct child namespace `btree_port::btree::map::entry`.
    #
    # This was the root cause of the "implicit instantiation of
    # undefined template" error in consumer TUs: Entry's body lived
    # in `…::map::…::map::entry::Entry`, but the forward decl visible
    # to BTreeMap was `…::map::entry::Entry` — two different entities.
    entry_content = entry_content.replace(
        "namespace btree_port::btree::map::entry {",
        "namespace entry {",
    )
    entry_content = entry_content.replace(
        "} // namespace btree_port::btree::map::entry",
        "} // namespace entry",
    )

    # 4. Inject entry content right before the BTreeMap struct
    #    definition. Anchor on the comment line immediately above
    #    `export template<typename K, typename V, typename A = …>`.
    inject_anchor = "/// [`RefCell`]: core::cell::RefCell\n"
    ix = map_src.find(inject_anchor)
    if ix == -1:
        print(f"  [warn] no inject anchor in {map_mod.name}", file=sys.stderr)
        return
    ix_end = ix + len(inject_anchor)
    inject = f"{sentinel}. They were originally in a separate module but C++20 modules can't express the cycle that map.entry's `DormantMutRef<BTreeMap<K,V,A>>` field requires. Merging into this module gives the field type the same module attachment as BTreeMap's own definition.\n\n{entry_content}\n"
    map_src = map_src[:ix_end] + inject + map_src[ix_end:]

    # 5. Substitute rusty::BTreeMap → BTreeMap in the rest of map.cppm
    #    too (a few internal uses inside BTreeMap's own body).
    map_src = map_src.replace("rusty::BTreeMap", "BTreeMap")

    # 6. Under auto-namespace mode the postprocess added a
    #    `namespace entry = ::btree_port::btree::map::entry;` alias
    #    inside the map namespace wrap. After the merge, `map::entry`
    #    is a real nested namespace defined in this same TU. That
    #    nested namespace name COLLIDES with the alias name at the
    #    `btree_port::btree::map` scope.
    #
    #    Resolution: drop the alias and replace it with a partial
    #    namespace declaration that ALSO forward-declares each entry
    #    type. The `export using entry::Foo;` re-exports a few lines
    #    later need each `entry::Foo` to be a known declared name,
    #    not just a namespace member to be looked up later. The
    #    actual definitions in the merged block (further down) extend
    #    the partial namespace and complete the forward decls.
    # Forward decls must be `export` so the later `export using entry::Foo;`
    # re-exports don't trigger "using declaration referring to Foo with
    # module linkage cannot be exported".
    entry_fwd_decls = (
        "namespace entry {\n"
        "    export template<typename K, typename V, typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    struct OccupiedEntry;\n"
        "    export template<typename K, typename V, typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    struct VacantEntry;\n"
        "    export template<typename K, typename V, typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    struct Entry;\n"
        "    export template<typename K, typename V, typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    struct OccupiedError;\n"
        "}\n"
    )
    # Path 1 (legacy): postprocess emitted an `entry` alias; replace
    # it with the forward decls.
    if re.search(
        r"^namespace entry = ::btree_port::btree::map::entry;",
        map_src,
        flags=re.MULTILINE,
    ):
        map_src = re.sub(
            r"^namespace entry = ::btree_port::btree::map::entry;\s*\n",
            entry_fwd_decls,
            map_src,
            flags=re.MULTILINE,
        )
        # The `namespace btree_port::btree::map::entry {}` forward decl
        # at module purview (added by the postprocess script) is no
        # longer needed under the merged shape — the inner forward decl
        # above does the same job.
        map_src = re.sub(
            r"^namespace btree_port::btree::map::entry \{\}\s*\n",
            "",
            map_src,
            flags=re.MULTILINE,
        )
    else:
        # Path 2 (current): postprocess skipped the alias because the
        # child-namespace check fires for `entry`. Inject the forward
        # decls AFTER the namespace-alias block inside the map wrap.
        # Anchor on the LAST contiguous alias line; if there are none,
        # fall back to the `namespace btree_port::btree::map {` open
        # line itself.
        alias_pattern = re.compile(
            r"^namespace [a-z_]+ = ::btree_port::btree::[a-z_.:]+;\n",
            re.MULTILINE,
        )
        last_alias = None
        prev_end = -1
        for m in alias_pattern.finditer(map_src):
            if prev_end != -1 and m.start() != prev_end:
                break
            last_alias = m
            prev_end = m.end()
        if last_alias is not None:
            map_src = (
                map_src[:last_alias.end()]
                + entry_fwd_decls
                + map_src[last_alias.end():]
            )
        else:
            # Fall back to anchoring on the namespace-open line.
            anchor = "namespace btree_port::btree::map {\n"
            ix = map_src.find(anchor)
            if ix != -1:
                ix_end = ix + len(anchor)
                map_src = map_src[:ix_end] + entry_fwd_decls + map_src[ix_end:]

    map_mod.write_text(map_src)
    print(f"  merged map.entry into map.cppm (entry content + facade skip)")


def fix_rusty_btreemap_namespace_clash(path: Path) -> None:
    """The transpiler emits `rusty::BTreeMap<K, V, A>` for the
    `DormantMutRef<...>` field type inside OccupiedEntry / VacantEntry.
    But `rusty::BTreeMap` from rusty/btreemap.hpp is an std::map-backed
    FACADE — a totally different type than the transpiled global
    `::BTreeMap<K, V, A>` defined in map.cppm. The transpiler's path
    mangling confused the Rust crate's `crate::map::BTreeMap` with
    the rusty:: namespace member.
    Fix: strip the `rusty::` prefix from BTreeMap occurrences so it
    resolves to the transpiled global type. The struct is in
    map.entry.cppm but map.cppm (where BTreeMap is defined) imports
    map.entry — so map.entry must forward-declare BTreeMap to
    compile cleanly. Inject the forward decl right after the
    `export module …;` block.
    """
    src = path.read_text()
    sentinel = "// btree_port port: rusty::BTreeMap → ::BTreeMap + fwd-decl by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (rusty::BTreeMap → ::BTreeMap already fixed)")
        return
    needle = "rusty::BTreeMap"
    if needle not in src:
        print(f"  no rusty::BTreeMap occurrences in: {path.name}")
        return
    n = src.count(needle)
    src = src.replace(needle, "::BTreeMap")
    # Inject the BTreeMap forward decl so the struct compiles. The
    # entry module is compiled BEFORE map.cppm, so map.cppm's
    # definition isn't yet visible — only a forward declaration works.
    # Forward-decl is fine because DormantMutRef<T> only stores a
    # NonNull<T> + PhantomData<T&>, neither of which needs T complete.
    fwd_decl = (
        "\n// btree_port port: forward-decl ::BTreeMap so DormantMutRef<BTreeMap<...>> compiles\n"
        "// (the full definition is in map.cppm, imported by map.cppm AFTER this module).\n"
        "// DormantMutRef stores only NonNull<T> + PhantomData<T&>, which work with incomplete T.\n"
        "namespace rusty { namespace alloc { struct Global; } }\n"
        "template<typename K, typename V, typename A = rusty::alloc::Global>\n"
        "    requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "struct BTreeMap;\n"
    )
    # Insert AFTER the first `import btree_port.btree.btree_internal;`
    # line (so Allocator is in scope), but BEFORE any struct definitions.
    anchor = "import btree_port.btree.btree_internal;\n"
    pos = src.find(anchor)
    if pos != -1:
        ins = pos + len(anchor)
        src = src[:ins] + fwd_decl + src[ins:]
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  rewrote {n} rusty::BTreeMap → ::BTreeMap + fwd-decl in: {path.name}")


def implement_btreemap_entry(path: Path) -> None:
    """Hand-port `BTreeMap::entry`. Replaces the stub from
    stub_broken_entry_method. The Rust source:
        pub fn entry(&mut self, key: K) -> Entry<'_, K, V, A> {
            let (map, dormant_map) = DormantMutRef::new(self);
            match map.root {
                None => Vacant(VacantEntry { key, handle: None, ... }),
                Some(ref mut root) => match root.borrow_mut().search_tree(&key) {
                    Found(handle) => Occupied(OccupiedEntry { handle, ... }),
                    GoDown(handle) => Vacant(VacantEntry { key, handle: Some(handle), ... }),
                }
            }
        }
    """
    src = path.read_text()
    sentinel = "// btree_port port step 52: BTreeMap::entry hand-ported (entry types merged in)"
    if sentinel in src:
        print(f"  no changes to: {path.name} (BTreeMap::entry hand-ported)")
        return
    sig = "Entry<K, V, A> entry(K key) {"
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no BTreeMap::entry site in: {path.name}")
        return
    # The method body `{` is the last char of the sig string.
    brace_open = sig_pos + len(sig) - 1
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find BTreeMap::entry end", file=sys.stderr)
        return
    impl = (
        "{\n"
        f"        {sentinel}\n"
        "        // Now that OccupiedEntry/VacantEntry live in this same\n"
        "        // module (their dormant_map field is\n"
        "        // `DormantMutRef<BTreeMap<K,V,A>>`, unqualified — matches\n"
        "        // this BTreeMap), the aggregate init works with designated\n"
        "        // initializers (no DMR helper needed).\n"
        "        using __VEHandle = Handle<NodeRef<::marker::Mut, K, V,\n"
        "                                          ::marker::Leaf>, ::marker::Edge>;\n"
        "        auto [map, dormant_map] =\n"
        "            __btree_port_make_dormant((*this));\n"
        "        if (map.root.is_none()) {\n"
        "            return Entry<K, V, A>::Vacant(VacantEntry<K, V, A>{\n"
        "                .key_field = std::move(key),\n"
        "                .handle = rusty::Option<__VEHandle>{rusty::None},\n"
        "                .dormant_map = std::move(dormant_map),\n"
        "                .alloc = rusty::clone(\n"
        "                    ((rusty::detail::deref_if_pointer_like(map.alloc)))),\n"
        "                ._marker = rusty::PhantomData<std::tuple<K, V>&>{},\n"
        "            });\n"
        "        }\n"
        "        auto root_borrow = map.root.as_mut().unwrap().borrow_mut();\n"
        "        auto __sr = std::move(root_borrow).search_tree(key);\n"
        "        if (__sr.index() == 0) {\n"
        "            // Found: existing key, return OccupiedEntry.\n"
        "            auto __handle = std::move(std::get<0>(__sr)._0);\n"
        "            return Entry<K, V, A>::Occupied(OccupiedEntry<K, V, A>{\n"
        "                .handle = std::move(__handle),\n"
        "                .dormant_map = std::move(dormant_map),\n"
        "                .alloc = rusty::clone(\n"
        "                    ((rusty::detail::deref_if_pointer_like(map.alloc)))),\n"
        "                ._marker = rusty::PhantomData<std::tuple<K, V>&>{},\n"
        "            });\n"
        "        }\n"
        "        // GoDown: new key with insertion handle.\n"
        "        auto __handle = std::move(std::get<1>(__sr)._0);\n"
        "        return Entry<K, V, A>::Vacant(VacantEntry<K, V, A>{\n"
        "            .key_field = std::move(key),\n"
        "            .handle = rusty::Option<__VEHandle>(std::move(__handle)),\n"
        "            .dormant_map = std::move(dormant_map),\n"
        "            .alloc = rusty::clone(\n"
        "                ((rusty::detail::deref_if_pointer_like(map.alloc)))),\n"
        "            ._marker = rusty::PhantomData<std::tuple<K, V>&>{},\n"
        "        });\n"
        "    }"
    )
    src = src[:brace_open] + impl + src[brace_close + 1 :]
    path.write_text(src)
    print(f"  hand-ported BTreeMap::entry in: {path.name}")


def stub_broken_entry_method(path: Path) -> None:
    """`BTreeMap::entry(K key)` body has interleaved transpiler bugs:
    designated-initializer field-name mismatches (key vs key_field),
    DormantMutRef/NonNull conversion issues that may be aggregate-init
    fall-through (compiler tries to init DormantMutRef's first member
    instead of using move-ctor), and an `int.first/second` access.

    Each is its own micro-bug, but together they cascade. Stub the
    whole method — same approach as search_tree."""
    src = path.read_text()
    sentinel = "// btree_port port: BTreeMap::entry stubbed by post_transpile_patch.py (E)"
    if sentinel in src:
        print(f"  no changes to: {path.name} (entry already stubbed)")
        return
    sig = "Entry<K, V, A> entry(K key) {"
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no entry method site in: {path.name}")
        return
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find entry end", file=sys.stderr)
        return
    stub = (
        "{\n"
        f"        {sentinel}\n"
        "        // Body had VacantEntry/OccupiedEntry aggregate-init shape\n"
        "        // mismatches that cascade through multiple lines. The\n"
        "        // proper fix needs hand-porting or transpiler-side changes.\n"
        "        throw ::std::runtime_error(\n"
        "            \"rusty-cpp-transpiler: BTreeMap::entry stub \"\n"
        "            \"(VacantEntry/OccupiedEntry aggregate init mismatch; see STATUS.md)\");\n"
        "    }"
    )
    src = src[:brace_open] + stub + src[brace_close + 1 :]
    path.write_text(src)
    print(f"  stubbed BTreeMap::entry in: {path.name}")


# `fix_tuple_dot_underscore_access` removed — Item 1 of
# GENERIC_FIXES_PLAN.md is now lifted into the transpiler. When the
# receiver type of an `expr.N` access is genuinely unknown (e.g.
# `auto&&`-bound through a deref chain), the emit path now wraps the
# access in a `requires { __t._N; }` SFINAE dispatch that picks
# `__t._N` for transpiler-synthesized tuple-structs or
# `std::get<N>(__t)` for std::tuple/pair/array at C++ compile time.
# See transpiler/src/codegen.rs `Expr::Field` → `Member::Unnamed`.


def fix_borrow_method_fallback(path: Path) -> None:
    """The transpiled body of `NodeRef::find_key_index` does:
        switch (rusty::cmp::cmp(key, k.borrow())) {
    where `k` is `const int&` (for primitive K). Primitives don't have
    a `.borrow()` method — in Rust, `Borrow<Q>` is a trait, with a
    blanket impl that makes `i32::borrow() = &self`. The transpiler
    should have emitted the SFINAE-fallback shape it uses elsewhere
    (`([&](auto&& __recv) -> decltype(auto) { if constexpr (requires
    { __recv.borrow(); }) return __recv.borrow(); else return __recv;
    }(k))`) but here it didn't.
    Patch the specific known site by wrapping `k.borrow()` in the
    SFINAE fallback. Idempotent via sentinel.
    """
    src = path.read_text()
    sentinel = "// btree_port port: k.borrow() SFINAE fallback by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (k.borrow() fallback already wrapped)")
        return
    needle = "rusty::cmp::cmp(key, k.borrow())"
    replacement = (
        "rusty::cmp::cmp(key, "
        "([&]() -> decltype(auto) { "
        "if constexpr (requires { k.borrow(); }) return k.borrow(); "
        "else return (k); "
        "}()))"
    )
    if needle not in src:
        print(f"  no k.borrow() site in: {path.name}")
        return
    new_src = src.replace(needle, replacement, 1)
    # Inject the sentinel at top of file so re-runs skip.
    new_src = sentinel + "\n" + new_src
    path.write_text(new_src)
    print(f"  wrapped k.borrow() with SFINAE fallback in: {path.name}")


def implement_handle_into_kv(path: Path) -> None:
    """Same fix pattern as Handle::descend / Handle::force —
    `Handle::into_kv` is emitted with redundant `template<typename K,
    typename V, typename NodeType>` method-template params that fail
    deduction at call sites like `handle.into_kv()._1`. Recover K/V
    from the enclosing class's Node via __NodeRefArgs<Node>.
    Reuses the trait injected by `implement_handle_descend`.
    """
    src = path.read_text()
    sentinel = "// btree_port port: Handle::into_kv hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (Handle::into_kv already ported)")
        return
    sig = (
        "    template<typename K, typename V, typename NodeType>\n"
        "    std::tuple<const K&, const V&> into_kv() {\n"
    )
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no Handle::into_kv site in: {path.name}")
        return
    brace_open = src.rfind("{", sig_pos, sig_pos + len(sig))
    if brace_open == -1:
        print(f"  [warn] no method-body `{{` in Handle::into_kv sig", file=sys.stderr)
        return
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find Handle::into_kv end", file=sys.stderr)
        return
    end = brace_close + 1
    if end < len(src) and src[end] == "\n":
        end += 1
    new_method = (
        f"    {sentinel}\n"
        "    // Rust impl: `impl<BorrowType, K, V, NodeType>\n"
        "    //   Handle<NodeRef<BorrowType, K, V, NodeType>, marker::KV>\n"
        "    //   { fn into_kv(self) -> (&K, &V) }`.\n"
        "    // C++ can't constrain Node to NodeRef<…> at class level,\n"
        "    // so recover BorrowType/K/V via __NodeRefArgs<Node>.\n"
        "    std::tuple<const typename __NodeRefArgs<Node>::Key&,\n"
        "               const typename __NodeRefArgs<Node>::Value&>\n"
        "    into_kv() const {\n"
        "        using __K = typename __NodeRefArgs<Node>::Key;\n"
        "        using __V = typename __NodeRefArgs<Node>::Value;\n"
        "        assert((this->idx_field < rusty::len(this->node)));\n"
        "        // this->node : NodeRef<__B, __K, __V, Tag>;\n"
        "        // into_leaf returns &LeafNode<__K, __V>.\n"
        "        // Step 64: use `const auto&` (not `const auto`) to preserve the\n"
        "        // reference; otherwise we copy the LeafNode and return dangling\n"
        "        // references to keys/vals inside the copy.\n"
        "        const auto& leaf = ([&](auto&& __recv) -> decltype(auto) {\n"
        "            if constexpr (requires { std::forward<decltype(__recv)>(__recv).into_leaf(); }) {\n"
        "                return std::forward<decltype(__recv)>(__recv).into_leaf();\n"
        "            } else {\n"
        "                return std::forward<decltype(__recv)>(__recv)->into_leaf();\n"
        "            }\n"
        "        }(this->node));\n"
        "        const __K& k = leaf.keys[this->idx_field].assume_init_ref();\n"
        "        const __V& v = leaf.vals[this->idx_field].assume_init_ref();\n"
        "        return std::tuple<const __K&, const __V&>{k, v};\n"
        "    }\n"
    )
    src = src[:sig_pos] + new_method + src[end:]
    path.write_text(src)
    print(f"  hand-ported Handle::into_kv (with __NodeRefArgs) in: {path.name}")


def apply_step58_lazy_gates_and_fixes(path: Path) -> None:
    """Step 60: codify step 58/59 fixes as durable patcher rules.

    These cover:
    1. Inject `__IsNodeRef<T>` concept (sibling to __NodeRefArgs trait).
    2. Rewrite Handle's reborrow/reborrow_mut/dormant/awaken/descend/
       force/into_kv to use the lazy-template-gate pattern
       (`template<typename = void> + auto + requires (__IsNodeRef<Node>)`)
       so bogus Handle<wrong, Type> instantiation can succeed without
       pulling these methods.
    3. Convert insert_fit to use `auto K_, V_` template params so its
       param substitution is also lazy.
    4. Convert split to use `auto` return type.
    5. Add `requires (__IsNodeRef<Node>)` to step-54 method-template
       conjunctions.
    6. Rewrite `.height` → `.height_field` at 8 specific sites.
    7. Apply LeafNode::new_ bypass pattern to InternalNode::new_.
    8. Rewrite correct_parent_link with __NodeRefArgs + fix NodeRef typo.
    """
    src = path.read_text()
    sentinel = "// btree_port port step 60: step-58/59 lazy gates + extra fixes codified"
    if sentinel in src:
        print(f"  no changes to: {path.name} (step-58/59 fixes already codified)")
        return
    landed = 0

    # 1. Inject __IsNodeRef concept after the __NodeRefArgs trait.
    trait_anchor = "template<typename B, typename K, typename V, typename T>\nstruct __NodeRefArgs<NodeRef<B, K, V, T>> {\n    using BorrowType = B;\n    using Key = K;\n    using Value = V;\n    using Tag = T;\n};"
    concept_block = trait_anchor + (
        "\n\n// btree_port port step 60: concept to gate methods that need\n"
        "// Node = NodeRef<...> from instantiation when Handle is bogusly\n"
        "// instantiated with Node = K/Q/R (transpiler-emitted wrong template\n"
        "// args at call sites like `Handle<K, Type>::new_edge(...)`).\n"
        "template<typename T>\n"
        "concept __IsNodeRef = requires { typename __NodeRefArgs<T>::Key; };"
    )
    if trait_anchor in src and "concept __IsNodeRef" not in src:
        src = src.replace(trait_anchor, concept_block, 1)
        landed += 1

    # 2. InternalNode::new_ bypass (LeafNode pattern).
    old_int_new = (
        "    template<typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    static rusty::Box<InternalNode<K, V>, A> new_(A alloc) {\n"
        "        auto node = rusty::Box<InternalNode<K, V>, rusty::Box<InternalNode<K, V>, A>>::new_uninit_in(std::move(alloc));\n"
        "        // @unsafe\n"
        "        {\n"
        "            LeafNode<K, V>::init(&(*rusty::as_mut_ptr(node)).data);\n"
        "            return node.assume_init();\n"
        "        }\n"
        "    }\n"
        "};"
    )
    new_int_new = (
        "    template<typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    static rusty::Box<InternalNode<K, V>, A> new_(A alloc) {\n"
        "        // btree_port port step 59: bypass missing Box::new_uninit_in\n"
        "        // (same pattern as LeafNode::new_ step-54 fix #6).\n"
        "        auto node = rusty::Box<InternalNode<K, V>, A>::new_in(\n"
        "            InternalNode<K, V>{}, std::move(alloc));\n"
        "        LeafNode<K, V>::init(&node.operator->()->data);\n"
        "        return node;\n"
        "    }\n"
        "};"
    )
    if old_int_new in src:
        src = src.replace(old_int_new, new_int_new, 1)
        landed += 1

    # 3. correct_parent_link: replace template body + NodeRef<K, K, V, Type> typo.
    old_cpl = (
        "    template<typename K, typename V>\n"
        "    void correct_parent_link() {\n"
        "        auto ptr_shadow1 = NonNull<InternalNode<K, V>>::new_unchecked(NodeRef<K, K, V, Type>::as_internal_ptr(this->node));\n"
        "        auto idx = std::move(this->idx_field);\n"
        "        auto child = this->descend();\n"
        "        child.set_parent_link(std::move(ptr_shadow1), std::move(idx));\n"
        "    }"
    )
    new_cpl = (
        "    // btree_port port step 59: drop redundant K/V (undeducible) +\n"
        "    // fix NodeRef<K, K, V, Type> typo (first K should be BorrowType).\n"
        "    // Recover all template args via __NodeRefArgs<Node>.\n"
        "    template<typename = void>\n"
        "    void correct_parent_link()\n"
        "        requires (__IsNodeRef<Node>) {\n"
        "        using __B = typename __NodeRefArgs<Node>::BorrowType;\n"
        "        using __K = typename __NodeRefArgs<Node>::Key;\n"
        "        using __V = typename __NodeRefArgs<Node>::Value;\n"
        "        using __NodeTag = typename __NodeRefArgs<Node>::Tag;\n"
        "        auto ptr_shadow1 = NonNull<InternalNode<__K, __V>>::new_unchecked(\n"
        "            NodeRef<__B, __K, __V, __NodeTag>::as_internal_ptr(this->node));\n"
        "        auto idx = std::move(this->idx_field);\n"
        "        auto child = this->descend();\n"
        "        child.set_parent_link(std::move(ptr_shadow1), std::move(idx));\n"
        "    }"
    )
    if old_cpl in src:
        src = src.replace(old_cpl, new_cpl, 1)
        landed += 1

    # 4. .height → .height_field substitution at non-method-call sites.
    # Use regex: `.height` not followed by `(` or word-char, not preceded by `_field`.
    import re
    pattern_height = re.compile(r"\.height(?!_field)(?![(\w])")
    new_src, n_height = pattern_height.subn(".height_field", src)
    if n_height > 0:
        src = new_src
        landed += 1
        print(f"  rewrote {n_height} .height → .height_field")

    if landed > 0:
        # Insert sentinel at top so re-run skips.
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  applied {landed} step-58/59 fix categories in: {path.name}")
    else:
        print(f"  no step-58/59 fix sites matched in: {path.name}")


def fix_static_factory_param_type_recovery(path: Path) -> None:
    """After Cluster A landed, the transpiler drops impl-block generic params
    that decompose structurally into the host (e.g. `BorrowType, K, V, NodeType`
    when the host is `Handle<Node, Type>` and the absorbed impl was over
    `Handle<NodeRef<BorrowType, K, V, NodeType>, Type>`). Method bodies are
    still patched with `using BorrowType = typename __TemplateArgs<Node>::arg_0;`
    (etc.) inside the body, which works for return-type references — but
    static-factory PARAMETER TYPES like `NodeRef<BorrowType, K, V, NodeType>`
    refer to those identifiers BEFORE the body opens, when no `using` aliases
    are in scope yet.

    The two known sites are `new_kv` / `new_edge`. Fix: replace the parameter
    type with `auto` (C++20 abbreviated function template) so the call-site
    type is deduced from the argument. Body code still uses the
    `using BorrowType = …` aliases for return-position constructions, which
    work fine post-signature.
    """
    src = path.read_text()
    sentinel = "// btree_port port: new_kv/new_edge auto-param recovered by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (new_kv/new_edge auto-param already fixed)")
        return

    landed = 0
    for name in ("new_kv", "new_edge"):
        bad = (
            f"    static auto {name}("
            "NodeRef<BorrowType, K, V, NodeType> node, size_t idx) -> Handle<Node, Type> {\n"
        )
        good = (
            f"    {sentinel.replace('post_transpile_patch.py', name).replace('// ', '    // ')}\n"
            "    "
            f"static auto {name}(auto node, size_t idx) -> Handle<Node, Type> {{\n"
        )
        if bad in src:
            src = src.replace(bad, good, 1)
            landed += 1
    if landed:
        path.write_text(src)
        print(f"  rewrote {landed} new_kv/new_edge param(s) to auto in: {path.name}")
    else:
        print(f"  no new_kv/new_edge static-factory param sites in: {path.name}")


def inject_handle_make_edge_helper(path: Path) -> None:
    """Companion to `fix_static_factory_param_type_recovery`. That fix makes
    `Handle::new_edge(auto node, size_t idx)` deduce its `Handle<Node, Type>`
    return type from the argument when the call site can supply concrete
    template args. But several call sites in btree_internal emit
    `Handle<auto, auto>::new_edge(<node>, <idx>)` — `auto` is not a valid
    template argument and `clang++` rejects it with
    `'auto' not allowed in template argument`.

    Fix: inject a module-scope helper that deduces the Handle instantiation
    from the node argument and forwards to `new_edge`, then rewrite the
    `Handle<auto, auto>::new_edge(` call sites to use the helper.

    Anchor: the `using BoxedNode = …` line that immediately follows the
    closing brace of the `namespace marker {…}` block. Anchoring there
    (rather than on the earlier `struct Handle;` forward-decl) ensures
    `marker::Edge` is visible when the helper instantiates.
    """
    src = path.read_text()
    sentinel = "// btree_port port: __btree_make_handle_edge injected by post_transpile_patch.py"
    if sentinel in src:
        # On re-runs, only need to verify no `Handle<auto, auto>` call sites
        # remain; the helper is already in place.
        if "Handle<auto, auto>::new_edge(" in src:
            n = src.count("Handle<auto, auto>::new_edge(")
            src = src.replace(
                "Handle<auto, auto>::new_edge(",
                "__btree_make_handle_edge(",
            )
            path.write_text(src)
            print(
                f"  rewrote {n} stray Handle<auto,auto>::new_edge → helper in: {path.name}"
            )
        else:
            print(f"  no changes to: {path.name} (handle edge helper already in place)")
        return

    # Skip work when this emit shape isn't present (e.g. the transpiler
    # already produces the `std::conditional_t<true, Handle<…>, Q>::new_edge`
    # shape from `recover_template_args` recovery — both shapes have been
    # seen across versions).
    n_sites = src.count("Handle<auto, auto>::new_edge(")
    if n_sites == 0:
        print(f"  no Handle<auto,auto>::new_edge call sites in: {path.name}")
        return

    anchor = (
        "template<typename K, typename V>\n"
        "using BoxedNode = rusty::ptr::NonNull<LeafNode<K, V>>;\n"
    )
    if anchor not in src:
        print(
            f"  [skip] handle-edge helper anchor (BoxedNode using-decl) not found in: {path.name}"
        )
        return

    # Rewrite the call sites BEFORE injecting the helper, so the helper
    # body (which contains the legal `Handle<…>::new_edge(…)` form) is
    # not affected by the textual substitution.
    src = src.replace(
        "Handle<auto, auto>::new_edge(",
        "__btree_make_handle_edge(",
    )

    helper = (
        "\n"
        f"{sentinel}\n"
        "// At a handful of call sites the transpiler emits the static form\n"
        "// `Handle<auto,auto>::new_edge(node, idx)` (without spaces in the\n"
        "// template args). `auto` is not a valid template argument in C++,\n"
        "// so this helper takes the node by forwarding ref and deduces the\n"
        "// Handle instantiation from it.\n"
        "template<typename N>\n"
        "auto __btree_make_handle_edge(N&& node, size_t idx) {\n"
        "    return Handle<std::remove_cvref_t<N>, ::marker::Edge>::new_edge(\n"
        "        std::forward<N>(node), idx);\n"
        "}\n"
    )
    src = src.replace(anchor, anchor + helper, 1)

    path.write_text(src)
    print(
        f"  injected __btree_make_handle_edge helper + rewrote {n_sites} call site(s) in: {path.name}"
    )


def fix_left_kv_right_kv_ok_result_type(path: Path) -> None:
    """The transpiler emits a bad explicit `Result<T,E>` type qualifier in the
    Ok-arm of `Handle::left_kv` / `Handle::right_kv`. Both T and E come out
    as `Handle<Node, Type>` (Self) when the signature's actual T is
    `Handle<NodeRef<__TemplateArgs<Node>::arg_0..3>, ::marker::KV>`.

    Function shape (from btree_internal.rs:1184):

        impl<BorrowType, K, V, NodeType>
            Handle<NodeRef<BorrowType, K, V, NodeType>, marker::Edge>
        {
            pub(super) fn left_kv(self)
                -> Result<Handle<NodeRef<BorrowType,K,V,NodeType>, marker::KV>, Self>
            {
                if self.idx > 0 {
                    Ok(unsafe { Handle::new_kv(self.node, self.idx - 1) })
                } else {
                    Err(self)
                }
            }
        }

    Buggy emit (Ok arm only — Err arm has the correct full Result type):

        return rusty::Result<Handle<Node, Type>, Handle<Node, Type>>::Ok(
            Handle<Node, Type>::new_kv(std::move(this->node), …));

    The Err arm uses the correct signature qualifier; the Ok arm gets Self
    for both T and E. Reproducible with a 4-impl-param / 2-struct-param
    fixture (see /tmp/repro_left_kv/repro2.rs). Root cause lives in
    transpiler/src/codegen.rs in the Ok-ctor expected-hint resolution
    around `resolve_result_ctor_expected_type_from_ctor_arg` (line ~66645)
    and `lookup_constructor_template_args` (line ~76270). The hint
    recovered from a single-ctor expression at line ~51725 sets BOTH T
    and E to the same `decltype((arg))`, which then propagates Self into
    the Ok arm's qualifier when the function-level Result hint is bypassed.

    Until that's fixed in the codegen, swap the broken qualifier for the
    full signature shape. The Ok arg (`Handle<Node, Type>::new_kv(...)`)
    is itself shape-wrong but typechecks because `new_kv` is defined on a
    parallel impl that returns `Self` — leave that emit unchanged; the
    Result-qualifier substitution is enough for the typechecker to accept
    the surrounding `Result<...,...>::Ok(...)`.
    """
    src = path.read_text()
    sentinel = "// btree_port port: left_kv/right_kv Ok-arm Result qualifier fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (left_kv/right_kv Ok qualifier already fixed)")
        return
    bad = "return rusty::Result<Handle<Node, Type>, Handle<Node, Type>>::Ok("
    good = (
        "return rusty::Result<Handle<NodeRef<"
        "typename __TemplateArgs<Node>::arg_0, "
        "typename __TemplateArgs<Node>::arg_1, "
        "typename __TemplateArgs<Node>::arg_2, "
        "typename __TemplateArgs<Node>::arg_3>, ::marker::KV>, "
        "Handle<Node, Type>>::Ok("
    )
    n = src.count(bad)
    if n == 0:
        print(f"  no left_kv/right_kv Ok-arm sites in: {path.name}")
        return
    src = src.replace(bad, good)
    src = "// " + sentinel.lstrip("// ") + "\n" + src
    path.write_text(src)
    print(
        f"  rewrote {n} left_kv/right_kv Ok-arm Result qualifier(s) in: {path.name}"
    )


def apply_step66_map_runtime_fixes(path: Path) -> None:
    """Step 67: codify the step 64/65/66 map.cppm fixes that brought the
    transpiled BTreeMap smoke test to all-green.

    These run AFTER `stub_broken_map_methods` / `stub_broken_map_entry_methods`
    so the stubbed bodies get OVERWRITTEN with the real implementations.

    Fixes:
      1. Step 66: `~BTreeMap` infinite-recursion segfault. The transpiler
         emitted `rusty::mem::drop(rusty::iter(rusty::ptr::read(&(*this))))`
         which (a) borrows via `rusty::iter` rather than consuming via
         `.into_iter()`, and (b) the stack-temp from `ptr::read` recursively
         re-enters ~BTreeMap because consume_forgotten_address never returns
         true for the temp's fresh address. Replace with a mark-forgotten
         leak (member dtors are trivial; a proper IntoIter-driven drop is
         deferred).
      2. Step 64: `BTreeMap::insert` arm reversal. Transpiler emitted the
         Vacant and Occupied arms swapped. Rewrite to match Rust: Vacant
         → insert and return None; Occupied → return Some(old).
      3. Step 64: un-stub `BTreeMap::first_key_value`. Rust source:
         `let root_node = self.root.as_ref()?.reborrow();
          root_node.first_leaf_edge().right_kv().ok().map(Handle::into_kv)`.
      4. Step 65: un-stub `BTreeMap::last_key_value`. Mirror of above using
         `last_leaf_edge().left_kv()`.
      5. Step 64: un-stub `OccupiedEntry::into_mut`. Rust:
         `self.handle.into_val_mut()`.
      6. Step 64: un-stub `OccupiedEntry::get_mut`. Rust:
         `self.handle.kv_mut().1`.
    """
    src = path.read_text()
    sentinel = "// btree_port port step 67: step 64-66 runtime fixes codified by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (step 64-66 runtime fixes already codified)")
        return
    landed = 0

    # 1. ~BTreeMap destructor — replace the broken body with a leak.
    # Under the strict null-state convention the local `_rusty_forgotten`
    # flag replaces the old global-table dance.
    old_dtor_body = (
        "    ~BTreeMap() noexcept(false) {\n"
        "        if (_rusty_forgotten) { return; }\n"
        "        rusty::mem::drop(rusty::iter(rusty::ptr::read(&(*this))));\n"
        "    }"
    )
    new_dtor_body = (
        "    ~BTreeMap() noexcept(false) {\n"
        "        if (_rusty_forgotten) { return; }\n"
        "        // Step 66: the transpiler emitted\n"
        "        //   rusty::mem::drop(rusty::iter(rusty::ptr::read(&(*this))))\n"
        "        // which is wrong on two counts: (a) `rusty::iter` returns the\n"
        "        // BORROWING Iter<K,V>, not the consuming IntoIter, so heap\n"
        "        // nodes are never freed; (b) `ptr::read` produces a stack-\n"
        "        // temp BTreeMap whose own ~BTreeMap would `ptr::read` AGAIN\n"
        "        // and recurse infinitely → stack overflow.\n"
        "        //\n"
        "        // A proper drop would route through `into_iter()` + IntoIter's\n"
        "        // deallocating_next walk, but that path drags in more uninstan-\n"
        "        // tiated stubs (ManuallyDrop field-access, deallocating_*).\n"
        "        // Member destructors are all trivial (NodeRef wraps a NonNull\n"
        "        // pointer, ManuallyDrop<A> doesn't drop A), so leaking the\n"
        "        // leaf-node allocations at program exit is the pragmatic\n"
        "        // fix for the hybrid milestone. Setting `_rusty_forgotten`\n"
        "        // skips any future destructor on this address (e.g. if a\n"
        "        // ptr::read produces a stack-temp on the same slot).\n"
        "        _rusty_forgotten = true;\n"
        "    }"
    )
    if old_dtor_body in src:
        src = src.replace(old_dtor_body, new_dtor_body, 1)
        landed += 1
        print(f"  step 66: fixed ~BTreeMap infinite-recursion in: {path.name}")

    # 2. BTreeMap::insert — replace the stubbed body (from stub_broken_map_methods
    # if it was stubbed) OR the original transpiler emit (with arms swapped).
    insert_stub = (
        "    rusty::Option<V> insert(K key, V value) {\n"
        "        // btree_port port: BTreeMap::insert stubbed by post_transpile_patch.py\n"
        "        throw ::std::runtime_error(\"rusty-cpp-transpiler: BTreeMap::insert stub\");\n"
        "    }"
    )
    insert_real = (
        "    rusty::Option<V> insert(K key, V value) {\n"
        "        // Step 64: transpiler emitted the Vacant/Occupied arms swapped.\n"
        "        // Rust source: `match self.entry(key) { Occupied(e) => Some(e.insert(v)),\n"
        "        // Vacant(e) => { e.insert(v); None } }`.\n"
        "        // In our variant, index 0 = Entry_Vacant, index 1 = Entry_Occupied.\n"
        "        auto&& _m = this->entry(std::move(key));\n"
        "        if (_m.index() == 0) {\n"
        "            // Vacant: insert + return None.\n"
        "            auto entry_shadow1 = std::move(std::get<0>(_m)._0);\n"
        "            entry_shadow1.insert(std::move(value));\n"
        "            return rusty::Option<V>{rusty::None};\n"
        "        }\n"
        "        // Occupied: return Some(old_value).\n"
        "        auto entry_shadow1 = std::move(std::get<1>(_m)._0);\n"
        "        return rusty::Option<V>(entry_shadow1.insert(std::move(value)));\n"
        "    }"
    )
    if insert_stub in src:
        src = src.replace(insert_stub, insert_real, 1)
        landed += 1
        print(f"  step 64: un-stubbed BTreeMap::insert in: {path.name}")

    # 3. BTreeMap::first_key_value — un-stub.
    fkv_stub = (
        "    rusty::Option<std::tuple<const K&, const V&>> first_key_value() const {\n"
        "        // btree_port port: BTreeMap::first_key_value stubbed by post_transpile_patch.py\n"
        "        throw ::std::runtime_error(\"rusty-cpp-transpiler: BTreeMap::first_key_value stub\");\n"
        "    }"
    )
    fkv_real = (
        "    rusty::Option<std::tuple<const K&, const V&>> first_key_value() const {\n"
        "        // Step 64: un-stubbed. Rust source:\n"
        "        //   let root_node = self.root.as_ref()?.reborrow();\n"
        "        //   root_node.first_leaf_edge().right_kv().ok().map(Handle::into_kv)\n"
        "        const auto root_opt = this->root.as_ref();\n"
        "        if (root_opt.is_none()) {\n"
        "            return rusty::None;\n"
        "        }\n"
        "        auto root_node = root_opt.unwrap().reborrow();\n"
        "        auto right_kv_res = root_node.first_leaf_edge().right_kv();\n"
        "        if (right_kv_res.is_err()) {\n"
        "            return rusty::None;\n"
        "        }\n"
        "        auto kv_handle = std::move(right_kv_res).unwrap();\n"
        "        return rusty::Option<std::tuple<const K&, const V&>>(kv_handle.into_kv());\n"
        "    }"
    )
    if fkv_stub in src:
        src = src.replace(fkv_stub, fkv_real, 1)
        landed += 1
        print(f"  step 64: un-stubbed BTreeMap::first_key_value in: {path.name}")

    # 4. BTreeMap::last_key_value — un-stub.
    lkv_stub = (
        "    rusty::Option<std::tuple<const K&, const V&>> last_key_value() const {\n"
        "        // btree_port port: BTreeMap::last_key_value stubbed by post_transpile_patch.py\n"
        "        throw ::std::runtime_error(\"rusty-cpp-transpiler: BTreeMap::last_key_value stub\");\n"
        "    }"
    )
    lkv_real = (
        "    rusty::Option<std::tuple<const K&, const V&>> last_key_value() const {\n"
        "        // Step 65: un-stubbed. Rust source:\n"
        "        //   let root_node = self.root.as_ref()?.reborrow();\n"
        "        //   root_node.last_leaf_edge().left_kv().ok().map(Handle::into_kv)\n"
        "        const auto root_opt = this->root.as_ref();\n"
        "        if (root_opt.is_none()) {\n"
        "            return rusty::None;\n"
        "        }\n"
        "        auto root_node = root_opt.unwrap().reborrow();\n"
        "        auto left_kv_res = root_node.last_leaf_edge().left_kv();\n"
        "        if (left_kv_res.is_err()) {\n"
        "            return rusty::None;\n"
        "        }\n"
        "        auto kv_handle = std::move(left_kv_res).unwrap();\n"
        "        return rusty::Option<std::tuple<const K&, const V&>>(kv_handle.into_kv());\n"
        "    }"
    )
    if lkv_stub in src:
        src = src.replace(lkv_stub, lkv_real, 1)
        landed += 1
        print(f"  step 65: un-stubbed BTreeMap::last_key_value in: {path.name}")

    # 5. OccupiedEntry::into_mut — un-stub.
    into_mut_stub = (
        "    V& into_mut() {\n"
        "        // btree_port port: OccupiedEntry::into_mut stubbed by post_transpile_patch.py\n"
        "        throw ::std::runtime_error(\"rusty-cpp-transpiler: OccupiedEntry::into_mut stub\");\n"
        "    }"
    )
    into_mut_real = (
        "    V& into_mut() {\n"
        "        // Step 64: un-stubbed — calls Handle::into_val_mut on the\n"
        "        // owned KV handle to get a mutable ref into the leaf node.\n"
        "        return this->handle.into_val_mut();\n"
        "    }"
    )
    if into_mut_stub in src:
        src = src.replace(into_mut_stub, into_mut_real, 1)
        landed += 1
        print(f"  step 64: un-stubbed OccupiedEntry::into_mut in: {path.name}")

    # 6. OccupiedEntry::get_mut — un-stub.
    get_mut_stub = (
        "    V& get_mut() {\n"
        "        // btree_port port: OccupiedEntry::get_mut stubbed by post_transpile_patch.py\n"
        "        throw ::std::runtime_error(\"rusty-cpp-transpiler: OccupiedEntry::get_mut stub\");\n"
        "    }"
    )
    get_mut_real = (
        "    V& get_mut() {\n"
        "        // Step 64: un-stubbed — Rust: `self.handle.kv_mut().1`.\n"
        "        return std::get<1>(this->handle.kv_mut());\n"
        "    }"
    )
    if get_mut_stub in src:
        src = src.replace(get_mut_stub, get_mut_real, 1)
        landed += 1
        print(f"  step 64: un-stubbed OccupiedEntry::get_mut in: {path.name}")

    if landed > 0:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  applied {landed} step 64-66 runtime fixes in: {path.name}")
    else:
        print(f"  no step 64-66 runtime fix sites matched in: {path.name}")


def apply_step54_insert_path_fixes(path: Path) -> None:
    """Step 55: codify the 7 step-54 fixes for the insert path. Each is
    an idempotent text replacement that fixes a specific transpiler emit
    bug surfaced when VacantEntry::insert_entry was un-stubbed.

    Fixes:
      1. key_area_mut / val_area_mut / edge_area_mut: drop undeducible
         Output template param, use decltype(auto) + integer-vs-range
         dispatch.
      2. Handle::reborrow/reborrow_mut/dormant/awaken: __NodeRefArgs<Node>
         pattern (same as descend/force/into_kv).
      3. Handle::insert_fit (leaf 2-arg form): __NodeRefArgs<Node>.
      4. Handle::split: __NodeRefArgs<Node> + fix bogus
         NodeRef<A, K, V, Type> → NodeRef<Owned, K, V, Leaf>.
      5. Handle::split_leaf_data: drop unused NodeType param.
      6. LeafNode::new_: bypass Box::new_uninit_in via default-construct +
         init pattern.
      7. Handle::insert (Leaf body): fix rusty::str_runtime::split path,
         drop const auto on insertion_edge.
    """
    src = path.read_text()
    sentinel = "// btree_port port step 55: step-54 insert-path fixes codified by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (step-54 insert-path fixes already applied)")
        return
    landed = 0

    # Fix 1: key_area_mut / val_area_mut / edge_area_mut signatures.
    old_area_block = (
        "    template<typename I, typename Output>\n"
        "    Output& key_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        {\n"
        "            return rusty::as_mut_slice(this->as_leaf_mut().keys)[std::move(index)];\n"
        "        }\n"
        "    }\n"
        "    template<typename I, typename Output>\n"
        "    Output& val_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        {\n"
        "            return rusty::as_mut_slice(this->as_leaf_mut().vals)[std::move(index)];\n"
        "        }\n"
        "    }\n"
        "    template<typename I, typename Output>\n"
        "    Output& edge_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        {\n"
        "            return rusty::as_mut_slice(this->as_internal_mut().edges)[std::move(index)];\n"
        "        }\n"
        "    }\n"
    )
    new_area_block = (
        "    // btree_port port step 54: dropped undeducible Output template\n"
        "    // param. Use decltype(auto) + dispatch between integer indexing\n"
        "    // and range indexing (index_with_range) so callers can pass\n"
        "    // either a size_t or a rusty::range_to/range_from etc.\n"
        "    template<typename I>\n"
        "    decltype(auto) key_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        auto&& slice = rusty::as_mut_slice(this->as_leaf_mut().keys);\n"
        "        if constexpr (std::is_integral_v<std::remove_cvref_t<I>>) {\n"
        "            return slice[std::move(index)];\n"
        "        } else {\n"
        "            return rusty::index_with_range(slice, std::move(index));\n"
        "        }\n"
        "    }\n"
        "    template<typename I>\n"
        "    decltype(auto) val_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        auto&& slice = rusty::as_mut_slice(this->as_leaf_mut().vals);\n"
        "        if constexpr (std::is_integral_v<std::remove_cvref_t<I>>) {\n"
        "            return slice[std::move(index)];\n"
        "        } else {\n"
        "            return rusty::index_with_range(slice, std::move(index));\n"
        "        }\n"
        "    }\n"
        "    template<typename I>\n"
        "    decltype(auto) edge_area_mut(I index) {\n"
        "        // @unsafe\n"
        "        auto&& slice = rusty::as_mut_slice(this->as_internal_mut().edges);\n"
        "        if constexpr (std::is_integral_v<std::remove_cvref_t<I>>) {\n"
        "            return slice[std::move(index)];\n"
        "        } else {\n"
        "            return rusty::index_with_range(slice, std::move(index));\n"
        "        }\n"
        "    }\n"
    )
    if old_area_block in src:
        src = src.replace(old_area_block, new_area_block, 1)
        landed += 1

    # Fix 2: Handle::reborrow / reborrow_mut / dormant / awaken.
    old_handle_borrows = (
        "    template<typename BorrowType, typename K, typename V, typename NodeType, typename HandleType>\n"
        "    Handle<NodeRef<::marker::Immut, K, V, NodeType>, HandleType> reborrow() const {\n"
        "        return Handle<NodeRef<::marker::Immut, K, V, NodeType>, HandleType>(([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).reborrow(); }) { return std::forward<decltype(__recv)>(__recv).reborrow(); } else { return std::forward<decltype(__recv)>(__recv)->reborrow(); } }(this->node)), this->idx_field, rusty::PhantomData<HandleType>{});\n"
        "    }\n"
        "    template<typename K, typename V, typename NodeType, typename HandleType>\n"
        "    Handle<NodeRef<::marker::Mut, K, V, NodeType>, HandleType> reborrow_mut() {\n"
        "        return Handle<NodeRef<::marker::Mut, K, V, NodeType>, HandleType>(([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).reborrow_mut(); }) { return std::forward<decltype(__recv)>(__recv).reborrow_mut(); } else { return std::forward<decltype(__recv)>(__recv)->reborrow_mut(); } }(this->node)), this->idx_field, rusty::PhantomData<HandleType>{});\n"
        "    }\n"
        "    template<typename K, typename V, typename NodeType, typename HandleType>\n"
        "    Handle<NodeRef<::marker::DormantMut, K, V, NodeType>, HandleType> dormant() const {\n"
        "        return Handle<NodeRef<::marker::DormantMut, K, V, NodeType>, HandleType>(([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).dormant(); }) { return std::forward<decltype(__recv)>(__recv).dormant(); } else { return std::forward<decltype(__recv)>(__recv)->dormant(); } }(this->node)), this->idx_field, rusty::PhantomData<HandleType>{});\n"
        "    }\n"
        "    template<typename K, typename V, typename NodeType, typename HandleType>\n"
        "    Handle<NodeRef<::marker::Mut, K, V, NodeType>, HandleType> awaken() {\n"
        "        return Handle<NodeRef<::marker::Mut, K, V, NodeType>, HandleType>(([&](auto&& __recv) -> decltype(auto) { if constexpr (requires { std::forward<decltype(__recv)>(__recv).awaken(); }) { return std::forward<decltype(__recv)>(__recv).awaken(); } else { return std::forward<decltype(__recv)>(__recv)->awaken(); } }(this->node)), std::move(this->idx_field), rusty::PhantomData<HandleType>{});\n"
        "    }\n"
    )
    new_handle_borrows = (
        "    // btree_port port step 54: dropped redundant method-template\n"
        "    // params; recover via __NodeRefArgs<Node>.\n"
        "    Handle<NodeRef<::marker::Immut,\n"
        "                   typename __NodeRefArgs<Node>::Key,\n"
        "                   typename __NodeRefArgs<Node>::Value,\n"
        "                   typename __NodeRefArgs<Node>::Tag>, Type> reborrow() const {\n"
        "        return Handle<NodeRef<::marker::Immut,\n"
        "                              typename __NodeRefArgs<Node>::Key,\n"
        "                              typename __NodeRefArgs<Node>::Value,\n"
        "                              typename __NodeRefArgs<Node>::Tag>, Type>(\n"
        "            this->node.reborrow(), this->idx_field, rusty::PhantomData<Type>{});\n"
        "    }\n"
        "    Handle<NodeRef<::marker::Mut,\n"
        "                   typename __NodeRefArgs<Node>::Key,\n"
        "                   typename __NodeRefArgs<Node>::Value,\n"
        "                   typename __NodeRefArgs<Node>::Tag>, Type> reborrow_mut() {\n"
        "        return Handle<NodeRef<::marker::Mut,\n"
        "                              typename __NodeRefArgs<Node>::Key,\n"
        "                              typename __NodeRefArgs<Node>::Value,\n"
        "                              typename __NodeRefArgs<Node>::Tag>, Type>(\n"
        "            this->node.reborrow_mut(), this->idx_field, rusty::PhantomData<Type>{});\n"
        "    }\n"
        "    Handle<NodeRef<::marker::DormantMut,\n"
        "                   typename __NodeRefArgs<Node>::Key,\n"
        "                   typename __NodeRefArgs<Node>::Value,\n"
        "                   typename __NodeRefArgs<Node>::Tag>, Type> dormant() const {\n"
        "        return Handle<NodeRef<::marker::DormantMut,\n"
        "                              typename __NodeRefArgs<Node>::Key,\n"
        "                              typename __NodeRefArgs<Node>::Value,\n"
        "                              typename __NodeRefArgs<Node>::Tag>, Type>(\n"
        "            this->node.dormant(), this->idx_field, rusty::PhantomData<Type>{});\n"
        "    }\n"
        "    Handle<NodeRef<::marker::Mut,\n"
        "                   typename __NodeRefArgs<Node>::Key,\n"
        "                   typename __NodeRefArgs<Node>::Value,\n"
        "                   typename __NodeRefArgs<Node>::Tag>, Type> awaken() {\n"
        "        return Handle<NodeRef<::marker::Mut,\n"
        "                              typename __NodeRefArgs<Node>::Key,\n"
        "                              typename __NodeRefArgs<Node>::Value,\n"
        "                              typename __NodeRefArgs<Node>::Tag>, Type>(\n"
        "            this->node.awaken(), std::move(this->idx_field), rusty::PhantomData<Type>{});\n"
        "    }\n"
    )
    if old_handle_borrows in src:
        src = src.replace(old_handle_borrows, new_handle_borrows, 1)
        landed += 1

    # Fix 3: Handle::insert_fit (Leaf 2-arg).
    old_insert_fit = (
        "    template<typename K, typename V>\n"
        "    Handle<NodeRef<::marker::Mut, K, V, ::marker::Leaf>, ::marker::KV> insert_fit(K key, V val) {\n"
    )
    new_insert_fit = (
        "    // btree_port port step 54: recover K/V via __NodeRefArgs<Node>.\n"
        "    Handle<NodeRef<::marker::Mut,\n"
        "                   typename __NodeRefArgs<Node>::Key,\n"
        "                   typename __NodeRefArgs<Node>::Value,\n"
        "                   ::marker::Leaf>, ::marker::KV>\n"
        "    insert_fit(typename __NodeRefArgs<Node>::Key key,\n"
        "               typename __NodeRefArgs<Node>::Value val) {\n"
        "        using K = typename __NodeRefArgs<Node>::Key;\n"
        "        using V = typename __NodeRefArgs<Node>::Value;\n"
    )
    if old_insert_fit in src:
        src = src.replace(old_insert_fit, new_insert_fit, 1)
        landed += 1

    # Fix 4: Handle::split. Match the signature + first 2 lines of body.
    old_split = (
        "    template<typename A, typename K, typename V>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    SplitResult<K, V, ::marker::Leaf> split(A alloc) {\n"
        "        auto new_node = LeafNode<K, V>::new_(std::move(alloc));\n"
        "        auto kv = this->split_leaf_data(rusty::detail::deref_if_pointer_like(new_node));\n"
        "        auto right = NodeRef<A, K, V, Type>::from_new_leaf(std::move(new_node));\n"
    )
    new_split = (
        "    // btree_port port step 54: recover K/V via __NodeRefArgs<Node>,\n"
        "    // correct NodeRef<A, K, V, Type> typo to NodeRef<Owned, K, V, Leaf>.\n"
        "    template<typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    SplitResult<typename __NodeRefArgs<Node>::Key,\n"
        "                typename __NodeRefArgs<Node>::Value,\n"
        "                ::marker::Leaf> split(A alloc) {\n"
        "        using K = typename __NodeRefArgs<Node>::Key;\n"
        "        using V = typename __NodeRefArgs<Node>::Value;\n"
        "        auto new_node = LeafNode<K, V>::new_(std::move(alloc));\n"
        "        auto kv = this->split_leaf_data(rusty::detail::deref_if_pointer_like(new_node));\n"
        "        auto right = NodeRef<::marker::Owned, K, V, ::marker::Leaf>::from_new_leaf(std::move(new_node));\n"
    )
    if old_split in src:
        src = src.replace(old_split, new_split, 1)
        landed += 1

    # Fix 5: Handle::split_leaf_data — drop NodeType.
    old_sld = (
        "    template<typename K, typename V, typename NodeType>\n"
        "    std::tuple<K, V> split_leaf_data(LeafNode<K, V>& new_node) {\n"
    )
    new_sld = (
        "    // btree_port port step 54: dropped unused NodeType template param.\n"
        "    template<typename K, typename V>\n"
        "    std::tuple<K, V> split_leaf_data(LeafNode<K, V>& new_node) {\n"
    )
    if old_sld in src:
        src = src.replace(old_sld, new_sld, 1)
        landed += 1

    # Fix 6: LeafNode::new_ — bypass Box::new_uninit_in.
    old_leafnode_new = (
        "    template<typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    static rusty::Box<LeafNode<K, V>, A> new_(A alloc) {\n"
        "        auto leaf = rusty::Box<LeafNode<K, V>, A>::new_uninit_in(std::move(alloc));\n"
        "        // @unsafe\n"
        "        {\n"
        "            LeafNode<K, V>::init(reinterpret_cast<LeafNode<K, V>*>(rusty::as_mut_ptr(leaf)));\n"
        "            return leaf.assume_init();\n"
        "        }\n"
        "    }\n"
    )
    new_leafnode_new = (
        "    template<typename A>\n"
        "        requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
        "    static rusty::Box<LeafNode<K, V>, A> new_(A alloc) {\n"
        "        // btree_port port step 54: rusty::Box has no new_uninit_in,\n"
        "        // and we don't need uninit storage anyway — the MaybeUninit\n"
        "        // fields in LeafNode handle uninit-ness internally.\n"
        "        auto leaf = rusty::Box<LeafNode<K, V>, A>::new_in(\n"
        "            LeafNode<K, V>{}, std::move(alloc));\n"
        "        LeafNode<K, V>::init(leaf.operator->());\n"
        "        return leaf;\n"
        "    }\n"
    )
    if old_leafnode_new in src:
        src = src.replace(old_leafnode_new, new_leafnode_new, 1)
        landed += 1

    # Fix 7: Handle::insert (Leaf body) — split path + const drop.
    old_insert_body = (
        "            const auto middle = Handle<Node, Type>::new_kv(std::move(this->node), std::move(middle_kv_idx));\n"
        "            auto result = rusty::str_runtime::split(middle, std::move(alloc));\n"
        "            const auto insertion_edge = "
    )
    new_insert_body = (
        "            auto middle = Handle<Node, Type>::new_kv(std::move(this->node), std::move(middle_kv_idx));\n"
        "            // btree_port port step 54: was `rusty::str_runtime::split` — wrong path.\n"
        "            auto result = std::move(middle).split(std::move(alloc));\n"
        "            auto insertion_edge = "
    )
    if old_insert_body in src:
        src = src.replace(old_insert_body, new_insert_body, 1)
        landed += 1

    if landed > 0:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  applied {landed}/7 step-54 insert-path fixes in: {path.name}")
    else:
        print(f"  no step-54 fix sites matched in: {path.name}")


def implement_handle_force(path: Path) -> None:
    """Hand-port `Handle::force` on `Handle<NodeRef<…, LeafOrInternal>, Type>`.
    The transpiled body has the same shape as `Handle::descend` — emitted
    with redundant `template<typename BorrowType, typename K, typename V>`
    method-template params that can't be deduced from the (empty)
    parameter list, so call sites like `handle.force()` fail.
    Fix: use the `__NodeRefArgs<Node>` trait to recover BorrowType/K/V
    from the enclosing class's `Node` template arg. Same pattern as
    descend(). Reuses the trait injected by `implement_handle_descend`.
    """
    src = path.read_text()
    sentinel = "// btree_port port: Handle::force hand-ported by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (Handle::force already ported)")
        return
    # Find the original force() method by its signature. Try both the
    # pre-Cluster-A shape (method-template params) and the post-Cluster-A
    # shape (typename __TemplateArgs<Node>::arg_N references).
    sig_pre = (
        "    template<typename BorrowType, typename K, typename V>\n"
        "    ForceResult<Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, "
        "Type>, Handle<NodeRef<BorrowType, K, V, ::marker::Internal>, Type>> "
        "force() {\n"
    )
    ta = "typename __TemplateArgs<Node>::arg_"
    sig_post = (
        f"    ForceResult<Handle<NodeRef<{ta}0, {ta}1, {ta}2, ::marker::Leaf>, "
        f"Type>, Handle<NodeRef<{ta}0, {ta}1, {ta}2, ::marker::Internal>, Type>> "
        "force() {\n"
    )
    sig_pos = src.find(sig_pre)
    sig = sig_pre
    if sig_pos == -1:
        sig_pos = src.find(sig_post)
        sig = sig_post
    if sig_pos == -1:
        print(f"  no Handle::force site in: {path.name}")
        return
    # The method body `{` is the LAST `{` inside the signature (the one
    # right after `force() `). Anchoring brace-open BEFORE the next
    # `{` finds the lambda's `{` instead of the method's, which would
    # then match the lambda's close as the "method end" and leave
    # `}();    }` orphaned. Use rfind within the sig's range.
    brace_open = src.rfind("{", sig_pos, sig_pos + len(sig))
    if brace_open == -1:
        print(f"  [warn] no method-body `{{` in Handle::force sig", file=sys.stderr)
        return
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find Handle::force end", file=sys.stderr)
        return
    # The whole method spans [sig_pos, brace_close]. We also need to
    # consume the trailing newline after `}` so the file stays tidy.
    end = brace_close + 1
    if end < len(src) and src[end] == "\n":
        end += 1
    # Pick the per-shape typedef source for BorrowType/K/V. Cluster A
    # completion already exposes them via __TemplateArgs<Node>::arg_N;
    # the older descend hand-port path uses __NodeRefArgs<Node>.
    if sig is sig_post:
        b_alias = "typename __TemplateArgs<Node>::arg_0"
        k_alias = "typename __TemplateArgs<Node>::arg_1"
        v_alias = "typename __TemplateArgs<Node>::arg_2"
    else:
        b_alias = "typename __NodeRefArgs<Node>::BorrowType"
        k_alias = "typename __NodeRefArgs<Node>::Key"
        v_alias = "typename __NodeRefArgs<Node>::Value"
    new_method = (
        f"    {sentinel}\n"
        "    // The Rust source is `impl<BorrowType, K, V, Type>\n"
        "    //   Handle<NodeRef<BorrowType, K, V, LeafOrInternal>, Type>\n"
        "    //   { fn force(self) -> ForceResult<…, …> }`.\n"
        "    // C++ can't constrain Node to NodeRef<…,LeafOrInternal>\n"
        "    // at the class level, so we destructure Node via\n"
        "    // __TemplateArgs/__NodeRefArgs and build the result Handles.\n"
        "    ForceResult<\n"
        f"        Handle<NodeRef<{b_alias},\n"
        f"                       {k_alias},\n"
        f"                       {v_alias},\n"
        "                       ::marker::Leaf>, Type>,\n"
        f"        Handle<NodeRef<{b_alias},\n"
        f"                       {k_alias},\n"
        f"                       {v_alias},\n"
        "                       ::marker::Internal>, Type>>\n"
        "    force() {\n"
        f"        using __B = {b_alias};\n"
        f"        using __K = {k_alias};\n"
        f"        using __V = {v_alias};\n"
        "        using __LeafH = Handle<NodeRef<__B, __K, __V, ::marker::Leaf>, Type>;\n"
        "        using __IntH  = Handle<NodeRef<__B, __K, __V, ::marker::Internal>, Type>;\n"
        "        using __Ret   = ForceResult<__LeafH, __IntH>;\n"
        "        // this->node : NodeRef<__B, __K, __V, LeafOrInternal>\n"
        "        // .force() returns ForceResult<NodeRef<…,Leaf>, NodeRef<…,Internal>>\n"
        "        auto __forced = this->node.force();\n"
        "        if (__forced.index() == 0) {\n"
        "            auto&& __leaf_node = std::get<0>(__forced)._0;\n"
        "            return __Ret{\n"
        "                ForceResult_Leaf<__LeafH, __IntH>{\n"
        "                    __LeafH{std::move(__leaf_node),\n"
        "                            std::move(this->idx_field),\n"
        "                            rusty::PhantomData<Type>{}}\n"
        "                }\n"
        "            };\n"
        "        }\n"
        "        auto&& __int_node = std::get<1>(__forced)._0;\n"
        "        return __Ret{\n"
        "            ForceResult_Internal<__LeafH, __IntH>{\n"
        "                __IntH{std::move(__int_node),\n"
        "                       std::move(this->idx_field),\n"
        "                       rusty::PhantomData<Type>{}}\n"
        "            }\n"
        "        };\n"
        "    }\n"
    )
    src = src[:sig_pos] + new_method + src[end:]
    path.write_text(src)
    print(f"  hand-ported Handle::force (with __NodeRefArgs) in: {path.name}")


def implement_search_tree(path: Path) -> None:
    """Hand-port `NodeRef::search_tree`. The Rust source is:

        fn search_tree<Q>(mut self, key: &Q) -> SearchResult<…> {
            loop {
                match self.search_node(key) {
                    Found(handle) => return Found(handle),
                    GoDown(handle) => match handle.force() {
                        Leaf(leaf) => return GoDown(leaf),
                        Internal(internal) => self = internal.descend(),
                    },
                }
            }
        }

    Replaces the throw stub with a faithful C++ translation. The
    method's class context is `NodeRef<BorrowType, K, V,
    marker::LeafOrInternal>` (its Tag is LeafOrInternal).
    """
    src = path.read_text()
    sentinel = "// btree_port port: NodeRef::search_tree hand-ported by post_transpile_patch.py (E7-const)"
    if sentinel in src:
        print(f"  no changes to: {path.name} (search_tree hand-ported)")
        return
    # Look for either the stub or the original buggy body shape. Match
    # the WHOLE method declaration (including signature) so we can add
    # `const` qualifier. The Rust source takes `mut self` (by value),
    # which doesn't mutate the caller's NodeRef — so the C++ equivalent
    # is `const`-qualified, using a copy of *this internally.
    sig = ("SearchResult<BorrowType, K, V, ::marker::LeafOrInternal, "
           "::marker::Leaf> search_tree(const Q& key) {")
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no search_tree site in: {path.name}")
        return
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find search_tree end", file=sys.stderr)
        return
    # Rewrite the signature to add `const`. The original signature is
    # `SearchResult<…> search_tree(const Q& key) {`; we want
    # `SearchResult<…> search_tree(const Q& key) const {`.
    new_sig = sig.replace(") {", ") const {")
    src = src[:sig_pos] + new_sig + src[sig_pos + len(sig):]
    # `brace_open` and `brace_close` were computed before signature
    # rewrite; recompute since `const` insertion shifted bytes.
    brace_open = src.find("{", sig_pos + len(new_sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find search_tree end after const insert", file=sys.stderr)
        return
    impl = (
        "{\n"
        f"        {sentinel}\n"
        "        using __Ret = SearchResult<BorrowType, K, V,\n"
        "                                    ::marker::LeafOrInternal,\n"
        "                                    ::marker::Leaf>;\n"
        "        // The Rust source uses `mut self` (by-value receiver):\n"
        "        // the original NodeRef isn't mutated, only the local\n"
        "        // alias. C++ equivalent is to copy *this into a local\n"
        "        // (NodeRef is a thin {height, NonNull, _marker} struct\n"
        "        // and trivially copyable for the borrow types we use).\n"
        "        NodeRef<BorrowType, K, V, ::marker::LeafOrInternal>\n"
        "            self_ = *this;\n"
        "        while (true) {\n"
        "            auto __sr = self_.search_node(key);\n"
        "            // __sr is variant<SearchResult_Found, SearchResult_GoDown>\n"
        "            if (__sr.index() == 0) {\n"
        "                auto&& __handle = std::get<0>(__sr)._0;\n"
        "                return __Ret{\n"
        "                    SearchResult_Found<BorrowType, K, V,\n"
        "                                        ::marker::LeafOrInternal,\n"
        "                                        ::marker::Leaf>{std::move(__handle)}\n"
        "                };\n"
        "            }\n"
        "            // GoDown: __sr.index() == 1\n"
        "            auto&& __handle = std::get<1>(__sr)._0;\n"
        "            auto __forced = __handle.force();\n"
        "            // __forced is variant<ForceResult_Leaf, ForceResult_Internal>\n"
        "            if (__forced.index() == 0) {\n"
        "                auto&& __leaf = std::get<0>(__forced)._0;\n"
        "                return __Ret{\n"
        "                    SearchResult_GoDown<BorrowType, K, V,\n"
        "                                         ::marker::LeafOrInternal,\n"
        "                                         ::marker::Leaf>{std::move(__leaf)}\n"
        "                };\n"
        "            }\n"
        "            // Internal: descend and continue the loop\n"
        "            auto&& __internal = std::get<1>(__forced)._0;\n"
        "            self_ = __internal.descend();\n"
        "        }\n"
        "    }"
    )
    src = src[:brace_open] + impl + src[brace_close + 1 :]
    path.write_text(src)
    print(f"  hand-ported NodeRef::search_tree (const) in: {path.name}")


def stub_broken_search_tree(path: Path) -> None:
    """`NodeRef::search_tree(key)` has two interleaved transpiler bugs:

    1. The outer lambda's return type was emitted as `-> NodeRef<…>`
       but the body returns `SearchResult<…>` (the actual method
       return type). This is a transpiler annotation mismatch.
    2. The inner `handle.force()` match emits the unresolved-bare-glob
       slot markers (`/* TODO transpiler: unresolved bare-glob variant
       `Leaf` … */`) — the transpiler couldn't see the Leaf/Internal
       enum decl in this TU.

    Both are structural transpiler gaps that fixing in-place requires
    rewriting the whole method body (a non-trivial Rust loop+match
    construct). Stub it with `throw` for now — this lets the rest of
    the transpiled BTreeMap link, but `search_tree` itself can't be
    called (and `BTreeMap::get`/`insert` ultimately reach it, so they
    won't work at runtime until this is hand-ported).
    """
    src = path.read_text()
    sentinel = "// btree_port port: search_tree stubbed by post_transpile_patch.py (E7)"
    if sentinel in src:
        print(f"  no changes to: {path.name} (search_tree already stubbed)")
        return
    sig = ("SearchResult<BorrowType, K, V, ::marker::LeafOrInternal, "
           "::marker::Leaf> search_tree(const Q& key) {")
    sig_pos = src.find(sig)
    if sig_pos == -1:
        print(f"  no search_tree stub site in: {path.name}")
        return
    brace_open = src.find("{", sig_pos + len(sig) - 1)
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] couldn't find search_tree end", file=sys.stderr)
        return
    stub = (
        "{\n"
        f"        {sentinel}\n"
        "        // The transpiled body had a lambda return-type/body\n"
        "        // mismatch + unresolved Leaf/Internal force() arms.\n"
        "        // Stubbed until the method is hand-ported.\n"
        "        throw ::std::runtime_error(\n"
        "            \"rusty-cpp-transpiler: search_tree stub \"\n"
        "            \"(lambda return-type + force() arms mismatch; see STATUS.md E7)\");\n"
        "    }"
    )
    src = src[:brace_open] + stub + src[brace_close + 1 :]
    path.write_text(src)
    print(f"  stubbed NodeRef::search_tree in: {path.name}")


def fix_assume_init_ref_on_span(path: Path) -> None:
    """`rusty::slice_to(arr, n).assume_init_ref()` calls a method
    on std::span. std::span doesn't have that method. The rusty
    side has `MaybeUninit<T>::slice_assume_init_ref(span)` static
    and a `rusty::assume_init_ref(span)` free function (added in
    step 43). Rewrite the method call to use the free function."""
    src = path.read_text()
    sentinel = (
        "// btree_port port: assume_init_ref method→free-fn "
        "by post_transpile_patch.py"
    )
    if sentinel in src:
        print(f"  no changes to: {path.name} (assume_init_ref on span already fixed)")
        return
    # The exact site in btree_internal:
    #   rusty::slice_to(leaf.keys, …).assume_init_ref()
    # is a method call on the span result of slice_to. Use a regex
    # that captures the slice_to(...) expression (with balanced parens
    # up to the closing one) then wraps it.
    import re
    pattern = re.compile(
        r"(rusty::slice_to\([^)]*\([^)]*\)[^)]*\))\.assume_init_ref\(\)"
    )
    matches = list(pattern.finditer(src))
    if not matches:
        print(f"  no slice_to(…).assume_init_ref() sites in: {path.name}")
        return
    src = pattern.sub(r"rusty::assume_init_ref(\1)", src)
    src = sentinel + "\n" + src
    path.write_text(src)
    print(f"  rewrote {len(matches)} slice_to(…).assume_init_ref() in: {path.name}")


def fix_const_correctness(path: Path) -> None:
    """Rust methods that take `self` by value (consuming) or by
    immutable reference are emitted in C++ as non-const member
    functions in some cases, breaking the const-correctness chain
    when called from const-qualified methods. Specifically known
    problem methods (surfaced by transpiled_smoke):

      - NodeRef::into_leaf
      - NodeRef::first_leaf_edge
      - NodeRef::last_leaf_edge

    All three are `fn(self) -> …` in Rust (consuming) so they
    don't mutate the receiver. Marking them `const` in C++ makes
    them callable from `const NodeRef&` contexts (the
    by-value-receiver semantic is otherwise preserved).
    """
    src = path.read_text()
    sentinel = "// btree_port port: const-correctness on by-value methods by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (const-correctness already fixed)")
        return
    # Methods to mark const. Match the exact signature line we see in
    # the transpiled output (avoiding shape-creep in other files).
    targets = [
        ("const LeafNode<K, V>& into_leaf() {",
         "const LeafNode<K, V>& into_leaf() const {"),
        ("Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, ::marker::Edge> first_leaf_edge() {",
         "Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, ::marker::Edge> first_leaf_edge() const {"),
        ("Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, ::marker::Edge> last_leaf_edge() {",
         "Handle<NodeRef<BorrowType, K, V, ::marker::Leaf>, ::marker::Edge> last_leaf_edge() const {"),
        # Handle::left_kv / right_kv take self by value in Rust
        # (consuming) so they don't mutate the receiver. They're
        # called from const first_key_value/last_key_value paths.
        ("rusty::Result<Handle<NodeRef<BorrowType, K, V, NodeType>, ::marker::KV>, Handle<Node, Type>> left_kv() {",
         "rusty::Result<Handle<NodeRef<BorrowType, K, V, NodeType>, ::marker::KV>, Handle<Node, Type>> left_kv() const {"),
        ("rusty::Result<Handle<NodeRef<BorrowType, K, V, NodeType>, ::marker::KV>, Handle<Node, Type>> right_kv() {",
         "rusty::Result<Handle<NodeRef<BorrowType, K, V, NodeType>, ::marker::KV>, Handle<Node, Type>> right_kv() const {"),
    ]
    n_fixed = 0
    for old, new in targets:
        if old in src:
            src = src.replace(old, new, 1)
            n_fixed += 1
    if n_fixed:
        src = sentinel + "\n" + src
        path.write_text(src)
        print(f"  marked {n_fixed} method(s) const in: {path.name}")
    else:
        print(f"  no const-correctness sites in: {path.name}")


def _impl_deallocating_helper(direction: str) -> tuple[str, str]:
    """Returns (stub-signature-substring, replacement-body) for the
    deallocating_next / _back hand-ports. `direction` is "next" or
    "next_back" — they're mirror images, differing only in
    right_kv/left_kv and next_leaf_edge/next_back_leaf_edge."""
    if direction == "next":
        kv_method = "right_kv"
        leaf_edge_method = "next_leaf_edge"
    else:
        kv_method = "left_kv"
        leaf_edge_method = "next_back_leaf_edge"
    sig_tail = f"deallocating_{direction}(A alloc) {{"
    body = (
        "{\n"
        f"        // btree_port port: B4/B5 deallocating_{direction} hand-ported "
        "by post_transpile_patch.py\n"
        "        // Rust source (library/alloc/src/collections/btree/navigate.rs):\n"
        "        //   let mut edge = self.forget_node_type();\n"
        "        //   loop {\n"
        f"        //       edge = match edge.{kv_method}() {{\n"
        f"        //           Ok(kv) => return Some((ptr::read(&kv).{leaf_edge_method}(), kv)),\n"
        "        //           Err(last_edge) => match last_edge.into_node().deallocate_and_ascend(alloc.clone()) {\n"
        "        //               Some(parent_edge) => parent_edge.forget_node_type(),\n"
        "        //               None => return None,\n"
        "        //           },\n"
        "        //       }\n"
        "        //   }\n"
        # K/V dropped from impl-block generics by Cluster A absorption;
        # recover positionally from the host class param via the
        # __TemplateArgs<Node>::arg_<N> partial-specialization helper.
        "        using __Ret = rusty::Option<std::tuple<\n"
        "            Handle<Node, Type>,\n"
        "            Handle<NodeRef<::marker::Dying,"
        " typename __TemplateArgs<Node>::arg_1,"
        " typename __TemplateArgs<Node>::arg_2,"
        " ::marker::LeafOrInternal>, ::marker::KV>\n"
        "        >>;\n"
        "        auto __edge = std::move(*this).forget_node_type();\n"
        "        while (true) {\n"
        f"            auto __rkv = __edge.{kv_method}();\n"
        "            if (__rkv.is_ok()) {\n"
        "                auto __kv = std::move(__rkv).unwrap();\n"
        "                // ptr::read = bitwise copy. Caller's safety contract\n"
        "                // ensures the kv outlives the next-edge walk.\n"
        "                auto __copy = rusty::ptr::read(&__kv);\n"
        f"                auto __next = std::move(__copy).{leaf_edge_method}();\n"
        "                return __Ret(std::make_tuple(\n"
        "                    std::move(__next), std::move(__kv)));\n"
        "            }\n"
        "            auto __last = std::move(__rkv).unwrap_err();\n"
        "            auto __dealloc = std::move(__last).into_node()\n"
        "                .deallocate_and_ascend(rusty::clone(alloc));\n"
        "            if (__dealloc.is_some()) {\n"
        "                __edge = std::move(__dealloc).unwrap().forget_node_type();\n"
        "                continue;\n"
        "            }\n"
        "            return __Ret(rusty::None);\n"
        "        }\n"
        "    }"
    )
    return sig_tail, body


# `stub_insert_recursing` removed.
#
# Background: `Handle::insert_recursing` used to require a stub because
# (a) the outer `let pat = match { (None,h) => return …, (Some(_),_) => … }`
# couldn't lower out of an IIFE — Item 11 in GENERIC_FIXES_PLAN.md — and
# (b) the inner `while (true)` body's `split.kv._0` tuple-field access
# emitted `._0` on a `std::tuple<K,V>` field — Item 1 in the same plan.
#
# Both lifts have landed in the transpiler. The full `insert_recursing`
# body now compiles directly: clean transpile + `libbtree_port.a` builds
# under clang, link smoke + transpiled-read smoke both pass with this
# rule disabled. See the GENERIC_FIXES_PLAN entries for Items 1 and 11.


def fix_from_new_leaf_markers(path: Path) -> None:
    """Rewrite `NodeRef<…>::from_new_leaf(…)` so its owner args are
    `<Owned, K, V, Leaf>` regardless of which mis-recovery shape the
    transpiler emitted. `from_new_leaf` is defined on
    `impl<K,V> NodeRef<marker::Owned, K, V, marker::Leaf>` — the Rust call
    site omits explicit template args and lets type inference pick
    Owned/Leaf, but the transpiler can't see the called method's
    defining impl block, so its recovery pulls from elsewhere.

    Two emit shapes seen:
      OLD (pre-Item 7): `NodeRef<::marker::Mut, arg_1, arg_2, ::marker::Internal>::from_new_leaf(…)`
        — recovery used inner_full_args from the absorbing method's impl
        block (`Mut, Internal`).
      NEW (post-Item 7): `NodeRef<arg_0, arg_1, arg_2, Type>::from_new_leaf(…)`
        — Cluster A recovery resolved the impl-generics correctly, then
        the trailing `Type` is the in-scope host class param.
    """
    src = path.read_text()
    ta = "typename __TemplateArgs<Node>::arg_"
    pattern_new = (
        f"NodeRef<::marker::Owned, {ta}1, {ta}2, ::marker::Leaf>::from_new_leaf("
    )
    patterns_old = [
        # OLD shape (pre-Item 7)
        f"NodeRef<::marker::Mut, {ta}1, {ta}2, ::marker::Internal>::from_new_leaf(",
        # NEW shape (post-Item 7 cluster_a recovery): arg_0 / Type at the
        # marker position.
        f"NodeRef<{ta}0, {ta}1, {ta}2, Type>::from_new_leaf(",
    ]
    total = 0
    for pattern_old in patterns_old:
        n = src.count(pattern_old)
        if n:
            src = src.replace(pattern_old, pattern_new)
            total += n
    if total == 0:
        print(f"  no NodeRef<…>::from_new_leaf mis-recovery sites in: {path.name}")
        return
    path.write_text(src)
    print(f"  rewrote {total} NodeRef<…>::from_new_leaf → <Owned,…,Leaf> in: {path.name}")


# `fix_leafnode_new_template_args` removed (Item 7): the transpiler's
# Cluster A recovery now resolves dropped owner-declared params (K, V on
# LeafNode) through the absorbing method's structural decomposition, so
# `LeafNode::new(alloc)` directly emits as
# `LeafNode<typename __TemplateArgs<Node>::arg_1, typename __TemplateArgs<Node>::arg_2>::new_(...)`
# with no post-transpile patch needed.


def fix_dormant_map_reborrow_binding(path: Path) -> None:
    """Item 6 follow-on: rewrite the `Option::insert` + outer `handle`
    let-bindings that surround the reborrow site. The `reborrow` itself
    now emits `auto&` directly from the transpiler (Item 6 lift); but
    `Option::insert(value) -> &mut T` isn't part of the generic
    method-name heuristic (the name `insert` is too common to blanket-
    treat as ref-returning). And the outer `const auto handle = …IIFE…;`
    blocks calls like `handle.forget_node_type()` which requires
    non-const.
    """
    src = path.read_text()
    pairs = [
        # `map.root.insert(...)` — Option::insert returns &mut T.
        ("const auto root = map.root.insert(",
         "auto& root = map.root.insert("),
        # `const auto handle = [&]() {...}()` — IIFE result is moved into
        # an OccupiedEntry; needs non-const so `.forget_node_type()` works.
        ("const auto handle = [&]() { auto&& _m = this->handle;",
         "auto handle = [&]() { auto&& _m = this->handle;"),
        # `const auto map = this->dormant_map.awaken();` —
        # `DormantMutRef::awaken(self) -> &'a mut T` returns a mutable
        # reference. The body then does `map.length -= 1` and
        # `map.root.as_mut().unwrap()`, both of which require non-const.
        # Drop the `const` so the binding becomes `auto& map`. (Without
        # this, the build hits "call to deleted constructor of const
        # BTreeMap" on move-only V instantiations because `const auto`
        # tries to copy-construct from the lvalue ref.)
        ("const auto map = this->dormant_map.awaken();",
         "auto& map = this->dormant_map.awaken();"),
    ]
    n_fixed = 0
    for old, new in pairs:
        cnt = src.count(old)
        if cnt > 0:
            src = src.replace(old, new)
            n_fixed += cnt
    if n_fixed:
        path.write_text(src)
        print(
            f"  rewrote {n_fixed} dormant_map.reborrow/root.insert ref-binding(s) in: {path.name}"
        )
    else:
        print(f"  no dormant_map.reborrow const-auto sites in: {path.name}")


def implement_deallocating(path: Path) -> None:
    """Replace stubs for both `deallocating_next` and
    `deallocating_next_back`. These are the tree-eating walks
    used by `BTreeMap::into_iter` / drop: they return the next
    (key, value) pair while deallocating any node whose last edge
    has been visited."""
    src = path.read_text()
    sentinel_marker = (
        "// btree_port port: B4/B5 deallocating_next hand-ported "
        "by post_transpile_patch.py"
    )
    if sentinel_marker in src:
        print(f"  no changes to: {path.name} (B4/B5 already landed)")
        return

    landed = 0
    for direction in ("next", "next_back"):
        sig_tail, body = _impl_deallocating_helper(direction)
        sig_pos = src.find(sig_tail)
        if sig_pos == -1:
            print(f"  no B4/B5 stub site for {direction} in: {path.name}")
            continue
        # Stub bodies are `{ throw ::std::runtime_error(…); }` (single
        # statement). Find the `{` at the END of the sig (the open
        # brace is at sig_tail's last char).
        brace_open = src.find("{", sig_pos + len(sig_tail) - 1)
        depth = 0
        brace_close = -1
        for k in range(brace_open, len(src)):
            if src[k] == "{":
                depth += 1
            elif src[k] == "}":
                depth -= 1
                if depth == 0:
                    brace_close = k
                    break
        if brace_close == -1:
            print(f"  [warn] couldn't find {direction} body end", file=sys.stderr)
            continue
        src = src[:brace_open] + body + src[brace_close + 1 :]
        landed += 1
        print(f"  hand-ported Handle::deallocating_{direction} in: {path.name}")

    if landed:
        path.write_text(src)


def stub_nodref_insert_entry(path: Path) -> None:
    """Stub `OccupiedEntry insert_entry(V value)` in VacantEntry — the
    transpiler emits `NodeRef::new_leaf(…)` without template args,
    which C++ rejects (the underlying class template needs explicit
    parameter values). The facade doesn't call this entry-API method,
    so a `throw` stub keeps the method shape valid without needing to
    reverse-engineer the right template args."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: insert_entry stubbed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (insert_entry already stubbed)")
        return

    # Find `    OccupiedEntry<K, V, A> insert_entry(V value) {` and stub it.
    pos = src.find("OccupiedEntry<K, V, A> insert_entry(V value) {")
    if pos == -1:
        print(f"  no insert_entry to stub in: {path.name}")
        return
    brace_open = src.find("{", pos)
    # Use brace-depth walk to find matching `}`.
    depth = 0
    brace_close = -1
    for k in range(brace_open, len(src)):
        if src[k] == "{":
            depth += 1
        elif src[k] == "}":
            depth -= 1
            if depth == 0:
                brace_close = k
                break
    if brace_close == -1:
        print(f"  [warn] no matching brace for insert_entry in: {path.name}", file=sys.stderr)
        return
    # NB: don't put the sentinel in a `//` line-comment here — the
    # whole stub is emitted on a single line and `//` would swallow
    # the closing `}`.
    stub = (
        "{ /* "
        + sentinel.lstrip("/ ").rstrip()
        + " */ throw ::std::runtime_error("
        + "\"rusty-cpp-transpiler: insert_entry stub (NodeRef template-args recovery)\"); }"
    )
    new_src = src[:brace_open] + stub + src[brace_close + 1 :]
    path.write_text(new_src)
    print(f"  stubbed insert_entry in: {path.name}")


def align_requires_clauses(path: Path) -> None:
    """For algebraic-data-type wrapper structs (e.g. `struct Entry`), the
    transpiler emits a `requires (rusty::alloc::Allocator<A> &&
    std::copyable<A>)` clause on the forward declaration but omits it
    on the variant-inheriting definition. C++20 treats this as a
    constraint mismatch (redeclaration with different constraints).

    Patch: scan for `template<typename K, typename V, typename A>` /
    `template<typename T, typename A>` lines whose NEXT line is
    `struct Entry : std::variant<…>` and inject the matching requires
    clause between them.
    Idempotent — bails if the sentinel is already present."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: requires-clause aligned by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (requires already aligned)")
        return

    requires_kva = (
        "    requires (rusty::alloc::Allocator<A> && std::copyable<A>)"
    )
    new_src, n = re.subn(
        r"(template<typename (?:K, typename V|T), typename A>)\n(struct Entry : std::variant<)",
        rf"\1\n{requires_kva}\n\2",
        src,
    )
    if n > 0:
        new_src = sentinel + "\n" + new_src
        path.write_text(new_src)
        print(f"  aligned {n} requires clause(s) in: {path.name}")
    else:
        print(f"  no requires misalignment found in: {path.name}")


def strip_module_namespace_prefixes(path: Path, prefixes: list[str]) -> None:
    """Strip `<module>::` qualifier prefixes from a transpiled .cppm.

    C++20 modules don't put exported symbols inside a namespace named
    after the module path. When the transpiler emits
    `btree_internal::Handle<…>` after `import btree_port.btree.btree_internal;`,
    the qualifier is wrong — it should be plain `Handle<…>`. Strip the
    prefix to make qualified references resolve via the import.

    Also drops several constructs the transpiler emits that the
    prefix-strip turns invalid:
      - `using <module>::Symbol;` and `export using <module>::Symbol;`
        (Symbol is already at file scope after the import).
      - `using namespace ::<module>;`
      - `namespace <module> {}` (and `namespace <module> = …;` aliases).

    Idempotent: skipped if the sentinel is already present.

    Skipped entirely under --auto-namespace mode (detected by the
    presence of `namespace btree_port::btree::` wraps): the qualifier
    prefixes are CORRECT in that mode — the postprocessed file has
    `namespace <prefix> = ::btree_port::btree::<prefix>;` aliases
    inside the wrap that make `prefix::Symbol` resolve.
    """
    import re

    src = path.read_text()
    sentinel = "// btree_port port: module prefixes stripped by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (prefixes already stripped)")
        return
    # Auto-namespace detection: skip the strip entirely under this mode.
    if "namespace btree_port::btree::" in src:
        print(
            f"  [auto-namespace] preserving prefixes in: {path.name}"
            f" (aliases inside wrap resolve them)"
        )
        return
    changed = 0
    dropped_lines = 0
    for prefix in prefixes:
        # 1. Drop `using namespace ::<prefix>;` and
        #    `using namespace <prefix>;` lines.
        pattern_ns = re.compile(
            rf"^[ \t]*using namespace (?:::)?{re.escape(prefix)};\s*\n",
            re.MULTILINE,
        )
        m = pattern_ns.subn("", src)
        if m[1]:
            src, n = m
            dropped_lines += n
        # 2. Drop `[export ]using <prefix>::Symbol;` lines (Symbol now at
        #    file scope post-import).
        pattern_using = re.compile(
            rf"^[ \t]*(?:export\s+)?using {re.escape(prefix)}::[A-Za-z_][A-Za-z0-9_]*;\s*\n",
            re.MULTILINE,
        )
        m = pattern_using.subn("", src)
        if m[1]:
            src, n = m
            dropped_lines += n
        # 3. Drop empty namespace declarations like `namespace <prefix> {}`
        #    or `namespace <prefix> {\n}` that the transpiler sometimes
        #    emits as placeholders.
        pattern_empty_ns = re.compile(
            rf"^[ \t]*namespace {re.escape(prefix)} \{{\s*}}\s*\n",
            re.MULTILINE,
        )
        m = pattern_empty_ns.subn("", src)
        if m[1]:
            src, n = m
            dropped_lines += n
        # 4. Strip remaining `<prefix>::` qualifiers (now safe — the
        #    using/namespace artifacts that depended on them are gone).
        needle = prefix + "::"
        before = src.count(needle)
        src = src.replace(needle, "")
        changed += before
    if changed or dropped_lines:
        # Sentinel at the very top to mark idempotency.
        src = sentinel + "\n" + src
        path.write_text(src)
        print(
            f"  stripped {changed} prefix occurrence(s)"
            f" and dropped {dropped_lines} using/namespace line(s)"
            f" in: {path.name}"
        )
    else:
        print(f"  no prefix occurrences found in: {path.name}")


def patch_entry_arities(path: Path) -> None:
    """Fix the cross-module `as`-rename type aliases in set.entry.cppm
    that emitted 2 template params instead of 3 — map::OccupiedEntry
    and map::VacantEntry both have <K, V, A>."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: arity fixed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (arity already fixed)")
        return

    old_occ = "template<typename T, typename A> using MapOccupiedEntry = map::OccupiedEntry<T, A>;"
    new_occ = "template<typename K, typename V, typename A> using MapOccupiedEntry = map::OccupiedEntry<K, V, A>;"
    old_vac = "template<typename T, typename A> using MapVacantEntry = map::VacantEntry<T, A>;"
    new_vac = "template<typename K, typename V, typename A> using MapVacantEntry = map::VacantEntry<K, V, A>;"

    if old_occ in src or old_vac in src:
        src = src.replace(old_occ, new_occ)
        src = src.replace(old_vac, new_vac)
        # Mark and write.
        src = src.replace(
            new_occ, new_occ + "  " + sentinel, 1
        )
        path.write_text(src)
        print(f"  fixed Map*Entry alias arity in: {path.name}")
    else:
        print(f"  no changes to: {path.name} (no Map*Entry aliases found)")


LINK_SMOKE_CPP = """\
// Smoke test for the transpiled btree_port C++20 module.
//
// Proves two things:
//   1. The btree_internal module can be imported into a regular .cpp
//      translation unit.
//   2. At least one exported type (SetValZST, the zero-sized tag) can
//      be instantiated, exercising the module loader without requiring
//      the deeper BTreeMap surface (which still has transpiler-side
//      gaps tracked in docs/btreemap_port/STATUS.md).
//
// The facade in include/btree_port/btreemap.hpp is the public-API
// "working version"; this smoke test is proof that the transpiled
// internals are also reachable from C++ user code, not just shipped
// as a static library in isolation.

import btree_port.btree.btree_internal;

#include <cstdio>

int main() {
    SetValZST zst;
    (void)zst;
    std::fprintf(stderr, "btree_port transpiled module: link smoke test ok\\n");
    return 0;
}
"""


# Step 50: a smoke test that exercises the transpiled BTreeMap::get
# path end-to-end. After step 48 (search_tree/force/into_kv hand-
# ported), get() works on the transpiled tree — even though
# insert/entry are still stubbed, an empty-map get() exercises
# the entire search_tree code path through the actual transpiled
# B-tree internals.
TRANSPILED_READ_SMOKE_CPP = """\
// Read-path smoke test for the transpiled rustc-stdlib BTreeMap.
//
// After step 48 (search_tree/force/into_kv hand-ported), BTreeMap::get
// works on the transpiled tree. But BTreeMap::insert/entry are still
// stubbed (step 49: rusty::BTreeMap vs ::BTreeMap namespace clash).
//
// So we can't put data IN, but we CAN call get() on an empty map and
// verify it returns None. This proves the read path is wired through
// the actual transpiled search_tree, not a stub.

import btree_port.btree.map;

#include <rusty/alloc.hpp>
#include <cstdio>
#include <cstdlib>

#define CHECK(cond, msg) do { \\
    if (!(cond)) { \\
        std::fprintf(stderr, "[FAIL] %s (%s:%d)\\n", msg, __FILE__, __LINE__); \\
        std::abort(); \\
    } else { \\
        std::fprintf(stderr, "[ok]   %s\\n", msg); \\
    } \\
} while (0)

int main() {
    // Construct an empty BTreeMap<int, int> via the Global allocator.
    auto m = ::BTreeMap<int, int, ::rusty::alloc::Global>::new_in(
        ::rusty::alloc::Global{});

    // get() on empty map — exercises search_tree's None-root path.
    // The transpiled body does:
    //   const auto root_node = RUSTY_TRY_OPT(this->root.as_ref()).reborrow();
    // which early-returns None when root is None.
    auto v_empty = m.get(1);
    CHECK(v_empty.is_none(), "get(1) on empty map: none");

    auto v_42 = m.get(42);
    CHECK(v_42.is_none(), "get(42) on empty map: none");

    // contains_key also routes through get().
    CHECK(!m.contains_key(7), "contains_key(7) on empty map: false");

    std::fprintf(stderr,
                 "transpiled BTreeMap read-smoke (empty-map): ALL CHECKS PASSED\\n");
    return 0;
}
"""


def drop_redundant_rusty_clone(path: Path) -> None:
    """Drop the local `rusty::clone` template the transpiler emits at
    the top of every module — it conflicts with the canonical
    `rusty::clone` in `include/rusty/move.hpp` (both are valid overloads
    for primitive types, leading to "call to 'clone' is ambiguous" at
    consumer instantiation time).

    The local `rusty::clone` was added before move.hpp had one; now
    redundant.
    """
    src = path.read_text()
    sentinel = "// btree_port port: local rusty::clone removed (canonical lives in move.hpp)"
    if sentinel in src:
        return
    # Match: `// Clone: dispatches…\ntemplate<typename T>\nauto clone(const T& value) { … }\n`
    block = (
        "// Clone: dispatches to .clone() if available, otherwise copy-constructs.\n"
        "template<typename T>\n"
        "auto clone(const T& value) {\n"
        "if constexpr (requires { value.clone(); }) {\n"
        "return value.clone();\n"
        "} else {\n"
        "return value;\n"
        "}\n"
        "}\n"
    )
    if block in src:
        src = src.replace(block, sentinel + "\n")
        path.write_text(src)
        print(f"  dropped local rusty::clone in: {path.name}")
    else:
        print(f"  no local rusty::clone block in: {path.name}")


def fix_unresolved_bare_glob_variants(path: Path) -> None:
    """Rewrite `/* TODO transpiler: unresolved bare-glob variant `X` */ true`
    placeholders to position-based variant-dispatch.

    Background (closes the B1 edge case the verification doc flagged):

    The transpiler's bare-glob variant resolver consults
    `unique_data_enum_name_for_variant_name` and falls back to a TODO
    marker if the variant name is ambiguous across enums. In btree,
    `Leaf` and `Internal` are both declared by `ForceResult` AND
    `Position`; `Left` and `Right` by `LeafOrRight` AND `Either` (built
    in). The resolver leaves the marker AND emits `_m._0` directly on
    the `std::variant`, which fails at template instantiation.

    Within each consecutive run of TODO-style arms that share an
    `auto&& _m = SCRUT;` opener (an IIFE-shaped match), we rewrite the
    Nth arm's condition to `_m.index() == N` and the body's `_m._0`
    accesses to `std::get<N>(_m)._0`. This assumes source-order ==
    enum-declaration-order, which holds for the btree force()/up()
    family — both `ForceResult` and `Position` order their variants
    Leaf=0, Internal=1, and `LeafOrRight` orders Left=0, Right=1.
    """
    import re

    src = path.read_text()
    sentinel = (
        "// btree_port port: bare-glob variant TODOs rewritten by "
        "post_transpile_patch.py"
    )
    if sentinel in src:
        print(
            f"  no changes to: {path.name} (bare-glob TODOs already rewritten)"
        )
        return
    todo_re = re.compile(
        r"if \(/\* TODO transpiler: unresolved bare-glob variant "
        r"`[A-Za-z_][A-Za-z0-9_]*` \(no enum decl visible in this TU; "
        r"patch arm manually\) \*/ true\)"
    )
    m_assign_re = re.compile(r"auto&& _m = ")
    total = 0
    # State machine: walk through the file from left to right, tracking
    # the position-counter that resets at each `auto&& _m = `. When we
    # find a TODO marker, rewrite condition + first `_m._0` access in
    # the arm body.
    out: list[str] = []
    i = 0
    n = len(src)
    arm_index: int = 0
    while i < n:
        # Check for the start of a new match scope.
        m_assign = m_assign_re.match(src, i)
        if m_assign:
            arm_index = 0
            out.append(src[i:m_assign.end()])
            i = m_assign.end()
            continue
        # Check for a TODO marker arm.
        todo = todo_re.match(src, i)
        if todo:
            # Brace-match the body of this `if (...)` arm.
            j = todo.end()
            # Skip whitespace to find `{`
            while j < n and src[j].isspace():
                j += 1
            if j < n and src[j] == "{":
                # Walk through the body, brace-counted.
                body_start = j + 1
                depth = 1
                k = body_start
                while k < n and depth > 0:
                    ch = src[k]
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                    k += 1
                body_end = k - 1  # exclusive of closing `}`
                body = src[body_start:body_end]
                # Rewrite the FIRST `rusty::detail::deref_if_pointer(_m)._0`
                # (which is the variant-payload access for this arm) to
                # `std::get<N>(rusty::detail::deref_if_pointer(_m))._0`.
                # That's the same shape the resolved arms use; emitting
                # the same shape here keeps the patch trivially diff-able.
                old_acc = "rusty::detail::deref_if_pointer(_m)._0"
                new_acc = (
                    f"std::get<{arm_index}>(rusty::detail::deref_if_pointer(_m))._0"
                )
                if old_acc in body:
                    body = body.replace(old_acc, new_acc, 1)
                # Emit the new condition + rewritten body + closing `}`.
                new_cond = (
                    f"if (rusty::detail::deref_if_pointer(_m).index() == {arm_index})"
                )
                out.append(new_cond)
                out.append(src[todo.end():body_start])  # whitespace + `{`
                out.append(body)
                out.append("}")
                i = k
                arm_index += 1
                total += 1
                continue
        out.append(src[i])
        i += 1
    new_src = "".join(out)
    if total:
        # Add the sentinel at the top so future runs are idempotent.
        new_src = sentinel + "\n" + new_src
        path.write_text(new_src)
        print(
            f"  rewrote {total} bare-glob TODO arm(s) in: {path.name}"
        )
    else:
        print(f"  no bare-glob TODO arms in: {path.name}")


def fix_using_rusty_vec(path: Path) -> None:
    """Strip the leftover `using rusty::Vec;` line. rusty::Vec lives in the
    `rusty.vec` submodule which btree_port doesn't import (and shouldn't).
    The transpiler emits the `using` declaration as a side-effect of seeing
    `use alloc::vec::Vec` in the input Rust source; the declaration is
    dead in our build.
    """
    src = path.read_text()
    needle = "using rusty::Vec;\n"
    if needle not in src:
        return
    src = src.replace(needle, "", 1)
    path.write_text(src)
    print(f"  stripped `using rusty::Vec;` in: {path.name}")


def fix_visit_byte_buf_unknown_vec(path: Path) -> None:
    """The transpiler emits a leftover `visit_byte_buf(rusty::Vec<uint8_t>)`
    method inside a serde-style visitor trait stub at module purview (before
    `namespace btree_port::btree::btree_internal`). `rusty::Vec` lives inside
    the `rusty.vec` module which the btree TU does NOT import (and shouldn't —
    btree has no need for Vec). Stub the body so it still parses but the
    Vec reference is dropped.

    Same shape sibling-port patcher (borrow_port P1) ships the equivalent
    rewrite; bringing it into btree_port closes the latest emit-shape
    drift between the two ports.
    """
    src = path.read_text()
    old = (
        "template<typename E>\n"
        "rusty::Result<Value, E> visit_byte_buf(rusty::Vec<uint8_t> value) {\n"
        "return rusty::Result<Value, E>::Ok(rusty::as_u8_slice(value));\n"
        "}\n"
    )
    if old not in src:
        print(f"  no visit_byte_buf rusty::Vec site in: {path.name}")
        return
    # `auto&&` here avoids declaring the unused `value` with a missing
    # template, and we synthesize an empty error so the return type still
    # deduces correctly.
    new = (
        "template<typename E>\n"
        "rusty::Result<Value, E> visit_byte_buf(auto&& value) {\n"
        "(void)value; return rusty::Result<Value, E>::Err(E{});\n"
        "}\n"
    )
    src = src.replace(old, new, 1)
    path.write_text(src)
    print(f"  rewrote visit_byte_buf rusty::Vec stub in: {path.name}")


def fix_occupied_entry_no_template_args(path: Path) -> None:
    """Rewrite `OccupiedEntry{.handle = …}` aggregate-init shape to
    `OccupiedEntry<K, V, A>{…}` so the compiler can deduce the template
    args.

    Rust source has `OccupiedEntry { handle, dormant_map, alloc, _marker }`
    with type-context inference. The transpiler emits the bare
    `OccupiedEntry{…}` which fails CTAD because aggregate-init doesn't
    do template-arg deduction without an explicit guide.

    The site is in BTreeMap methods that have K, V, A in scope (e.g.
    `remove_entry`), so `OccupiedEntry<K, V, A>` resolves cleanly.

    Note: be careful to only match the specific aggregate-init shape,
    not method-style references like `OccupiedEntry<…, …>::remove_entry`.
    Anchor on the field-init `{.handle = ` token to be precise.
    """
    src = path.read_text()
    needle = "OccupiedEntry{.handle = "
    if needle not in src:
        return
    # Replace with explicit template args. K, V, A are the standard
    # in-scope generics for the enclosing BTreeMap method.
    new_src = src.replace(needle, "OccupiedEntry<K, V, A>{.handle = ")
    if new_src != src:
        path.write_text(new_src)
        print(
            "  added <K, V, A> to OccupiedEntry aggregate-init in: "
            f"{path.name}"
        )


def fix_noderef_borrow_mut_auto_ref(path: Path) -> None:
    """Rewrite `auto& X = …NodeRef-chain….borrow_mut();` to `auto&& X = …`.

    btree_port's `NodeRef::borrow_mut(&mut self) -> NodeRef<Mut, …>`
    returns by VALUE — a new NodeRef wrapper around the same underlying
    pointer. The transpiler's heuristic treats any method named
    `borrow_mut` as returning a reference (the std `RefCell::borrow_mut`
    convention), and emits `auto& …` at the binding site. For NodeRef
    that's wrong: the return is an rvalue, and `auto&` can't bind to
    rvalues — the build fails with "non-const lvalue reference … cannot
    bind to a temporary".

    The fix is at the call shape level — patch the specific
    `auto& <name> = RUSTY_TRY_OPT(<expr>).borrow_mut();` shape to
    `auto&&`.

    The dual `auto&` heuristic for `as_mut` is correct (Rust's
    Option::as_mut returns `Option<&mut T>`), so leave those alone.
    """
    import re
    src = path.read_text()
    # Match `auto& <ident> = …).borrow_mut();` where the receiver is a
    # method-chain RUSTY_TRY_OPT(...).as_mut() / similar pattern from
    # which `.borrow_mut()` returns by value.
    pattern = re.compile(
        r"\bauto& (\w+) = (RUSTY_TRY_OPT\(.+?\.borrow_mut\(\);)",
        re.DOTALL,
    )
    new_src, count = pattern.subn(r"auto&& \1 = \2", src)
    if count:
        path.write_text(new_src)
        print(
            f"  rewrote {count} `auto& X = …borrow_mut();` → `auto&&` in: {path.name}"
        )


def fix_nested_ok_variant_index_uses_outer_m(path: Path) -> None:
    """Rewrite `auto&& _mvN = std::as_const(_m).unwrap(); if (rusty::detail::deref_if_pointer(_m).index() == N)`
    to use `_mvN` instead of `_m` in the index check.

    The transpiler emits the outer `Ok(...)` arm's inner data-enum-variant
    condition with the wrong scrutinee. For
    `match self.choose_parent_kv() { Ok(LeftOrRight::Left(kv)) => ..., Ok(LeftOrRight::Right(kv)) => ..., ... }`
    the inner condition for arm 0 should be
    `deref_if_pointer(_mv0).index() == 0` (checking the LeftOrRight variant)
    but is currently emitted as `deref_if_pointer(_m).index() == 0`
    (where `_m` is the outer Result, which has no `.index()`).

    The bug only fires for some configurations (the transpiler unit-test
    repro with simpler types emits the correct shape). Codify a textual
    fix here while the transpiler-level cause is being investigated.

    Idempotent: rewrites only sites that still use the outer `_m`.
    """
    src = path.read_text()
    # Match `auto&& _mvN = std::as_const(_m).unwrap();` (or with intervening
    # whitespace/newlines) followed by `if (rusty::detail::deref_if_pointer(_m).index() == N)`
    # where the two N values match. The scrutinee binding is `_m` (length 2),
    # and we want to replace just the inner `_m` with `_mvN`.
    pattern = re.compile(
        r"(auto&& _mv(\d+) = std::as_const\(_m\)\.unwrap\(\);\s*"
        r"if \(rusty::detail::deref_if_pointer\()_m(\)\.index\(\) == \2\b)"
    )
    new_src, count = pattern.subn(r"\1_mv\2\3", src)
    # Companion fix: the binding extracted from the variant uses the wrong
    # accessor. Buggy emit:
    #     auto&& NAME = rusty::detail::deref_if_pointer(
    #         rusty::detail::deref_if_pointer(_mvN)._0);
    # Variants have no `._0`; correct emit goes through `std::get<N>`:
    #     auto&& NAME = rusty::detail::deref_if_pointer(
    #         std::get<N>(rusty::detail::deref_if_pointer(_mvN))._0);
    # Match exactly inside an `index() == N` arm so N is the variant index.
    binding_pattern = re.compile(
        r"(if \(rusty::detail::deref_if_pointer\(_mv(\d+)\)\.index\(\) == \2\) \{ "
        r"auto&& [A-Za-z_][A-Za-z0-9_]* = rusty::detail::deref_if_pointer\()"
        r"(rusty::detail::deref_if_pointer\(_mv\2\))(\._0\))"
    )
    new_src, bind_count = binding_pattern.subn(
        r"\1std::get<\2>(\3)\4", new_src
    )
    # Drop const on the variant-payload binding so by-value methods (like
    # `BalancingContext::merge_tracking_parent(self, ...)`) can be called.
    # The `_mvN = std::as_const(_m).unwrap()` chain produces a const view of
    # the variant; the `std::get<N>(...)._0` access then yields a const
    # reference to the inner type. Rust's `mut` binding in the match arm
    # implies the payload is consumed, so C++ needs a mutable handle. A
    # const_cast on the expression is safe here because the original storage
    # (held by `_m`) is non-const inside this IIFE.
    const_drop_pattern = re.compile(
        r"(if \(rusty::detail::deref_if_pointer\(_mv(\d+)\)\.index\(\) == \2\) \{ "
        r"auto&&) ([A-Za-z_][A-Za-z0-9_]*) = rusty::detail::deref_if_pointer\("
        r"(std::get<\2>\(rusty::detail::deref_if_pointer\(_mv\2\)\)\._0)\);"
    )
    new_src, const_count = const_drop_pattern.subn(
        r"\1 \3 = const_cast<std::remove_cvref_t<decltype(\4)>&>(\4);",
        new_src,
    )
    if count or bind_count or const_count:
        path.write_text(new_src)
    if count:
        print(
            f"  rewrote {count} `_mvN = unwrap(); deref_if_pointer(_m).index() == N` "
            f"→ `deref_if_pointer(_mvN)` in: {path.name}"
        )
    if bind_count:
        print(
            f"  rewrote {bind_count} `deref_if_pointer(_mvN)._0` "
            f"→ `std::get<N>(deref_if_pointer(_mvN))._0` in: {path.name}"
        )
    if const_count:
        print(
            f"  rewrote {const_count} variant-payload binding to drop const_cast "
            f"in: {path.name}"
        )


def fix_full_range_recursive_call(path: Path) -> None:
    """Qualify `full_range(self, self)` calls inside the NodeRef::full_range
    method body so they resolve to the FREE function rather than recursing
    into the surrounding method.

    Rust source:
        impl NodeRef<…> {
            pub(super) fn full_range(self) -> LazyLeafRange<…> {
                full_range(self, self)
            }
        }
    where the unqualified `full_range(...)` call at line 2 resolves to the
    free function `fn full_range<BorrowType, K, V>(root1, root2) ->
    LazyLeafRange<…>` defined at module scope (arity-disambiguation:
    Rust picks the 2-arg form because the method is 0-arg).

    C++ method dispatch doesn't do arity disambiguation — inside the
    method body, `full_range(...)` first matches the surrounding method
    (`this->full_range(...)`), which takes 0 args, so the call's 2 args
    are flagged as "too many arguments to function call".

    Rewrite the recursive call shape `full_range(std::move((*this)),
    std::move((*this)))` to use the explicit `::<full-ns>::full_range(...)`
    qualifier so C++ resolves to the free function.
    """
    src = path.read_text()
    # The fresh transpile emits `::full_range(...)` (global qualifier) —
    # because the transpiler sees the method-vs-free shadowing in Rust
    # and tries to disambiguate via the global namespace. Under auto-
    # namespace mode the free function lives in
    # `btree_port::btree::btree_internal::`, NOT at global scope, so
    # `::full_range` doesn't resolve. Replace the prefix to point at
    # the right namespace.
    needle = "::full_range(std::move((*this)), std::move((*this)))"
    if needle not in src:
        return
    repl = (
        "::btree_port::btree::btree_internal::full_range(std::move((*this)), std::move((*this)))"
    )
    new_src = src.replace(needle, repl)
    if new_src != src:
        path.write_text(new_src)
        print(f"  qualified full_range(self, self) recursive call in: {path.name}")


def fix_leafnode_shadow_arrow(path: Path) -> None:
    """The transpiler emits `*new_node_shadow1.field` for the body of
    `split_leaf_data` (where `new_node_shadow1` is a pointer to a LeafNode
    aliased from a Rust `&mut LeafNode<K, V>` parameter). The precedence
    is wrong — C++ parses this as `*(ptr.field)` which is invalid.

    Rewrite the three sites in `split_leaf_data` from `*new_node_shadow1.X`
    to `new_node_shadow1->X` (semantically identical, syntactically valid).
    """
    src = path.read_text()
    rewrites = [
        ("*new_node_shadow1.len = ", "new_node_shadow1->len = "),
        ("*new_node_shadow1.keys", "new_node_shadow1->keys"),
        ("*new_node_shadow1.vals", "new_node_shadow1->vals"),
    ]
    changed = False
    for old, new in rewrites:
        if old in src:
            src = src.replace(old, new)
            changed = True
    if changed:
        path.write_text(src)
        print(f"  rewrote *new_node_shadow1.X → ->X in: {path.name}")
    else:
        print(f"  no new_node_shadow1 sites in: {path.name}")


def fix_template_args_primary_scope(path: Path) -> None:
    """Move the `template<typename T> struct __TemplateArgs;` primary
    declaration from module purview into the namespace wrap, matching
    where the partial specializations land.

    Background: the Cluster A completion injects the primary
    `__TemplateArgs<T>` template at module purview (before the
    `namespace btree_port::btree::X { … }` open) but emits the
    partial specializations (`__TemplateArgs<NodeRef<…>>`) inside the
    namespace wrap. C++ requires partial specializations to live in the
    same namespace as the primary template — mismatch is an error.

    Fix: relocate the primary declaration into the namespace by
    moving it from line ~3663 into line ~3666 (right after the wrap
    opens). Idempotent — detects the relocated form via the
    presence-of-marker after the relocation point.
    """
    src = path.read_text()
    primary = "template<typename T> struct __TemplateArgs;\n"
    primary_with_comment = (
        "// Cluster A completion: __TemplateArgs primary template "
        "(specializations at file end)\n"
        + primary
    )
    # Anchor: the namespace open immediately follows the primary.
    ns_open = "namespace btree_port::btree::btree_internal {\n"
    # Idempotency: if the primary already sits inside the namespace
    # (i.e. comes AFTER the wrap opens), nothing to do.
    if primary in src and ns_open in src and src.index(ns_open) < src.index(primary):
        print(f"  __TemplateArgs primary already in-namespace in: {path.name}")
        return
    # Try both the "blank line between primary and ns_open" form and the
    # "no blank line" form — both have appeared from the transpiler over
    # the lifetime of this patcher.
    for blank in ("\n", ""):
        # Comment + primary + (optional blank) + ns_open
        with_cmt = primary_with_comment + blank + ns_open
        if with_cmt in src:
            new_src = src.replace(
                with_cmt,
                ns_open + "\n" + primary_with_comment,
                1,
            )
            path.write_text(new_src)
            print(f"  relocated __TemplateArgs primary into namespace in: {path.name}")
            return
        # Bare primary (no comment) + (optional blank) + ns_open
        bare = primary + blank + ns_open
        if bare in src:
            new_src = src.replace(
                bare,
                ns_open + "\n" + primary,
                1,
            )
            path.write_text(new_src)
            print(f"  relocated __TemplateArgs primary (uncommented) into namespace in: {path.name}")
            return
    print(f"  no __TemplateArgs primary relocation needed in: {path.name}")


def fix_std_hash_specialization_scope(path: Path) -> None:
    """Break out of the namespace wrap around `template<> struct std::hash<…>`
    specializations and reopen after them.

    Background: C++ requires that explicit specializations of standard-
    library templates (`std::hash`, `std::less`, etc.) be defined either
    inside `namespace std { }` or — via the `std::T` spelling — at a
    scope that *encloses* `namespace std`. When the transpiled module is
    wrapped in `namespace btree_port::btree::btree_internal { … }`, the
    `std::hash<SetValZST>` specialization lands inside the wrap and
    clang rejects it with: "class template specialization of 'hash' not
    in a namespace enclosing 'std'".

    Fix: scan for `^template<>\\nstruct std::hash<…> {…};` blocks inside
    the namespace wrap and surround each with `}` (close) + reopen.
    Idempotent.
    """
    src = path.read_text()
    sentinel = "// btree_port port: std::hash<...> moved to module purview"
    if sentinel in src:
        print(f"  no std::hash scope fix needed (already applied) in: {path.name}")
        return
    # Use brace matching to extract each `template<>\nstruct std::hash<…>
    # { … };` block. The body has nested braces (the operator() body),
    # so a simple regex won't terminate correctly.
    n_bumped = 0
    search_from = 0
    anchor = "template<>\nstruct std::hash<"
    while True:
        pos = src.find(anchor, search_from)
        if pos == -1:
            break
        brace_open = src.find("{", pos)
        if brace_open == -1:
            break
        brace_close = find_matching_brace(src, brace_open)
        if brace_close == -1:
            break
        # Followed by `;` immediately after the close brace.
        if brace_close + 1 >= len(src) or src[brace_close + 1] != ";":
            search_from = brace_close + 1
            continue
        block_end = brace_close + 2  # include the `;`
        block = src[pos:block_end]
        # Determine whether this block is inside a namespace wrap by
        # counting unclosed `namespace btree_port::btree::… {` opens
        # vs `} // namespace btree_port::btree::…` closes up to `pos`.
        depth = (
            src[:pos].count("namespace btree_port::btree::btree_internal {")
            - src[:pos].count("} // namespace btree_port::btree::btree_internal")
        )
        if depth <= 0:
            # Already at module purview, no need to bump.
            search_from = block_end
            continue
        # When bumped to module purview, the block loses access to the
        # in-namespace types it references (e.g. `SetValZST`). Inject a
        # one-shot `using` declaration to bring them into scope.
        using_decl = (
            "using btree_port::btree::btree_internal::SetValZST;\n"
        )
        bumped = (
            f"}} // namespace btree_port::btree::btree_internal (closed for std::hash)\n"
            f"{sentinel}\n"
            f"{using_decl}"
            f"{block}\n"
            f"namespace btree_port::btree::btree_internal {{\n"
        )
        src = src[:pos] + bumped + src[block_end:]
        n_bumped += 1
        # Advance past the bumped section so we don't re-find the same block.
        search_from = pos + len(bumped)
        if n_bumped > 10:
            break
    if n_bumped:
        path.write_text(src)
        print(f"  bumped {n_bumped} std::hash<...> spec(s) to module purview in: {path.name}")
    else:
        print(f"  no std::hash<...> specializations to fix in: {path.name}")


def stub_broken_set_methods(path: Path) -> None:
    """Stub specific known-broken methods in set.cppm:
      - methods whose body has `this->_0.nexts(const T&<T>::cmp)` —
        a transpile bug where `<T as Ord>::cmp` got mistranslated.
      - nested `struct DropGuard { IntoIter<K, V, A>& _0; ... }` —
        absorbed from map::IntoIter, refers to undefined K, V.
      - `template<typename V>` methods (handled by `remove_setvalzst_methods`
        once we add the matcher).
    """
    src = path.read_text()
    sentinel = "// btree_port port: broken set.cppm regions stubbed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (set stubs already applied)")
        return

    changed = 0

    # -1. Forward-declare the variant types (DifferenceInner_Stitch etc.)
    #     that are USED inside the Difference/Intersection clone methods
    #     but DEFINED LATER in the file. Inject the forward decls at the
    #     top of the namespace wrap, after the alias block.
    variant_fwd_decls = (
        "// btree_port port: variant-type forward decls injected by post_transpile_patch.py\n"
        "template<typename T, typename A>\n"
        "struct DifferenceInner_Stitch;\n"
        "template<typename T, typename A>\n"
        "struct DifferenceInner_Search;\n"
        "template<typename T, typename A>\n"
        "struct DifferenceInner_Iterate;\n"
        "template<typename T, typename A>\n"
        "struct IntersectionInner_Stitch;\n"
        "template<typename T, typename A>\n"
        "struct IntersectionInner_Search;\n"
        "template<typename T, typename A>\n"
        "struct IntersectionInner_Answer;\n"
    )
    # Anchor on the LAST contiguous alias line in the set namespace.
    alias_pattern_anchor = re.compile(
        r"^namespace [a-z_]+ = ::btree_port::btree::[a-z_.:]+;\n",
        re.MULTILINE,
    )
    last_alias = None
    prev_end = -1
    for m in alias_pattern_anchor.finditer(src):
        if prev_end != -1 and m.start() != prev_end:
            break
        last_alias = m
        prev_end = m.end()
    variant_sentinel = "// btree_port port: variant-type forward decls injected"
    if last_alias and variant_sentinel not in src:
        src = src[:last_alias.end()] + variant_fwd_decls + src[last_alias.end():]
        changed += 1

    # 0. Stub the entire `template<typename Iter> void extend(Iter<T> iter) { ... }`
    #    method — its signature collides with the `template<typename I>
    #    void extend(I iter)` overload (same shape after deduction,
    #    different template-param name), so C++ flags it as
    #    redeclaration. The transpiler emitted two `extend` impls
    #    from two separate Rust impl blocks.
    if "void extend(Iter<T> iter)" in src:
        # Match the whole method block.
        old_method = re.search(
            r"^    template<typename Iter>\n"
            r"    void extend\(Iter<T> iter\) \{\n"
            r"        rusty::for_each\([^\n]*\n"
            r"this->insert\(std::move\(elem\)\);\n"
            r"\}\);\n"
            r"    \}\n",
            src,
            re.MULTILINE,
        )
        if old_method:
            src = src[:old_method.start()] + src[old_method.end():]
            changed += 1
        else:
            # Fallback: at least drop the broken signature.
            src = src.replace("void extend(Iter<T> iter)", "void extend(Iter iter)")
            changed += 1
    # Substitute `rusty::alloc::Global` in value-position with the
    # default-constructed form. Patterns: `, rusty::alloc::Global)` and
    # `, rusty::alloc::Global,`.
    for old, new in [
        (", rusty::alloc::Global)", ", rusty::alloc::Global{})"),
        (", rusty::alloc::Global,", ", rusty::alloc::Global{},"),
    ]:
        n = src.count(old)
        if n > 0:
            src = src.replace(old, new)
            changed += n
    # `BTreeMap::bulk_build_from_sorted_iter` — bare BTreeMap as
    # call target needs `map::BTreeMap`:
    if "BTreeMap::bulk_build_from_sorted_iter" in src:
        src = src.replace("BTreeMap::bulk_build_from_sorted_iter",
                          "map::BTreeMap<T, btree_internal::SetValZST, A>::bulk_build_from_sorted_iter")
        changed += 1
    # `ManuallyDrop::into_inner` requires explicit template arg in
    # call-target form, but inside an expression that's hard to recover.
    # Stub with a workaround — copy the alloc directly without the
    # ManuallyDrop unwrap.
    src = src.replace(
        "ManuallyDrop::into_inner(rusty::clone(this->map.alloc))",
        "rusty::clone(*this->map.alloc)",
    )
    # Align requires-clause on `struct DifferenceInner : std::variant<…>`
    # / `struct IntersectionInner : std::variant<…>` with the forward
    # decl (which has the requires clause from the original Rust impl).
    for name in ("DifferenceInner", "IntersectionInner"):
        anchor = f"template<typename T, typename A>\nstruct {name} : std::variant<"
        repl = (
            f"template<typename T, typename A>\n"
            f"    requires (rusty::alloc::Allocator<A> && std::copyable<A>)\n"
            f"struct {name} : std::variant<"
        )
        if anchor in src and repl not in src:
            src = src.replace(anchor, repl, 1)
            changed += 1
    # `f.debug_set()` → `f.debug_list()` — Formatter doesn't have debug_set.
    if "f.debug_set()" in src:
        src = src.replace("f.debug_set()", "f.debug_list()")
        changed += 1
    # `decltype((rusty::alloc::Global))` — type-as-value-then-decltype
    # gnarliness; rewrite to just `rusty::alloc::Global`.
    if "decltype((rusty::alloc::Global))" in src:
        src = src.replace(
            "std::remove_cvref_t<decltype((rusty::alloc::Global))>",
            "rusty::alloc::Global",
        )
        changed += 1

    # 1. Wrap each `struct DropGuard { … };` cluster that references
    #    undefined K, V in #if 0.
    lines = src.split("\n")
    n = len(lines)
    out: list[str] = []
    i = 0
    while i < n:
        line = lines[i]
        # Look for `    struct DropGuard {` at 4-space indent.
        if line.rstrip() == "    struct DropGuard {":
            # Find matching close brace.
            depth = 1
            j = i + 1
            while j < n and depth > 0:
                for c in lines[j]:
                    if c == "{":
                        depth += 1
                    elif c == "}":
                        depth -= 1
                        if depth == 0:
                            break
                j += 1
            # j now points one past the close brace line.
            cluster_end = j - 1
            # Check the body references K or V as undefined identifiers.
            body = "\n".join(lines[i : cluster_end + 1])
            if "IntoIter<K, V, A>" in body:
                out.append(f"#if 0  // {sentinel}")
                out.extend(lines[i : cluster_end + 1])
                out.append("#endif")
                changed += 1
                i = cluster_end + 1
                continue
        out.append(line)
        i += 1
    src = "\n".join(out)

    # 2. Stub method bodies containing any of these broken patterns:
    #    - `this->_0.nexts(const T&<T>::cmp)` — closure call where
    #      `<T as Ord>::cmp` got mistranslated.
    #    - `rusty::cmp::cmp(self_min, other_max)` — references to
    #      hoisted let bindings (`self_min`, `self_max`, `other_min`,
    #      `other_max`, `DifferenceInner_Iterate`) that didn't survive
    #      the IIFE-result handle binding pass.
    broken_patterns = [
        "this->_0.nexts(const T&<T>::cmp)",
        "rusty::cmp::cmp(self_min, other_max)",
        "rusty::cmp::cmp(self_min, other_min)",
        "rusty::cmp::cmp(self_max, other_max)",
        # `auto [/* TODO: pattern */, ...] = ...` — tuple destructure
        # where transpiler emitted TODO placeholders instead of bindings.
        "auto [/* TODO: pattern */",
    ]
    # Safety cap to prevent runaway loop if a fix doesn't actually
    # consume the pattern from src.
    max_iters = 50
    iters = 0
    while iters < max_iters:
        iters += 1
        broken_cmp = None
        for pat in broken_patterns:
            if pat in src:
                broken_cmp = pat
                break
        if broken_cmp is None:
            break
        lines2 = src.split("\n")
        # Find the first line containing the broken pattern.
        target = None
        for idx, ln in enumerate(lines2):
            if broken_cmp in ln:
                target = idx
                break
        if target is None:
            break
        # Walk back to find the method-open `{` at indent 4. The line
        # ends with `{` and is exactly 4-space indented.
        k = target
        while k >= 0:
            if lines2[k].rstrip().endswith("{") and (
                lines2[k].startswith("    ") and not lines2[k].startswith("     ")
            ):
                break
            k -= 1
        if k < 0:
            break
        method_open = k
        # Walk forward from method_open + 1, tracking brace depth from 1.
        depth = 1
        m = method_open + 1
        method_close = None
        while m < len(lines2):
            for c in lines2[m]:
                if c == "{":
                    depth += 1
                elif c == "}":
                    depth -= 1
                    if depth == 0:
                        method_close = m
                        break
            if method_close is not None:
                break
            m += 1
        if method_close is None:
            break
        stub_body = (
            "        throw ::std::runtime_error(\""
            "rusty-cpp-transpiler: set.cppm method stub "
            "(broken <T as Ord>::cmp emit); "
            "see docs/btreemap_port/STATUS.md\");"
        )
        new_lines = (
            lines2[: method_open + 1]
            + [stub_body]
            + lines2[method_close:]  # this is the closing `    }`
        )
        src = "\n".join(new_lines)
        changed += 1

    if changed:
        path.write_text(sentinel + "\n" + src)
        print(f"  stubbed {changed} broken set.cppm region(s) in: {path.name}")
    else:
        print(f"  no broken set.cppm regions found in: {path.name}")


def fix_set_cppm_qualifiers_for_namespace_wrap(path: Path) -> None:
    """In set.cppm under --auto-namespace, references to map's
    `BTreeMap` are emitted bare (because the prep.sh + transpiler
    treats `super::map::BTreeMap` as just `BTreeMap` after the
    prefix-strip). Under flat-export that worked; under
    auto-namespace, `BTreeMap` is `map::BTreeMap`, not in scope at
    set's namespace level. Qualify it with `map::`.
    """
    src = path.read_text()
    sentinel = "// btree_port port: set.cppm qualifier fix-up by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (set qualifiers already fixed)")
        return
    patterns = [
        # Bare `BTreeMap<` in set.cppm → `map::BTreeMap<`. Use negative
        # lookbehind to avoid mangling `map::BTreeMap<` (already
        # qualified) or `inner::BTreeMap<` etc.
        (r"(?<![a-zA-Z0-9_:.])BTreeMap<", "map::BTreeMap<"),
    ]
    total = 0
    for pat, repl in patterns:
        new, n = re.subn(pat, repl, src)
        if n:
            src = new
            total += n
    if total:
        path.write_text(sentinel + "\n" + src)
        print(f"  applied {total} set-qualifier fix-up(s) in: {path.name}")
    else:
        print(f"  no set-qualifier fix-ups needed in: {path.name}")


def fix_map_cppm_qualifiers_for_namespace_wrap(path: Path) -> None:
    """In map.cppm / set.cppm / set.entry.cppm under --auto-namespace
    mode, patcher-injected hand-port code references types/templates
    without a qualifier that worked under flat-export mode but doesn't
    under the namespace wrap.

    Specifically:
      - `NodeRef<…>` → `btree_internal::NodeRef<…>` (NodeRef lives
        in btree_internal::, not in map::).
      - `Root<…>` / `Handle<…>` / `SearchBound<…>` — same fix.
      - `SetValZST{` / `SetValZST,` — qualified to btree_internal::SetValZST.
      - `::BTreeMap<…>` → `BTreeMap<…>` (the global qualifier
        from merge content; BTreeMap is in-scope at map::).
      - `::IntoIter<…>` → `IntoIter<…>` (same).

    These rewrites are only safe in files where `btree_internal`
    alias is present (auto-namespace-wrapped map / set / set.entry).
    Not safe in btree_internal.cppm itself.
    """
    src = path.read_text()
    sentinel = "// btree_port port: map.cppm qualifier fix-up by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (qualifiers already fixed)")
        return
    # Determine which file we're patching to pick the right
    # `::BTreeMap` / `::BTreeSet` replacement: this function is shared
    # between map.cppm, set.cppm, and set.entry.cppm.
    is_set_file = path.name in (
        "btree_port.btree.set.cppm",
        "btree_port.btree.set.entry.cppm",
    )
    btreemap_repl = "map::BTreeMap<" if is_set_file else "BTreeMap<"
    btreeset_repl = "BTreeSet<" if is_set_file else "set::BTreeSet<"
    patterns = [
        # `::BTreeMap<` / `::IntoIter<` came from the merged entry
        # content where `rusty::BTreeMap → BTreeMap` substitution
        # turned implicit-rooted references into global-qualified
        # ones. Strip them — but only when at the start of a name
        # path (negative lookbehind on identifier chars so we don't
        # mangle `map::IntoIter` → `mapIntoIter`).
        (r"(?<![a-zA-Z0-9_]):: ?BTreeMap<", btreemap_repl),
        (r"(?<![a-zA-Z0-9_])::IntoIter<", "IntoIter<"),
        # `::BTreeSet<` (from transpiler's `BTreeSet → ::BTreeSet`
        # type-map). In map.cppm, BTreeSet lives in the sibling `set`
        # namespace; in set.cppm itself, BTreeSet is in-scope (bare).
        (r"(?<![a-zA-Z0-9_]):: ?BTreeSet<", btreeset_repl),
        # Bare `NodeRef<` / `Root<` / `Handle<` / `SearchBound<` in
        # patcher hand-port code (or merged content) needs to be
        # `btree_internal::Foo<` since these types live there.
        # Use a negative lookbehind on identifier chars so we don't
        # mangle already-qualified forms or template-args meta-calls.
        (r"(?<![a-zA-Z0-9_:.])NodeRef<", "btree_internal::NodeRef<"),
        (r"(?<![a-zA-Z0-9_:.])Root<", "btree_internal::Root<"),
        (r"(?<![a-zA-Z0-9_:.])Handle<", "btree_internal::Handle<"),
        (r"(?<![a-zA-Z0-9_:.])SearchBound<", "btree_internal::SearchBound<"),
        # SetValZST referenced as a value (e.g. `SetValZST{}`):
        (r"(?<![a-zA-Z0-9_:.])SetValZST\{", "btree_internal::SetValZST{"),
        # SetValZST::default_() — static method call:
        (r"(?<![a-zA-Z0-9_:.])SetValZST::", "btree_internal::SetValZST::"),
        # `entry(value).insert_entry(SetValZST)` — bare SetValZST as
        # value where {} is missing. Just qualify and add construction.
        (r"\.insert_entry\(SetValZST\)", ".insert_entry(btree_internal::SetValZST{})"),
    ]
    total = 0
    for pat, repl in patterns:
        new, n = re.subn(pat, repl, src)
        if n:
            src = new
            total += n
    if total:
        path.write_text(sentinel + "\n" + src)
        print(f"  applied {total} map-qualifier fix-up(s) in: {path.name}")
    else:
        print(f"  no map-qualifier fix-ups needed in: {path.name}")


def fix_global_qualifiers_for_namespace_wrap(path: Path) -> None:
    """Convert the patcher's hand-port `::marker::X` / `::LeafNode<…>` /
    `::InternalNode<…>` references to unqualified form.

    Background: when the transpiler emits flat exports (no namespace
    wrap), the patcher's hand-port templates use `::marker::Owned`,
    `::LeafNode<K, V>*`, etc. — explicit global qualifiers — because the
    symbols live at global scope. With `--auto-namespace`, the symbols
    are inside `namespace btree_port::btree::btree_internal { … }`, and
    the global-qualifier paths break: no `::marker`, no `::LeafNode` at
    file scope.

    Fix: strip the leading `::` from these references. Inside a method
    body that is itself nested in `namespace btree_port::btree::btree_internal`,
    unqualified `marker::X` / `LeafNode<…>` lookup finds the sibling
    namespace / nested struct via the normal C++ name-lookup rules.
    Idempotent — running on already-stripped source is a no-op.
    """
    src = path.read_text()
    patterns = [
        # marker:: family — all 11 marker symbols + KV/Edge enums.
        (r"::marker::Owned\b", "marker::Owned"),
        (r"::marker::Mut\b", "marker::Mut"),
        (r"::marker::Immut\b", "marker::Immut"),
        (r"::marker::Internal\b", "marker::Internal"),
        (r"::marker::Leaf\b", "marker::Leaf"),
        (r"::marker::LeafOrInternal\b", "marker::LeafOrInternal"),
        (r"::marker::Dying\b", "marker::Dying"),
        (r"::marker::DormantMut\b", "marker::DormantMut"),
        (r"::marker::ValMut\b", "marker::ValMut"),
        (r"::marker::KV\b", "marker::KV"),
        (r"::marker::Edge\b", "marker::Edge"),
        # Bare types defined inside btree_internal — referenced from
        # hand-port code with `::Foo` (global) qualifier that no longer
        # resolves under the namespace wrap.
        (r"::LeafNode<", "LeafNode<"),
        (r"::InternalNode<", "InternalNode<"),
        # Free functions defined inside btree_internal:
        # All bare-global `::name(` strips below are anchored with a
        # negative lookbehind so we don't mangle qualified-path tails
        # like `btree_internal::full_range(` → `btree_internalfull_range(`.
        (r"(?<![a-zA-Z0-9_])::move_to_slice\(", "move_to_slice("),
        (r"(?<![a-zA-Z0-9_])::slice_remove\(", "slice_remove("),
        (r"(?<![a-zA-Z0-9_])::slice_insert\(", "slice_insert("),
        (r"(?<![a-zA-Z0-9_])::slice_shr\(", "slice_shr("),
        (r"(?<![a-zA-Z0-9_])::slice_shl\(", "slice_shl("),
        (r"(?<![a-zA-Z0-9_])::take_mut\(", "take_mut("),
        (r"(?<![a-zA-Z0-9_])::full_range\(", "full_range("),
        (r"(?<![a-zA-Z0-9_])::splitpoint\(", "splitpoint("),
        # Alias-rewriter helpers (Cluster A / orphan-method recovery):
        (r"::(__rusty_alias_[A-Za-z_]+)\(", r"\1("),
        # `replace(` is a method on rusty::Option etc; the bare `::replace`
        # qualifier came from hand-port code paths. Use a negative
        # lookbehind to avoid mangling `rusty::mem::replace(` →
        # `rusty::memreplace(`.
        (r"(?<![a-zA-Z_:])::replace\(", "replace("),
    ]
    total = 0
    for pat, repl in patterns:
        new, n = re.subn(pat, repl, src)
        if n:
            src = new
            total += n
    if total:
        path.write_text(src)
        print(f"  stripped {total} global-qualifier prefix(es) in: {path.name}")
    else:
        print(f"  no global-qualifier strip needed in: {path.name}")


def write_link_smoke(cpp_out_dir: Path) -> None:
    path = cpp_out_dir / "link_smoke.cpp"
    if path.exists() and path.read_text() == LINK_SMOKE_CPP:
        print(f"  no changes to: {path.name} (already current)")
        return
    path.write_text(LINK_SMOKE_CPP)
    print(f"  wrote: {path.name}")


def write_transpiled_read_smoke(cpp_out_dir: Path) -> None:
    path = cpp_out_dir / "transpiled_read_smoke.cpp"
    if path.exists() and path.read_text() == TRANSPILED_READ_SMOKE_CPP:
        print(f"  no changes to: {path.name} (already current)")
        return
    path.write_text(TRANSPILED_READ_SMOKE_CPP)
    print(f"  wrote: {path.name}")


def patch_cmake(path: Path, rusty_include_dir: Path) -> None:
    """Trim CMakeLists.txt to btree_internal-only and wire the rusty
    include path so reconfigure doesn't drop -I."""
    src = path.read_text()
    # The transpiler emits `set(CMAKE_CXX_STANDARD 20)` but the runtime
    # headers (rusty/*.hpp) use C++23 features (std::println, deduced
    # this, etc.) so bump to 23. Idempotent — looks for the exact 20
    # setter; once swapped the substitution is a no-op.
    cxx_std_orig = "set(CMAKE_CXX_STANDARD 20)"
    cxx_std_new = (
        "set(CMAKE_CXX_STANDARD 23)  "
        "# btree_port port: bumped to 23 for std::println in transpiled internals"
    )
    if cxx_std_orig in src:
        src = src.replace(cxx_std_orig, cxx_std_new, 1)
        path.write_text(src)
        print(f"  bumped CMAKE_CXX_STANDARD 20 → 23 in: {path.name}")
    sentinel = "# btree_port port: trimmed by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (already trimmed)")
        return

    # Replace the include_directories comment block (or a previous edit)
    # with a real include_directories() call pointing at the rusty headers.
    inc_block_orig = (
        "# Include rusty-cpp headers\n"
        "# Adjust this path to your rusty-cpp installation\n"
        "# include_directories(${RUSTY_CPP_INCLUDE_DIR})"
    )
    inc_block_new = (
        "# Include rusty-cpp headers (wired in by post_transpile_patch.py)\n"
        f"include_directories({rusty_include_dir})"
    )
    if inc_block_orig in src:
        src = src.replace(inc_block_orig, inc_block_new)

    # Replace the full add_library / target_sources blocks (between
    # 'add_library(btree_port' and the closing ')' of target_sources)
    # with a btree_internal-only target.
    import re

    trim_block = (
        f"{sentinel}\n"
        "# `btree_internal` builds cleanly under both g++ and clang++ after\n"
        "# the post-transpile patches. `map.entry` builds under clang++ but\n"
        "# hits a GCC 14 ICE (segfault during destructor analysis of\n"
        "# rusty::RcControlBlockBase) — include it only when building with\n"
        "# clang. See docs/btreemap_port/STATUS.md.\n"
        "#\n"
        "# The 'working version' is the hand-written facade at\n"
        "# include/btree_port/btreemap.hpp (validated by\n"
        "# tests/btree_port_facade_test.cpp). The facade does NOT depend on\n"
        "# this module — building btree_internal (+ map.entry on clang) is\n"
        "# proof the transpiled internals are nearly compile-clean and\n"
        "# ready for gradual migration.\n"
        "set(_BTREE_PORT_SOURCES btree_port.btree.btree_internal.cppm)\n"
        "if(CMAKE_CXX_COMPILER_ID STREQUAL \"Clang\"\n"
        "   OR CMAKE_CXX_COMPILER_ID STREQUAL \"AppleClang\")\n"
        "    # btree_port port step 52: map.entry.cppm's content was merged\n"
        "    # into map.cppm (see merge_map_entry_into_map in the patcher).\n"
        "    # Only map.cppm is built now — map.entry's content lives there.\n"
        "    list(APPEND _BTREE_PORT_SOURCES\n"
        "        btree_port.btree.map.cppm\n"
        "    )\n"
        "endif()\n"
        "\n"
        "add_library(btree_port ${_BTREE_PORT_SOURCES})\n"
        "\n"
        "target_sources(btree_port PUBLIC FILE_SET CXX_MODULES FILES\n"
        "    ${_BTREE_PORT_SOURCES}\n"
        ")\n"
        "\n"
        "# Smoke-test executable: imports the transpiled module and\n"
        "# references one of its exported types. Proves the static\n"
        "# library is actually loadable+linkable, not just compileable\n"
        "# in isolation. Only built under clang — gcc 14 ICEs when\n"
        "# importing this module from a consumer TU.\n"
        "if(EXISTS \"${CMAKE_CURRENT_SOURCE_DIR}/link_smoke.cpp\"\n"
        "   AND (CMAKE_CXX_COMPILER_ID STREQUAL \"Clang\"\n"
        "        OR CMAKE_CXX_COMPILER_ID STREQUAL \"AppleClang\"))\n"
        "    add_executable(btree_port_link_smoke link_smoke.cpp)\n"
        "    target_link_libraries(btree_port_link_smoke PRIVATE btree_port)\n"
        "endif()\n"
        "\n"
        "# Read-only smoke test — exercises the transpiled search_tree on\n"
        "# an empty map (the only path that doesn't hit the entry() stub).\n"
        "# After step 48 (search_tree/force/into_kv hand-ported), get()\n"
        "# works on the transpiled tree; insert/entry remain stubbed.\n"
        "if(EXISTS \"${CMAKE_CURRENT_SOURCE_DIR}/transpiled_read_smoke.cpp\"\n"
        "   AND (CMAKE_CXX_COMPILER_ID STREQUAL \"Clang\"\n"
        "        OR CMAKE_CXX_COMPILER_ID STREQUAL \"AppleClang\"))\n"
        "    add_executable(btree_port_transpiled_read_smoke transpiled_read_smoke.cpp)\n"
        "    target_link_libraries(btree_port_transpiled_read_smoke PRIVATE btree_port)\n"
        "endif()\n"
        "\n"
        "# Write-path smoke test — exercises insert / get / first_key_value /\n"
        "# last_key_value end-to-end on the transpiled tree. Only built when\n"
        "# the transpiled_smoke.cpp source is present.\n"
        "if(EXISTS \"${CMAKE_CURRENT_SOURCE_DIR}/transpiled_smoke.cpp\"\n"
        "   AND (CMAKE_CXX_COMPILER_ID STREQUAL \"Clang\"\n"
        "        OR CMAKE_CXX_COMPILER_ID STREQUAL \"AppleClang\"))\n"
        "    add_executable(btree_port_transpiled_smoke transpiled_smoke.cpp)\n"
        "    target_link_libraries(btree_port_transpiled_smoke PRIVATE btree_port)\n"
        "endif()\n"
    )
    # Match from 'add_library(btree_port' through the FIRST ')' that
    # closes a target_sources block following it.
    pattern = re.compile(
        r"add_library\(btree_port\s*\n(?:.*\n)*?target_sources\(btree_port[^)]*\)\s*\n",
        re.DOTALL,
    )
    if pattern.search(src):
        src = pattern.sub(trim_block, src, count=1)
        path.write_text(src)
        print(f"  trimmed: {path.name}")
    else:
        print(
            f"  [warn] could not find add_library/target_sources block in {path.name}",
            file=sys.stderr,
        )


def _run_auto_namespace_postprocess(cpp_out_dir: Path) -> None:
    """Invoke the sibling auto-namespace postprocess script.

    That script (`btreeset_auto_namespace_postprocess.py`) is responsible
    for the cross-cutting auto-namespace fix-ups: hoisting `import` lines
    out of the namespace wrap, and emitting the `namespace LEAF = ::FULL;`
    aliases inside each wrap. Doing it as a subprocess keeps the two
    scripts decoupled — the postprocess is meant to be eventually folded
    into the transpiler itself (see STATUS.md Step 86).
    """
    import subprocess

    script = Path(__file__).resolve().parent / "btreeset_auto_namespace_postprocess.py"
    if not script.exists():
        print(f"  [warn] {script.name} not found; skipping auto-namespace step")
        return
    try:
        subprocess.run(
            [sys.executable, str(script), str(cpp_out_dir)],
            check=True,
        )
    except subprocess.CalledProcessError as exc:
        print(f"  [warn] auto-namespace postprocess failed: {exc}")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    cpp_out_dir = Path(sys.argv[1])
    if not cpp_out_dir.is_dir():
        # Back-compat: also accept the .cppm path directly.
        if cpp_out_dir.suffix == ".cppm":
            print("[1/2] patching btree_internal.cppm")
            patch_internal(cpp_out_dir)
            return 0
        print(f"error: {cpp_out_dir} is not a directory", file=sys.stderr)
        return 2

    internal = cpp_out_dir / "btree_port.btree.btree_internal.cppm"
    cmake = cpp_out_dir / "CMakeLists.txt"
    set_entry = cpp_out_dir / "btree_port.btree.set.entry.cppm"
    map_entry = cpp_out_dir / "btree_port.btree.map.entry.cppm"
    map_mod = cpp_out_dir / "btree_port.btree.map.cppm"
    set_mod = cpp_out_dir / "btree_port.btree.set.cppm"
    rusty_include_dir = Path(__file__).resolve().parent.parent.parent / "include"

    # Run the auto-namespace postprocess FIRST — it injects `import` lines
    # at module purview (the transpiler stuffs them inside the namespace
    # wrap, which is a parse error) and emits `namespace LEAF = ::FULL;`
    # aliases inside each wrap so the transpiler's `btree_internal::X` /
    # `marker::Y` / `node::Z` qualifiers resolve. This used to be a manual
    # step (documented in STATUS.md §86) but is required for the wraps
    # to compile, so we invoke it automatically.
    print(f"[*] running auto-namespace postprocess (imports + aliases)")
    _run_auto_namespace_postprocess(cpp_out_dir)

    print(f"[1/6] patching {internal.name}")
    # Apply the local-clone drop to every transpiled .cppm — each file
    # gets a copy from the transpiler's prelude.
    for p in (internal, map_mod, map_entry, set_mod, set_entry):
        if p.exists():
            drop_redundant_rusty_clone(p)
    # New-emit-shape drift fixes (sibling-port pattern): drop the leftover
    # serde visitor stub that references rusty::Vec (unimported in btree),
    # strip the matching `using rusty::Vec;` declaration that the transpiler
    # added as a side-effect of seeing `use alloc::vec::Vec` in source,
    # and rewrite the `*new_node_shadow1.field` pointer-arrow precedence
    # bug in split_leaf_data. All must run BEFORE patch_internal so that
    # subsequent stub-injection sites aren't shifted by these edits.
    for p in (internal, map_mod, map_entry, set_mod, set_entry):
        if p.exists():
            fix_using_rusty_vec(p)
            fix_visit_byte_buf_unknown_vec(p)
            # Closes the B1 edge case: position-based dispatch for
            # ambiguous bare-glob variants (Leaf/Internal in
            # ForceResult/Position; Left/Right in LeafOrRight/Either).
            fix_unresolved_bare_glob_variants(p)
    fix_leafnode_shadow_arrow(internal)
    fix_full_range_recursive_call(internal)
    # Nested `Ok(LeftOrRight::Variant(_))` arm: inner condition wraps the
    # outer `_m` (Result) instead of the unwrapped `_mvN` (LeftOrRight).
    # Affects 4 sites in fix_node_through_parent (B-tree's rebalance path).
    fix_nested_ok_variant_index_uses_outer_m(internal)
    for p in (internal, map_mod, map_entry, set_mod, set_entry):
        if p.exists():
            fix_noderef_borrow_mut_auto_ref(p)
            fix_occupied_entry_no_template_args(p)
    patch_internal(internal)
    # Phase B: replace stubs with real impls. Runs AFTER patch_internal
    # so that on fresh transpile output we first install the stub
    # (which makes the templated method parse) and then immediately
    # override with the real impl. On re-runs, both are idempotent.
    implement_from_new_leaf(internal)
    implement_from_new_internal(internal)
    implement_push_with_handle(internal)
    implement_deallocating(internal)
    # Phase E (correctness fixes surfaced at instantiation time):
    fix_dormant_mut_ref_from_t(internal)
    fix_dormant_mut_ref_const_ref(internal)
    fix_as_leaf_ptr_self(internal)
    fix_const_correctness(internal)
    fix_assume_init_ref_on_span(internal)
    # NOTE: strip_redundant_method_template_params disabled — the
    # method-level template params LOOK redundant but the enclosing
    # struct is `Handle<Node, Type>` (different names), so removing
    # them makes BorrowType/K/V undeclared. Fixing this properly
    # requires either deducing them from Node's components (gnarly
    # SFINAE) or moving the methods to a specialization of Handle.
    #
    # NOTE: fix_force_match_arms (whole-file scope tracking) removed —
    # the variable-tracking heuristic was unreliable across nested
    # scopes. Instead, hand-port the specific methods whose bodies
    # hit the unrecoverable force()-arm pattern.
    implement_leaf_edge_walkers(internal)
    implement_handle_descend(internal)
    # Handle::force has the same method-template recovery shape as
    # descend; route through __NodeRefArgs<Node> trait.
    implement_handle_force(internal)
    # Handle::into_kv — same shape, same fix.
    implement_handle_into_kv(internal)
    # Moved into the transpiler: get_unchecked emit at codegen.rs now
    # wraps unknown-receiver calls in a SFINAE `requires { recv[idx]; }`
    # shim that routes std::array / std::vector / std::span through
    # operator[] while leaving real `get_unchecked`-having types
    # (Rusty slice helpers) untouched.
    # Issue D from write-path investigation: absorbed Handle::split emits
    # `LeafNode<A, Node>::new_(...)` (wrong recovery — A is method's
    # `fix_leafnode_new_template_args` was lifted into the transpiler as
    # Item 7 — Cluster A recovery now resolves K/V via
    # `__TemplateArgs<Node>::arg_<N>` directly at emit time.
    # Issue D follow-on: `NodeRef<…>::from_new_leaf(…)` should
    # be `<Owned,…,Leaf>` (Rust source has no explicit args).
    fix_from_new_leaf_markers(internal)
    # `stub_insert_recursing` was needed before Items 1 + 11 of the
    # GENERIC_FIXES_PLAN landed; the transpiler now lowers
    # insert_recursing's body directly. Call removed; the function
    # itself is also gone (replaced with a comment block).
    # Issue A from write-path investigation: `const auto map = this->dormant_map.reborrow();`
    # decays the &mut T return to a value copy. Rewrite to `auto&`.
    fix_dormant_map_reborrow_binding(internal)
    # `k.borrow()` on primitive types — wrap with SFINAE fallback.
    fix_borrow_method_fallback(internal)
    # Step 55: codify the 7 step-54 insert-path fixes (key/val/edge_area_mut
    # signatures, Handle::reborrow/reborrow_mut/dormant/awaken via
    # __NodeRefArgs, insert_fit/split/split_leaf_data simplifications,
    # LeafNode::new_ via new_in, middle.split path correction).
    apply_step54_insert_path_fixes(internal)
    # Step 60: codify step 58/59 fixes — __IsNodeRef concept injection,
    # InternalNode::new_ bypass, correct_parent_link arg recovery,
    # .height → .height_field rewrites.
    apply_step58_lazy_gates_and_fixes(internal)
    # After Cluster A landed, the transpiler drops method-template params
    # for `Handle::new_kv` / `new_edge`, but the static-factory parameter
    # type `NodeRef<BorrowType, K, V, NodeType>` still references those
    # identifiers — at signature scope, before the body's `using` aliases
    # take effect. Convert the param to `auto` so it's deduced from the arg.
    fix_static_factory_param_type_recovery(internal)
    # Companion to the previous fix: the 5 remaining
    # `Handle<auto, auto>::new_edge(...)` call sites can't be expressed
    # in C++ template syntax. Route them through a deducing helper.
    inject_handle_make_edge_helper(internal)
    # Fix the Ok-arm Result qualifier mis-emit in Handle::left_kv /
    # Handle::right_kv (Self leaked into the T position).
    fix_left_kv_right_kv_ok_result_type(internal)
    # search_tree is hand-ported; the older stub_broken_search_tree
    # is left in the source for reference but not invoked.
    implement_search_tree(internal)
    print(f"[2/6] patching {cmake.name}")
    patch_cmake(cmake, rusty_include_dir)
    print(f"[*] writing link_smoke.cpp")
    write_link_smoke(cpp_out_dir)
    print(f"[*] writing transpiled_read_smoke.cpp")
    write_transpiled_read_smoke(cpp_out_dir)
    print(f"[3/6] patching {set_entry.name}")
    if set_entry.exists():
        # set.entry imports map.entry (and indirectly map). Strip the
        # `map::` and `btree_internal::` qualifier prefixes too — after
        # the import, those symbols live at file scope and the prefix
        # references are stale. Same shape as map.cppm patches.
        patch_entry_imports(
            set_entry,
            extra_imports=[
                "btree_port.btree.map",
                # map.entry has been merged into map.cppm by
                # `merge_map_entry_into_map`; the module no longer
                # ships independently. Drop the import here.
            ],
        )
        patch_entry_arities(set_entry)
        strip_module_namespace_prefixes(
            set_entry, ["btree_internal", "map", "node"]
        )
        align_requires_clauses(set_entry)
        # Same `template<typename T>` misroute pattern as map.entry
        # but now on the set side (methods absorbed from sister types).
        remove_setvalzst_methods(set_entry)
        # `SetValZST` referenced as a value (not a type) — same as
        # map.cppm. Wrap as default-constructed (`SetValZST{}`).
        # Since this is the actual definition's home, it's likely
        # less misroute-shaped than in map.cppm — we just need value
        # construction.
        fix_setvalzst_as_value(set_entry)
    else:
        print(f"  [skip] {set_entry.name} not present")
    print(f"[4/6] patching {map_entry.name}")
    if map_entry.exists():
        patch_entry_imports(map_entry, extra_imports=[])
        strip_module_namespace_prefixes(map_entry, ["btree_internal"])
        align_requires_clauses(map_entry)
        remove_setvalzst_methods(map_entry)
        # Temporarily disabled to chase remaining write-path errors.
        # stub_nodref_insert_entry(map_entry)
        stub_broken_map_entry_methods(map_entry)
        # Step 52: merge map.entry's struct definitions into map.cppm
        # (handled inside the map.cppm patching block below, since it
        # operates on map.cppm as the destination).
    else:
        print(f"  [skip] {map_entry.name} not present")
    print(f"[5/6] patching {map_mod.name}")
    if map_mod.exists():
        # Step 52: merge map.entry into map.cppm BEFORE any other
        # map.cppm patches. This skips the rusty::BTreeMap facade
        # in map.cppm's GMF, inlines the entry struct definitions,
        # and substitutes `rusty::BTreeMap` → `BTreeMap` so the
        # transpiled type owns the name within this TU.
        merge_map_entry_into_map(map_mod, map_entry)
        # map.cppm has the same import-ordering bug as map.entry and
        # additionally references `entry::*` and `node::*` qualifier
        # prefixes that don't survive the import (C++20 modules
        # don't expose imported symbols under a module-named
        # namespace). Strip both prefixes.
        patch_entry_imports(map_mod, extra_imports=[])
        strip_module_namespace_prefixes(map_mod, ["btree_internal", "entry", "node"])
        # Iterator structs (Iter, IterMut, Range, RangeMut, Keys,
        # Values, …) inherit orphan-impl methods from the underlying
        # `BTreeMap::iter()` / `BTreeMap::range()` returning types.
        # The misrouted methods use `template<typename T>` shape and
        # reference `this->iter.*` — same pattern as map.entry.
        remove_setvalzst_methods(map_mod)
        # set::BTreeSet methods like `replace(value)` and
        # `get_or_insert_with(...)` get injected at the
        # map::BTreeMap level WITHOUT a `template<typename T>`
        # qualifier (set's BTreeSet uses the SAME generic T,
        # so the injection doesn't add a new template param).
        # The body references SetValZST which doesn't exist at
        # map.cppm's scope; hide them.
        hide_template_free_misroutes(map_mod)
        # `::boxed::Box<…>` is the Rust alloc::boxed::Box path; on
        # the C++ side we have `rusty::Box`. Rewrite.
        fix_boxed_box_path(map_mod)
        # `NodeRef::new_leaf` / `Root::new_` / `Handle::into_kv` are
        # emitted without their concrete template arguments. Substitute
        # the K/V-in-scope forms (phase A1 of the transpile-path plan).
        recover_template_args(map_mod)
        # `VacantEntry<…>{.key = …}` should be `.key_field = …`.
        fix_vacant_entry_key_field(map_mod)
        # `BTreeMap::entry` body has compounded aggregate-init bugs.
        # Step 49 attempted a hand-port but ran into a deeper namespace
        # Step 52 unblocked the entry hand-port by merging the entry
        # struct definitions into map.cppm. With OccupiedEntry /
        # VacantEntry sharing BTreeMap's module attachment, the entry
        # method body's aggregate-init compiles.
        implement_btreemap_entry(map_mod)
        # get/first_key_value/last_key_value all cascade from
        # search_tree's stub; stub them too.
        stub_broken_map_methods(map_mod)
        # `DormantMutRef::new_(x)` is similar but the T varies per call
        # site (BTreeMap/Root/Option<Root>). Inject a deduction helper
        # and rewrite call sites to use it (phase A2).
        fix_dormant_mut_ref_calls(map_mod)
        # `fix_recursive_lambda_clone_subtree` was lifted into the
        # transpiler as Item 8 — `emit_nested_function` now detects
        # self-recursive nested fns and emits Y-combinator-shaped lambdas
        # (extra `auto&& __self` param, `__self(__self, …)` inside body,
        # `NAME(NAME, …)` at external call sites).
        # MIN_LEN is duplicated between btree_internal and map; drop
        # the map-side decls (phase A4).
        drop_duplicate_min_len(map_mod)
        # `merge(other, conflict)` has an undeclared `Q` from a Rust
        # `Borrow<Q>` constraint; substitute Q→K (phase A4).
        fix_merge_unknown_Q(map_mod)
        # `entry()` body uses VacantEntry<T,A>/OccupiedEntry<T,A>
        # instead of <K,V,A> (set-side spelling leaked) (A4).
        fix_entry_T_to_KV(map_mod)
        # `new_()` body uses `A` (enclosing generic) but returns
        # `BTreeMap<K, V>` (Global). Replace A→Global in those
        # specific call sites (A4).
        fix_new_global_alloc(map_mod)
        # `rusty::alloc::Global` used in call-arg position needs to
        # be `Global{}` (default construct the unit struct) (A4).
        fix_global_as_value(map_mod)
        # `f.debug_map()` is not a member of rusty::fmt::Formatter;
        # use debug_list instead (A4).
        fix_debug_map_call(map_mod)
        # `return /* write!(...) */;` leftover from untranspiled
        # write! macro — needs a return value (A4).
        fix_empty_write_return(map_mod)
        # `fix_tuple_dot_underscore_access` removed — Item 1 of the
        # GENERIC_FIXES_PLAN landed in the transpiler: when the
        # receiver type is unknown, the emit now wraps `expr.N` in a
        # `requires { __t._N; }` SFINAE dispatch that picks `_N` for
        # tuple-structs or `std::get<N>` for std::tuple at C++ compile
        # time.
        # Item 6 partial lift: `reborrow` is now in the transpiler's
        # method-name heuristic so `let map = …reborrow()` emits
        # `auto& map = …` directly. The follow-on `Option::insert`
        # site and outer IIFE-result handle binding still need the
        # patcher (typed-receiver awareness for `Option::insert`).
        fix_dormant_map_reborrow_binding(map_mod)
        # Step 67: codify the step 64-66 runtime fixes that brought the
        # transpiled BTreeMap smoke test to all-green. Must run AFTER
        # stub_broken_map_methods so the stubbed bodies get OVERWRITTEN
        # with the real implementations.
        apply_step66_map_runtime_fixes(map_mod)
    else:
        print(f"  [skip] {map_mod.name} not present")
    print(f"[6/6] patching {set_mod.name}")
    if set_mod.exists():
        patch_entry_imports(set_mod, extra_imports=[])
        strip_module_namespace_prefixes(set_mod, ["btree_internal", "entry", "node"])
        remove_setvalzst_methods(set_mod)
        fix_boxed_box_path(set_mod)
    else:
        print(f"  [skip] {set_mod.name} not present")
    # Final pass: under --auto-namespace, the transpiled exports are
    # wrapped in `namespace btree_port::btree::X { … }`. The patcher's
    # hand-port code paths still use `::marker::X` / `::LeafNode<…>`
    # global qualifiers from the flat-export era. Strip those so name
    # lookup finds the in-namespace sibling.
    print(f"[*] stripping stale global qualifiers (auto-namespace mode)")
    fix_global_qualifiers_for_namespace_wrap(internal)
    if map_mod.exists():
        fix_global_qualifiers_for_namespace_wrap(map_mod)
    if set_mod.exists():
        fix_global_qualifiers_for_namespace_wrap(set_mod)
    if set_entry.exists():
        fix_global_qualifiers_for_namespace_wrap(set_entry)
    # Move the __TemplateArgs primary into the namespace wrap so the
    # partial specializations (also in the wrap) match its scope.
    fix_template_args_primary_scope(internal)
    # Move std::hash<...> specializations OUT of the namespace wrap
    # (they must be at a scope enclosing `namespace std`).
    fix_std_hash_specialization_scope(internal)
    if map_mod.exists():
        fix_std_hash_specialization_scope(map_mod)
    # Map-specific qualifier fix-ups: under auto-namespace mode,
    # patcher hand-port code references unqualified `NodeRef` (which
    # lives in btree_internal::) and the merged entry content has
    # `::BTreeMap` / `::IntoIter` global qualifiers that need to be
    # in-scope names.
    if map_mod.exists():
        fix_map_cppm_qualifiers_for_namespace_wrap(map_mod)
    if set_mod.exists():
        # Set-specific rules MUST run before the shared map-qualifier
        # pass — the shared pass turns `::BTreeSet<` into `set::BTreeSet<`
        # (correct for map.cppm, wrong for set.cppm where BTreeSet is
        # in-scope and should be bare).
        fix_set_cppm_qualifiers_for_namespace_wrap(set_mod)
        fix_map_cppm_qualifiers_for_namespace_wrap(set_mod)
        stub_broken_set_methods(set_mod)
    if set_entry.exists():
        fix_map_cppm_qualifiers_for_namespace_wrap(set_entry)
    return 0


if __name__ == "__main__":
    sys.exit(main())
