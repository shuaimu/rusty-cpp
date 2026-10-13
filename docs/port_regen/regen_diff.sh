#!/usr/bin/env bash
# Reproducible regen-diff for a vendored stdlib port.
#
# Runs the FULL documented pipeline for one port —
#   copy pinned rust-src subtree -> prep.sh -> transpile -> post_transpile_patch.py
# — and diffs the result against the committed transpiled/<port>/ artifact.
# This is the anti-rot check: if the pipeline no longer reproduces the
# vendored port, this surfaces exactly what drifted.
#
# The toolchain (hence the stdlib SOURCE) is pinned by rust-toolchain.toml.
# See docs/port_regen/STATUS.md for the un-rot project + per-port status.
#
# Usage:  docs/port_regen/regen_diff.sh <port>        # e.g. vec_port
#         docs/port_regen/regen_diff.sh <port> --keep  # keep the work dir
set -uo pipefail

PORT="${1:?usage: regen_diff.sh <port> [--keep]}"
KEEP="${2:-}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

SRC_ROOT="$(rustc --print sysroot)/lib/rustlib/src/rust/library"
VENDORED="$REPO_ROOT/transpiled/$PORT"
PATCHER="$REPO_ROOT/docs/$PORT/post_transpile_patch.py"
PREP="$REPO_ROOT/docs/$PORT/prep.sh"
WORK="$(mktemp -d "/tmp/regen_${PORT}.XXXXXX")"
trap '[[ "$KEEP" == "--keep" ]] || rm -rf "$WORK"' EXIT

[[ -d "$VENDORED" ]] || { echo "no vendored port at $VENDORED" >&2; exit 2; }

