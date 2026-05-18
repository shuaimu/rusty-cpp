#!/usr/bin/env bash
# Pre-process a vendored copy of rustc's library/alloc/src/collections/btree/
# for transpilation with rusty-cpp-transpiler.
#
# Why: the stdlib source uses several crate-internal paths that don't
# resolve when only the `btree` subtree is vendored — there is no
# enclosing `crate::alloc::*` or `crate::boxed::*` to look up. We
# rewrite those to the equivalent public std::* paths so the
# transpiler's existing std-mapping table can route them through to
# rusty::* without a separate crate-resolution pass.
#
# Usage: bash prep.sh <btree_dir>
# where <btree_dir> is a copy of the stdlib btree/ source.
#
# Idempotent — safe to re-run.
set -euo pipefail

BTREE_DIR="${1:?usage: prep.sh <btree_dir>}"

if [[ ! -d "$BTREE_DIR" ]]; then
  echo "error: $BTREE_DIR is not a directory" >&2
  exit 1
fi

# Strip the rustc-internal tests — they don't transpile (they depend
# on rand and other test-only crates). Each btree submodule has both:
#   1. A `mod tests;` declaration in its parent file (gated by
#      `#[cfg(test)]` which the transpiler skips anyway), and
#   2. An adjacent `<submodule>/tests.rs` file that we remove here.
#      Some btree versions used `tests*/` directories instead of
#      sibling `*.rs` files; handle both shapes.
find "$BTREE_DIR" -name "tests*" -type d -exec rm -rf {} + 2>/dev/null || true
find "$BTREE_DIR" -name "tests.rs" -type f -delete 2>/dev/null || true

# crate::alloc::* → std::alloc::* (Allocator, Global, Layout, AllocError)
# The btree code uses `crate::alloc::*` to reach the alloc crate's
# internal alloc module, but when only the btree subtree is vendored
# there is no enclosing alloc crate. Re-route to `std::alloc::*` so
# the transpiler's std-mapping table picks them up.
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::alloc::|use std::alloc::|g' \
  -e 's|crate::alloc::Allocator|std::alloc::Allocator|g' \
  -e 's|crate::alloc::Global|std::alloc::Global|g' \
  -e 's|crate::alloc::Layout|std::alloc::Layout|g' \
  -e 's|crate::alloc::AllocError|std::alloc::AllocError|g' \
  {} \;

# crate::boxed::Box → alloc::boxed::Box (same reasoning — there's no
# `crate::boxed` because the alloc crate's boxed module isn't
# vendored alongside btree).
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::boxed::Box|use alloc::boxed::Box|g' \
  {} \;

# crate::vec::Vec → alloc::vec::Vec (same reasoning).
find "$BTREE_DIR" -name "*.rs" -exec sed -i \
  -e 's|use crate::vec::Vec|use alloc::vec::Vec|g' \
  {} \;

# Targeted hand-patch: `merge_iter.rs::MergeIterInner::nexts` declares
# `let mut a_next;` / `let mut b_next;` without initializers (Rust allows
# this when the compiler can prove definite assignment via match arms;
# C++'s `auto` requires an initializer). Initialize with `None` so the
# transpiled output is compilable C++. Semantics unchanged — the
# variables are unconditionally overwritten in every match arm
# immediately after.
#
# Note: applied BEFORE the cycle-breaking concatenation below; the
# merge_iter.rs file gets folded into btree_internal.rs.
if [[ -f "$BTREE_DIR/merge_iter.rs" ]]; then
  sed -i \
    -e 's|^        let mut a_next;$|        let mut a_next = None;|' \
    -e 's|^        let mut b_next;$|        let mut b_next = None;|' \
    "$BTREE_DIR/merge_iter.rs"

  # `enum Peeked<I> { A(I::Item), B(I::Item) }` collides with the
  # BTree branching-factor const `B = 6` after the concatenation
  # (both `B` and `A` become file-scope symbols). Rename the
  # variants to `Left`/`Right` so the const `B` keeps the name.
  # Variants are referenced only inside merge_iter.rs, so the
  # rename is local to this file.
  sed -i \
    -e 's|enum Peeked<I: Iterator> {|enum Peeked<I: Iterator> {|' \
    -e 's|    A(I::Item),|    Left(I::Item),|' \
    -e 's|    B(I::Item),|    Right(I::Item),|' \
    -e 's|Peeked::A(|Peeked::Left(|g' \
    -e 's|Peeked::B(|Peeked::Right(|g' \
    -e 's|Peeked::A,|Peeked::Left,|g' \
    -e 's|Peeked::B,|Peeked::Right,|g' \
    -e 's|Peeked::A)|Peeked::Left)|g' \
    -e 's|Peeked::B)|Peeked::Right)|g' \
    -e 's|\.map(Peeked::A\b|.map(Peeked::Left|g' \
    -e 's|\.map(Peeked::B\b|.map(Peeked::Right|g' \
    "$BTREE_DIR/merge_iter.rs"
