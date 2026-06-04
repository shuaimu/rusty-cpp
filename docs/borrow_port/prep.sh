#!/usr/bin/env bash
# Pre-process rustc's alloc/src/borrow.rs for transpilation.
# Normalises core:: → std:: and crate:: → std:: so the transpiler
# resolves imports against the already-vendored rusty std surface.
set -euo pipefail
SRC="${1:?usage: prep.sh <lib.rs>}"
[[ -f "$SRC" ]] || { echo "error: $SRC is not a file" >&2; exit 1; }

# The original `use Cow::*;` triggers variant glob; keep it.
# Normalise std-internal `crate::` references to `std::` so the
# transpiler resolves them against rusty.cppm.
sed -i \
  -e 's|use core::|use std::|g' \
  -e 's|use crate::fmt|use std::fmt|g' \
  -e 's|use crate::string|use std::string|g' \
  "$SRC"
echo "[borrow_port prep] normalized $SRC"