# ── Per-port source subtree(s) + crate skeleton ──────────────────────────
# Each port vendors a different slice of rust-src. Add a case as ports are
# brought under this harness. mods = the `pub mod` lines for lib.rs.
mkdir -p "$WORK/crate/src"
case "$PORT" in
  vec_port)
    cp -r "$SRC_ROOT/alloc/src/vec"     "$WORK/crate/src/vec"
    cp -r "$SRC_ROOT/alloc/src/raw_vec" "$WORK/crate/src/raw_vec"
    [[ -f "$PREP" ]] && bash "$PREP" "$WORK/crate/src/vec" "$WORK/crate/src/raw_vec" >/dev/null
    mods=$'pub mod vec;\npub mod raw_vec;'
    ;;
  cell_port|rc_port|arc_port|string_port|binary_heap_port|linked_list_port|borrow_port|ascii_port)
    # Single-file ports: one rust-src file becomes src/lib.rs; the port's own
    # prep.sh rewrites it in place; its Cargo.toml.template (if any) is the
    # crate manifest (it carries the edition/lints the port was baselined on).
    case "$PORT" in
      cell_port)        src_file="$SRC_ROOT/core/src/cell.rs" ;;
      rc_port)          src_file="$SRC_ROOT/alloc/src/rc.rs" ;;
      arc_port)         src_file="$SRC_ROOT/alloc/src/sync.rs" ;;
      string_port)      src_file="$SRC_ROOT/alloc/src/string.rs" ;;
      binary_heap_port) src_file="$SRC_ROOT/alloc/src/collections/binary_heap/mod.rs" ;;
      linked_list_port) src_file="$SRC_ROOT/alloc/src/collections/linked_list.rs" ;;
      borrow_port)      src_file="$SRC_ROOT/alloc/src/borrow.rs" ;;
      ascii_port)       src_file="$SRC_ROOT/core/src/ascii/ascii_char.rs" ;;
    esac
    cp "$src_file" "$WORK/crate/src/lib.rs"
    prep_script="$PREP"
    # string_port was baselined with binary_heap_port's prep (its STATUS recipe).
    [[ "$PORT" == string_port && ! -f "$PREP" ]] && prep_script="$REPO_ROOT/docs/binary_heap_port/prep.sh"
    [[ -f "$prep_script" ]] && bash "$prep_script" "$WORK/crate/src/lib.rs" >/dev/null
    mods=""
    ;;
  vec_deque_port)
    # The vendored port is ONE module: collapse.py hoists the submodules into
    # lib.rs after prep (the Vec↔submodule import cycle C++20 modules forbid).
    cp "$SRC_ROOT"/alloc/src/collections/vec_deque/*.rs "$WORK/crate/src/"
    mv "$WORK/crate/src/mod.rs" "$WORK/crate/src/lib.rs"
    [[ -f "$PREP" ]] && bash "$PREP" "$WORK/crate/src/" >/dev/null
    python3 "$REPO_ROOT/docs/vec_deque_port/collapse.py" "$WORK/crate/src/" >/dev/null
    mods=""
    ;;
  core_slice_port)
    cp -r "$SRC_ROOT/core/src/slice" "$WORK/crate/src/slice"
    [[ -f "$PREP" ]] && bash "$PREP" "$WORK/crate/src/slice" >/dev/null
    mods=$'pub mod slice;'
    ;;
  btree_port)
    # docs/btreemap_port: the whole btree/ subtree under src/btree, its prep,
    # a `pub mod btree;` root (STATUS.md, "Regenerate").
    mkdir -p "$WORK/crate/src/btree"
    cp -r "$SRC_ROOT"/alloc/src/collections/btree/* "$WORK/crate/src/btree/"
    bash "$REPO_ROOT/docs/btreemap_port/prep.sh" "$WORK/crate/src/btree" >/dev/null
    PATCHER="$REPO_ROOT/docs/btreemap_port/post_transpile_patch.py"
    mods=$'pub mod btree;'
    ;;
  vec_tests_port|rc_tests_port|arc_tests_port|string_tests_port|vec_deque_tests_port|eq_diff_len_tests_port|btree_set_hash_tests_port|linked_list_tests_port|binary_heap_tests_port)
    # rustc's library/alloctests/tests/<name>.rs becomes src/lib.rs; the
    # port's prep.sh rewrites it in place.
    tname="${PORT%_tests_port}"
    # rust-src names the BinaryHeap test file `heap.rs`.
    [[ $tname == binary_heap ]] && tname=heap
    cp "$SRC_ROOT/alloctests/tests/$tname.rs" "$WORK/crate/src/lib.rs"
    [[ -f "$PREP" ]] && bash "$PREP" "$WORK/crate/src/lib.rs" >/dev/null
    mods=""
    ;;
  *)
    echo "port '$PORT' not yet configured in regen_diff.sh — add its" >&2
    echo "source subtree + prep invocation to the case statement." >&2
    exit 3
    ;;
esac

if [[ -f "$REPO_ROOT/docs/$PORT/Cargo.toml.template" ]]; then
  cp "$REPO_ROOT/docs/$PORT/Cargo.toml.template" "$WORK/crate/Cargo.toml"
else
cat > "$WORK/crate/Cargo.toml" <<EOF
[package]
name = "$PORT"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF
fi
# A single-file port's lib.rs IS the source; a multi-module port gets a root.
if [[ -n "$mods" ]]; then
  printf '#![allow(unused)]\n%s\n' "$mods" > "$WORK/crate/src/lib.rs"
fi

# ── Transpile + patch ────────────────────────────────────────────────────
echo "== transpiling $PORT (pinned $(rustc --version | awk '{print $2}')) =="
# --in-umbrella-closure: every port under this harness is RE-EXPORTED by the
# `rusty` umbrella (include/rusty/rusty.cppm), so it must not import the
# umbrella back — a module cycle — and names sibling collections by their deep
# path (`::rusty::port::vec::Vec`) over narrow imports. That is a build-graph
# fact only this pipeline knows; the transpiler defaults to the CONSUMER shape.
# --cxx-namespace: the vendored ports are wrapped in `namespace rusty::port::…`
# (the umbrella aliases `rusty::Vec` = `::rusty::port::vec::Vec`, …). The wrap
# is the port's public identity, not a transpiler default; each port names its
# own.
case "$PORT" in
  vec_port)          CXX_NS=rusty::port::vec ;;
  cell_port)         CXX_NS=rusty::port::cell ;;
  rc_port)           CXX_NS=rusty::port::rc ;;
  arc_port)          CXX_NS=rusty::port::sync ;;
  string_port)       CXX_NS=rusty::port::string ;;
  binary_heap_port)  CXX_NS=rusty::port::collections::binary_heap ;;
  linked_list_port)  CXX_NS=rusty::port::collections::linked_list ;;
  vec_deque_port)    CXX_NS=rusty::port::collections::vec_deque ;;
  hashbrown_port)    CXX_NS=rusty::port::collections::hashbrown ;;
  borrow_port)       CXX_NS=borrow_port ;;
  ascii_port)        CXX_NS=ascii_port ;;
  *_tests_port)      CXX_NS="$PORT" ;;   # the test ports' patchers anchor on `namespace <port> {`
  *)                 CXX_NS= ;;
esac
ns_flag=()
[[ -n "$CXX_NS" ]] && ns_flag=(--cxx-namespace "$CXX_NS")
"$REPO_ROOT/target/release/rusty-cpp-transpiler" \
  --crate "$WORK/crate/Cargo.toml" --in-umbrella-closure "${ns_flag[@]}" --output-dir "$WORK/out" > "$WORK/transpile.log" 2>&1
tx=$?
echo "transpile exit=$tx  ($(grep -c error: "$WORK/transpile.log" 2>/dev/null) error lines)"
[[ -f "$PATCHER" ]] && python3 "$PATCHER" "$WORK/out" > "$WORK/patch.log" 2>&1
echo "patcher: applied=$(grep -c '^  applied:' "$WORK/patch.log" 2>/dev/null)  skipped=$(grep -c '^  skipped:' "$WORK/patch.log" 2>/dev/null)  (skipped == 'already-applied OR anchor-drifted-no-op')"

# ── Diff (content-level, module-boundary-agnostic) ───────────────────────
# Concatenate + normalize both sides so the comparison is about CONTENT
# presence, not the .cppm the transpiler happened to split it into.
norm() { cat "$1"/*.cppm | sed 's/[[:space:]]\+/ /g; s/^ //; s/ $//' | grep -v '^$' | sort -u; }
comm -23 <(norm "$VENDORED") <(norm "$WORK/out") > "$WORK/only_vendored.txt"
comm -13 <(norm "$VENDORED") <(norm "$WORK/out") > "$WORK/only_regen.txt"

echo
echo "== $PORT: regen vs vendored =="
echo "  vendored modules: $(ls "$VENDORED"/*.cppm | wc -l)   regen modules: $(ls "$WORK/out"/*.cppm | wc -l)"
echo "  lines ONLY in vendored (candidate lost hand-fixes): $(wc -l < "$WORK/only_vendored.txt")"
echo "  lines ONLY in regen    (prelude/transpiler drift):  $(wc -l < "$WORK/only_regen.txt")"
[[ "$KEEP" == "--keep" ]] && echo "  work dir kept: $WORK"
echo
echo "  → inspect $WORK/only_vendored.txt for content the recipe fails to"
echo "    reproduce (real losses, minus stale-prelude noise)."