fi

# ── Cycle-breaking concatenation ──────────────────────────────────
#
# Rust's stdlib btree has cyclic dependencies between sibling files:
# node ↔ navigate ↔ search ↔ merge_iter ↔ fix ↔ remove ↔ split ↔
# append, each adding `impl NodeRef<…>` orphan-impls that reference
# types declared in their own file. Rust handles this via name-lookup
# (modules are name-resolution units, not compilation units); C++20
# modules require the import graph be a DAG.
#
# The pragmatic fix: concatenate the cyclic group into a single
# Rust module (`btree_internal.rs`) before transpiling. The resulting
# `btree_port.btree.btree_internal.cppm` has all the interdependent
# types and impls in one TU, so there's no cycle.
#
# The merge target name is `btree_internal` (not `core`) because
# `core` collides with Rust's stdlib `core::*` crate and gets
# misnormalized to `std::*` by the transpiler's path-mapping table.
INTERNAL="$BTREE_DIR/btree_internal.rs"
# Idempotency: only do the merge if it hasn't been done already.
if [[ ! -f "$INTERNAL" ]]; then
  # Inline the module-level consts that the original btree submodules
  # used to reach via `super::*::FOO` (now `super::FOO` after my
  # earlier rewrite). Placing them in btree_internal.rs avoids the
  # parent↔child module cycle that would otherwise arise: the parent
  # `mod.rs` imports btree_internal (the merged module) AND
  # btree_internal would import its parent to reach the consts.
  # Defining them inside the merged file breaks that knot.
  {
    echo "// Auto-generated by docs/btreemap_port/prep.sh."
    echo "// Concatenated btree cyclic-module group. Order matters:"
    echo "// leaf utilities first (mem, borrow, set_val), then the"
    echo "// type-declaration files (node, search), then files that"
    echo "// add orphan impls (navigate, fix, remove, split, append)."
    echo ""
    echo "// MIN_LEN inlined here to avoid a parent↔child C++20 module"
    echo "// cycle (the original lived in btree/mod.rs and the merged"
    echo "// content reaches it via super::MIN_LEN; that import would"
    echo "// create a cycle with the parent module importing this one)."
    echo "// The other btree-level consts (B, CAPACITY, …) are already"
    echo "// declared in the merged content via node.rs's verbatim copy."
    echo "pub const MIN_LEN: usize = B - 1;"
    echo ""
  } > "$INTERNAL"
  for f in mem.rs borrow.rs set_val.rs dedup_sorted_iter.rs merge_iter.rs node.rs search.rs navigate.rs fix.rs remove.rs split.rs append.rs; do
    if [[ -f "$BTREE_DIR/$f" ]]; then
      echo "// === $f ===" >> "$INTERNAL"
      # Drop intra-group `use super::…;` statements (the referenced
      # symbols are now in the same file). Also drop `mod tests;`
      # declarations since the test files were stripped above.
      sed -E '
        /^use super::(node|search|navigate|merge_iter|fix|remove|split|append|mem|borrow|set_val|dedup_sorted_iter)(::|;)/d
        /^#\[cfg\(test\)\]$/{N;/\nmod tests;$/d;}
        /^mod tests;$/d
      ' "$BTREE_DIR/$f" >> "$INTERNAL"
      echo "" >> "$INTERNAL"
      rm "$BTREE_DIR/$f"
    fi
  done

  # Replace mod.rs with one that only declares the merged module
  # (plus map/set, which are the public-API entry points and have
  # their own subdirectories).
  cat > "$BTREE_DIR/mod.rs" <<MOD
pub(crate) mod btree_internal;
pub mod map;
pub mod set;

