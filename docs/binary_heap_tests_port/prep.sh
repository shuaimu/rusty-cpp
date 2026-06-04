#!/usr/bin/env bash
# Pre-process the vendored rustc binary_heap.rs test file for transpilation.
# Normalizes the alloctests-flavored imports (`use alloc::...`, `use crate::testing::...`)
# to `use std::...` paths so the transpiler resolves them against rusty.hpp.
#
# Usage:
#   RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloctests/tests/collections/ | head -1)
#   TGT=/tmp/binary_heap_tests_port
#   mkdir -p $TGT/heap_tests_crate/src
#   cp $RUSTSRC/binary_heap.rs $TGT/heap_tests_crate/src/lib.rs
#   cp docs/binary_heap_tests_port/Cargo.toml.template $TGT/heap_tests_crate/Cargo.toml
#   bash docs/binary_heap_tests_port/prep.sh $TGT/heap_tests_crate/src/lib.rs
set -euo pipefail

SRC="${1:?usage: prep.sh <lib.rs>}"
if [[ ! -f "$SRC" ]]; then
  echo "error: $SRC is not a file" >&2
  exit 1
fi

sed -i \
  -e 's|use alloc::boxed::Box|use std::boxed::Box|g' \
  -e 's|use alloc::collections::|use std::collections::|g' \
  -e 's|use alloc::vec::|use std::vec::|g' \
  -e 's|use alloc::string::|use std::string::|g' \
  -e 's|use crate::testing::|use std::testing::|g' \
  -e 's|use core::|use std::|g' \
  "$SRC"

echo "[binary_heap_tests_port prep] normalized $SRC"
