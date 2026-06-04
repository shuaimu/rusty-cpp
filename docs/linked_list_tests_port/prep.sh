#!/usr/bin/env bash
# Pre-process rustc's library/alloctests/tests/linked_list.rs for transpilation.
# The test uses `crate::hash` (a 5-line helper in alloctests/tests/lib.rs);
# we prepend a local copy so the file is self-contained.
#
# Usage:
#   RUSTSRC=$(ls -d ~/.rustup/toolchains/*/lib/rustlib/src/rust/library/alloctests/tests/ | head -1)
#   TGT=/tmp/linked_list_tests_port
#   mkdir -p $TGT/heap_tests_crate/src
#   cp $RUSTSRC/linked_list.rs $TGT/heap_tests_crate/src/lib.rs
#   cp docs/linked_list_tests_port/Cargo.toml.template $TGT/heap_tests_crate/Cargo.toml
#   bash docs/linked_list_tests_port/prep.sh $TGT/heap_tests_crate/src/lib.rs
set -euo pipefail

SRC="${1:?usage: prep.sh <lib.rs>}"
if [[ ! -f "$SRC" ]]; then
  echo "error: $SRC is not a file" >&2
  exit 1
fi

# Replace `use crate::hash;` with an inline hash helper.
if ! grep -q "fn hash<" "$SRC"; then
    sed -i 's|use crate::hash;|use std::hash::{BuildHasher, Hash, BuildHasherDefault};\nuse std::collections::hash_map::DefaultHasher;\nfn hash<T: Hash>(t: \&T) -> u64 {\n    BuildHasherDefault::<DefaultHasher>::default().hash_one(t)\n}|' "$SRC"
fi

# Normalize alloc:: → std::
sed -i \
  -e 's|use alloc::collections::|use std::collections::|g' \
  -e 's|use core::|use std::|g' \
  "$SRC"

echo "[linked_list_tests_port prep] normalized $SRC"