// Constants previously in this file. They were originally
// \`pub(super)\` (visible to the parent of the btree module — i.e.
// the alloc crate root), but the merged \`btree_internal\` module
// needs to see them too, and \`pub(super)\` doesn't grant that to
// child modules. Promote to plain \`pub\` since they're const
// numerics, not API surface that needs hiding.
pub const B: usize = 6;
pub const CAPACITY: usize = 2 * B - 1;
pub const MIN_LEN_AFTER_SPLIT: usize = B - 1;
pub const KV_IDX_CENTER: usize = B - 1;
pub const EDGE_IDX_LEFT_OF_CENTER: usize = B - 1;
pub const EDGE_IDX_RIGHT_OF_CENTER: usize = B;
pub const MIN_LEN: usize = B - 1;
MOD

  # Now rewrite all `use super::<old_submodule>::…;` references in
  # map.rs / set.rs / map/entry.rs / set/entry.rs to point at the
  # merged module. We iterate per-prefix because GNU sed's BRE
  # alternation requires escaped pipes and is fiddlier than a loop.
  for prefix in node search navigate merge_iter fix remove split append mem borrow set_val dedup_sorted_iter; do
    for target in "$BTREE_DIR/map.rs" "$BTREE_DIR/set.rs" "$BTREE_DIR/map/entry.rs" "$BTREE_DIR/set/entry.rs"; do
      if [[ -f "$target" ]]; then
        sed -i "s|super::${prefix}::|super::btree_internal::|g; s|use super::${prefix};|use super::btree_internal;|g" "$target"
      fi
    done
  done

  # Drop `use super::map::MIN_LEN;` lines from the merged file — the
  # consts are now defined inside btree_internal.rs itself, so there
  # is no parent or sibling to import from.
  sed -i '/^use super::map::MIN_LEN;$/d' "$INTERNAL"
  sed -i '/^use super::MIN_LEN;$/d' "$INTERNAL"

  # Intra-merge path rewrites. After the concatenation, every
  # `super::<submodule>::SYM` reference in the merged file points
  # at a symbol that's now in the SAME file — the path is wrong
  # because the submodule no longer exists. Strip these prefixes
  # so the reference becomes a plain identifier.
  #
  # We use word-boundary anchors to avoid mangling `core::mem::*`
  # (the actual stdlib core::mem, not our local merged mem.rs).
  sed -i -E '
    s|\bsuper::mem::([A-Za-z_])|\1|g
    s|\bsuper::borrow::([A-Za-z_])|\1|g
    s|\bsuper::set_val::([A-Za-z_])|\1|g
    s|\bsuper::node::([A-Za-z_])|\1|g
    s|\bsuper::search::([A-Za-z_])|\1|g
    s|\bsuper::navigate::([A-Za-z_])|\1|g
    s|\bsuper::merge_iter::([A-Za-z_])|\1|g
    s|\bsuper::fix::([A-Za-z_])|\1|g
    s|\bsuper::remove::([A-Za-z_])|\1|g
    s|\bsuper::split::([A-Za-z_])|\1|g
    s|\bsuper::append::([A-Za-z_])|\1|g
    s|\bsuper::dedup_sorted_iter::([A-Za-z_])|\1|g
  ' "$INTERNAL"

  # Same idea for bare `node::FOO` / `set_val::FOO` references that
  # the original code used (without the `super::` prefix) when they
  # referenced an item visible at the same scope. The merge folded
  # those scopes together; the prefix is no longer meaningful.
  sed -i -E '
    s|\bnode::([A-Z][A-Za-z_]*)|\1|g
    s|\bset_val::([A-Z][A-Za-z_]*)|\1|g
  ' "$INTERNAL"

  # set/entry.rs uses `use super::{SetValZST, map};` and then
  # references `map::OccupiedEntry` / `map::VacantEntry`. The
  # transpiler's `import` for a sibling module doesn't surface a
  # `map::` namespace alias — symbols become reachable at file
  # scope after import. Rewrite the two references to be
  # qualified-by-import-name shape that the transpiler does
  # support: rename them explicitly via use.
  if [[ -f "$BTREE_DIR/set/entry.rs" ]]; then
    sed -i \
      -e 's|use super::{SetValZST, map};|use super::btree_internal::SetValZST; use super::map::{OccupiedEntry as MapOccupiedEntry, VacantEntry as MapVacantEntry};|' \
      -e 's|map::OccupiedEntry<|MapOccupiedEntry<|g' \
      -e 's|map::VacantEntry<|MapVacantEntry<|g' \
      "$BTREE_DIR/set/entry.rs"
  fi

  # Clean up empty subdirectories left after stripping test files.
  rmdir "$BTREE_DIR/borrow" "$BTREE_DIR/node" 2>/dev/null || true
fi

echo "Port-prep complete for $BTREE_DIR"
