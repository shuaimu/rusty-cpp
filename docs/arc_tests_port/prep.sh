#!/usr/bin/env bash
# Pre-process rustc's arc.rs for transpilation.
set -euo pipefail
SRC="${1:?usage: prep.sh <lib.rs>}"
[[ -f "$SRC" ]] || { echo "error: $SRC is not a file" >&2; exit 1; }
sed -i \
  -e 's|use alloc::collections::|use std::collections::|g' \
  -e 's|use alloc::|use std::|g' \
  -e 's|use core::|use std::|g' \
  "$SRC"
echo "[arc_tests_port prep] normalized $SRC"
