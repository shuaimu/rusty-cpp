#!/usr/bin/env bash
# Pre-process rustc's library/alloc/src/collections/btree/{map,set}/tests.rs
# for transpilation as a standalone crate.
#
# Two files we collapse into one src/lib.rs:
#   - map/tests.rs (117 #[test])
#   - set/tests.rs (33 #[test])
#
# Plus the test-helper modules from library/alloc/src/testing/:
#   - crash_test.rs (CrashTestDummy / Panic)
#   - ord_chaos.rs (Cyclic3 / Governed / Governor / IdBased)
#   - rng.rs       (DeterministicRng — already in transpiled/testing_port)
#
# The pipeline is the same as docs/btree_set_hash_tests_port/prep.sh — but
# because map/tests.rs uses `use super::*;` and `crate::testing::*` to reach
# internal types, we don't try to make it transpile cleanly on its own.
# Instead the stub .cppm registers each #[test] as a skip; un-stubbing
# happens incrementally as the surface gets ported.
#
# Usage:  bash prep.sh <lib.rs>     (rewrites crate-relative paths in place)

set -euo pipefail
SRC="${1:?usage: prep.sh <lib.rs>}"
[[ -f "$SRC" ]] || { echo "error: $SRC is not a file" >&2; exit 1; }
sed -i \
  -e 's|use alloc::collections::|use std::collections::|g' \
  -e 's|use alloc::|use std::|g' \
  -e 's|use core::|use std::|g' \
  "$SRC"
echo "[btree_tests_port prep] normalized $SRC"
