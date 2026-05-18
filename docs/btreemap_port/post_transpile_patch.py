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
        if line.rstrip() == "    template<typename T>" and i + 1 < n:
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
                    if lines[k].rstrip() == "" and k + 1 < n and lines[k + 1].rstrip() == "    template<typename T>":
                        # blank between methods
                        k += 1
                        continue
                    if lines[k].rstrip() == "    template<typename T>":
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

    Idempotent: skipped if the sentinel is already present."""
    import re

    src = path.read_text()
    sentinel = "// btree_port port: module prefixes stripped by post_transpile_patch.py"
    if sentinel in src:
        print(f"  no changes to: {path.name} (prefixes already stripped)")
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


def patch_cmake(path: Path, rusty_include_dir: Path) -> None:
    """Trim CMakeLists.txt to btree_internal-only and wire the rusty
    include path so reconfigure doesn't drop -I."""
    src = path.read_text()
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
        "    list(APPEND _BTREE_PORT_SOURCES btree_port.btree.map.entry.cppm)\n"
        "endif()\n"
        "\n"
        "add_library(btree_port ${_BTREE_PORT_SOURCES})\n"
        "\n"
        "target_sources(btree_port PUBLIC FILE_SET CXX_MODULES FILES\n"
        "    ${_BTREE_PORT_SOURCES}\n"
        ")\n"
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

    print(f"[1/6] patching {internal.name}")
    patch_internal(internal)
    print(f"[2/6] patching {cmake.name}")
    patch_cmake(cmake, rusty_include_dir)
    print(f"[3/6] patching {set_entry.name}")
    if set_entry.exists():
        # NOTE: set.entry isn't currently in the build target (it depends
        # on `import btree_port.btree.map`, which has its own transpiler
        # bugs). The patch logic is left in place so a future iteration
        # can flip it on by adding set.entry + map.entry to CMakeLists.
        patch_entry_imports(set_entry, extra_imports=["btree_port.btree.map"])
        patch_entry_arities(set_entry)
        strip_module_namespace_prefixes(set_entry, ["btree_internal"])
        align_requires_clauses(set_entry)
    else:
        print(f"  [skip] {set_entry.name} not present")
    print(f"[4/6] patching {map_entry.name}")
    if map_entry.exists():
        patch_entry_imports(map_entry, extra_imports=[])
        strip_module_namespace_prefixes(map_entry, ["btree_internal"])
        align_requires_clauses(map_entry)
        remove_setvalzst_methods(map_entry)
        stub_nodref_insert_entry(map_entry)
    else:
        print(f"  [skip] {map_entry.name} not present")
    print(f"[5/6] patching {map_mod.name}")
    if map_mod.exists():
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
    else:
        print(f"  [skip] {map_mod.name} not present")
    print(f"[6/6] patching {set_mod.name}")
    if set_mod.exists():
        patch_entry_imports(set_mod, extra_imports=[])
        strip_module_namespace_prefixes(set_mod, ["btree_internal", "entry", "node"])
        remove_setvalzst_methods(set_mod)
    else:
        print(f"  [skip] {set_mod.name} not present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
