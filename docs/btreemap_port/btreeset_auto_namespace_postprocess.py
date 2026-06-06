#!/usr/bin/env python3
"""Post-process --auto-namespace transpiled files to:
1. Move `import X.Y.Z;` lines that landed inside the namespace wrap
   back to module purview (before the `namespace … {` line).
2. Add explicit `import` lines for sibling modules that the transpiler
   missed in single-file mode (no crate-wide sibling info).
3. Emit `namespace LEAF = ::FULL;` aliases inside the wrap for each
   import, so existing `map::X` / `btree_internal::Y` emit shapes
   resolve correctly.

This is the work that should ideally happen in the transpiler itself
(see docs/btreemap_port/STATUS.md Step 86 caveat). Doing it as a
post-process step here is the pragmatic shortcut to unblock BTreeSet
vendoring without a deeper crate-mode refactor.
"""
import re
import sys
from pathlib import Path

def postprocess(path: Path, extra_imports: list[str]) -> None:
    text = path.read_text()
    lines = text.split('\n')

    # 0. Idempotency — strip artifacts from previous postprocess runs so
    #    we don't accumulate duplicates. The script inserts three things:
    #    a) import lines above the namespace wrap
    #    b) `namespace X {}` forward-decls right above the wrap
    #    c) `namespace LEAF = ::FULL;` aliases inside the wrap
    #    A re-run keeps adding these without removing prior copies. Strip
    #    each before re-inserting (the canonical set is recomputed below).
    ns_open_idx_pre = None
    for i, line in enumerate(lines):
        m = re.match(r'^namespace ([a-zA-Z_][a-zA-Z0-9_:]*) \{$', line)
        if m and m.group(1).startswith('btree_port::'):
            ns_open_idx_pre = i
            break
    if ns_open_idx_pre is not None:
        # (a)+(b): strip standalone import/forward-decl lines between the
        # last comment/code and the namespace open. Walk backwards from
        # the open, removing import lines and single-line `namespace X {}`
        # decls. Stop at the first line that's neither.
        i = ns_open_idx_pre - 1
        while i >= 0:
            s = lines[i].strip()
            if (re.match(r'^import [a-z_.]+;$', s)
                    or re.match(r'^namespace [a-zA-Z_:0-9]+ \{\}$', s)
                    or s == ''):
                lines[i] = None  # type: ignore
                i -= 1
            else:
                break
        lines = [ln for ln in lines if ln is not None]
        # (c): strip `namespace X = ::btree_port::...;` alias lines right
        # after the namespace open.
        ns_open_idx_pre = None
        for j, line in enumerate(lines):
            m = re.match(r'^namespace ([a-zA-Z_][a-zA-Z0-9_:]*) \{$', line)
            if m and m.group(1).startswith('btree_port::'):
                ns_open_idx_pre = j
                break
        if ns_open_idx_pre is not None:
            j = ns_open_idx_pre + 1
            while j < len(lines):
                s = lines[j].strip()
                if (re.match(r'^namespace [a-zA-Z_][a-zA-Z_0-9]* = ::btree_port::[a-zA-Z_:0-9]+;$', s)
                        or s == ''):
                    lines[j] = None  # type: ignore
                    j += 1
                else:
                    break
            lines = [ln for ln in lines if ln is not None]

    # 1. Find the namespace wrap open and close.
    ns_open_idx = None
    ns_close_idx = None
    ns_path = None
    for i, line in enumerate(lines):
        m = re.match(r'^namespace ([a-zA-Z_][a-zA-Z0-9_:]*) \{$', line)
        if m and m.group(1).startswith('btree_port::') and ns_open_idx is None:
            ns_open_idx = i
            ns_path = m.group(1)
        elif line.startswith('} // namespace btree_port::'):
            ns_close_idx = i
    if ns_open_idx is None:
        print(f"  [skip] no namespace btree_port:: wrap found in {path.name}")
        return

    # 2. Find `import` lines INSIDE the namespace (between open and close).
    imports_inside = []
    other_lines = []
    in_namespace = False
    for i, line in enumerate(lines):
        if i == ns_open_idx:
            in_namespace = True
        elif i == ns_close_idx:
            in_namespace = False
        if in_namespace and re.match(r'^import [a-z_.]+;\s*$', line):
            imports_inside.append(line)
            # Mark this line for removal — we'll filter it out below.
            lines[i] = None  # type: ignore
    lines = [ln for ln in lines if ln is not None]

    # Recompute indices after removal.
    ns_open_idx = None
    for i, line in enumerate(lines):
        if line is not None and re.match(r'^namespace [a-zA-Z_:]+ \{$', line) \
                and line.startswith(f'namespace {ns_path} {{'):
            ns_open_idx = i
            break
    assert ns_open_idx is not None

    # 3. Collect all imports (from-inside + extras), dedup, sort.
    all_imports = set(imports_inside)
    for imp in extra_imports:
        all_imports.add(f"import {imp};")
    all_imports = sorted(all_imports)

    # 4. Insert imports BEFORE the namespace-open. Insert a blank line first
    #    for readability.
    import_block = []
    for imp in all_imports:
        import_block.append(imp)
    import_block.append('')  # blank line
    for j, line in enumerate(import_block):
        lines.insert(ns_open_idx + j, line)
    ns_open_idx += len(import_block)

    # 5. Inside the namespace, right after the open brace, emit alias
    #    declarations for each import. Skip aliases whose leaf is our own
    #    namespace's leaf segment (would shadow ourselves).
    #
    #    For namespace aliases to compile, the target namespace must be
    #    declared BEFORE the alias. We can't always control that (some
    #    target namespaces are defined inside an imported module — fine
    #    — others are nested namespaces that the patcher merges into the
    #    SAME TU later, e.g. `map.entry` merged into `map.cppm` by
    #    `merge_map_entry_into_map`). Emit a global forward-decl of each
    #    target namespace at module purview RIGHT BEFORE the wrap-open
    #    so the in-wrap alias can resolve.
    own_leaf = ns_path.rsplit('::', 1)[-1]
    alias_block = []
    forward_decls = []
    seen = set()
    for imp_line in all_imports:
        m = re.match(r'^import ([a-z_.]+);$', imp_line)
        if not m:
            continue
        full = m.group(1)
        leaf = full.rsplit('.', 1)[-1]
        if leaf == own_leaf or leaf in seen:
            continue
        seen.add(leaf)
        cpp_path = full.replace('.', '::')
        # If the imported module's namespace is a child of the current
        # namespace (e.g. importing `btree_port.btree.set.entry` from
        # inside `namespace btree_port::btree::set { … }`), skip the
        # alias — `entry::Foo` will resolve via nested-namespace
        # lookup, and emitting `namespace entry = …` would collide
        # with the nested `entry` namespace.
        if cpp_path.startswith(ns_path + '::'):
            forward_decls.append(f'namespace {cpp_path} {{}}')
            continue
        forward_decls.append(f'namespace {cpp_path} {{}}')
        alias_block.append(f'namespace {leaf} = ::{cpp_path};')
    # Special-case: the transpiled btree code in `map.cppm` /
    # `set.cppm` / etc. references `marker::X` (the marker types
    # defined inside `btree_internal::marker`). Under flat-export
    # mode this resolved because `marker` was at global scope; under
    # auto-namespace mode `marker` is now nested inside btree_internal,
    # so we need an explicit alias.
    if "btree_internal" in seen and "marker" not in seen:
        seen.add("marker")
        alias_block.append(
            "namespace marker = ::btree_port::btree::btree_internal::marker;"
        )
    # Same idea for `node::X` references in the merged btree code:
    # the prep.sh step folded the rustc `node.rs` submodule into
    # `btree_internal.rs`, so `node::Root` / `node::MIN_LEN_AFTER_SPLIT`
    # are now top-level symbols of btree_internal. Alias `node` to
    # `btree_internal` so those references compile.
    if "btree_internal" in seen and "node" not in seen:
        seen.add("node")
        alias_block.append(
            "namespace node = ::btree_port::btree::btree_internal;"
        )
    # 5a. Drop the transpiler's placeholder `namespace X {}` lines and
    #     `using namespace X;` lines that originate from Rust `use X::*`
    #     where X is a sibling MODULE (not a real namespace). With the
    #     wrapping namespace + alias we emit just above, those
    #     placeholders collide with the alias names. The transpiler
    #     emits one or both per sibling-module reference.
    #     Run this strip BEFORE inserting forward-decls/aliases so the
    #     ns_open_idx we computed earlier (against the original lines)
    #     still matches up.
    leaf_names = {imp.split('.')[-1] for imp in extra_imports} | {
        m.group(1).split('.')[-1]
        for m in (re.match(r'^import ([a-z_.]+);$', imp_line)
                  for imp_line in all_imports)
        if m
    }
    n_stripped = 0
    for i, line in enumerate(lines):
        if line is None:
            continue
        s = line.strip()
        m_ns = re.match(r"^namespace ([a-zA-Z_][a-zA-Z_0-9]*) \{\}$", s)
        if m_ns and m_ns.group(1) in leaf_names:
            lines[i] = None
            n_stripped += 1
            continue
        m_using = re.match(
            r"^using namespace (?:::)?([a-zA-Z_][a-zA-Z_0-9]*);$", s
        )
        if m_using and m_using.group(1) in leaf_names:
            lines[i] = None
            n_stripped += 1
            continue
    if n_stripped:
        # Recompute ns_open_idx after the strip (it may have moved up if
        # any stripped lines were before the namespace open).
        lines = [ln for ln in lines if ln is not None]
        for i, line in enumerate(lines):
            if re.match(rf'^namespace {re.escape(ns_path)} \{{$', line):
                ns_open_idx = i
                break
    if forward_decls:
        # Insert forward-decls right before the namespace open.
        for j, line in enumerate(forward_decls):
            lines.insert(ns_open_idx + j, line)
        ns_open_idx += len(forward_decls)
    if alias_block:
        alias_block.append('')  # blank line
        for j, line in enumerate(alias_block):
            lines.insert(ns_open_idx + 1 + j, line)

    path.write_text('\n'.join(lines))
    print(f"  postprocessed {path.name}: "
          f"moved {len(imports_inside)} import(s) out, "
          f"added {len(extra_imports)} extra import(s), "
          f"emitted {len(alias_block) - (1 if alias_block else 0)} namespace alias(es)")


