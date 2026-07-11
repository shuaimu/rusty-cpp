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
  *)
    echo "port '$PORT' not yet configured in regen_diff.sh — add its" >&2
    echo "source subtree + prep invocation to the case statement." >&2
    exit 3
    ;;
esac

cat > "$WORK/crate/Cargo.toml" <<EOF
[package]
name = "$PORT"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
EOF
printf '#![allow(unused)]\n%s\n' "$mods" > "$WORK/crate/src/lib.rs"

# ── Transpile + patch ────────────────────────────────────────────────────
echo "== transpiling $PORT (pinned $(rustc --version | awk '{print $2}')) =="
"$REPO_ROOT/target/release/rusty-cpp-transpiler" \
  --crate "$WORK/crate/Cargo.toml" --output-dir "$WORK/out" > "$WORK/transpile.log" 2>&1
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