if __name__ == "__main__":
    # Default to the v3 crate-mode transpile output, but allow override
    # via argv[1] for ad-hoc reuse.
    src = Path(sys.argv[1]) if len(sys.argv) > 1 \
        else Path("/tmp/btreeset_v3/cpp_out")
    # All four files in the btree_port library target need the
    # imports-out-of-namespace + alias treatment. The transpiler emits
    # `import ...;` lines inside the `namespace btree_port::btree::X {`
    # wrap because the import-collection pass runs after the wrap is
    # opened; C++20 module syntax requires imports at module purview.
    #
    # btree_internal needs no sibling imports (it's the bottom of the
    # dependency graph), but we still run it through to handle any
    # stray imports the transpiler placed inside.
    postprocess(src / "btree_port.btree.btree_internal.cppm", [])
    postprocess(src / "btree_port.btree.map.cppm", [
        "btree_port.btree.btree_internal",
        # `btree_port.btree.map.entry` is merged into map.cppm by the
        # main patcher's `merge_map_entry_into_map`; the standalone
        # module no longer ships, so the import would be dead. Forward-
        # decl + alias still get emitted because merge_map_entry_into_map
        # keeps `entry::Foo` references in the merged content.
    ])
    postprocess(src / "btree_port.btree.map.entry.cppm", [
        "btree_port.btree.btree_internal",
        "btree_port.btree.map",
    ])
    postprocess(src / "btree_port.btree.set.cppm", [
        "btree_port.btree.btree_internal",
        "btree_port.btree.map",
        "btree_port.btree.set.entry",
    ])
    postprocess(src / "btree_port.btree.set.entry.cppm", [
        "btree_port.btree.btree_internal",
        "btree_port.btree.map",
    ])
