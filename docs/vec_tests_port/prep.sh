#!/usr/bin/env bash
# Pre-process rustc's vec.rs for transpilation.
set -euo pipefail
SRC="${1:?usage: prep.sh <lib.rs>}"
[[ -f "$SRC" ]] || { echo "error: $SRC is not a file" >&2; exit 1; }
sed -i \
  -e 's|use alloc::collections::|use std::collections::|g' \
  -e 's|use alloc::|use std::|g' \
  -e 's|use core::|use std::|g' \
  "$SRC"
echo "[vec_tests_port prep] normalized $SRC"
# fn-local structs/enums/impls become module-level items (C++ forbids local templates + friends).
python3 "$(dirname "${BASH_SOURCE[0]}")/prep_hoist.py" "$SRC"
# struct_with_counted_drop! invocations (shared alloctests macro) become
# module-level items with per-site `_L<line>` names, after the hoist so the
# hoist's own numbering matches the vendored port.
python3 "$(dirname "${BASH_SOURCE[0]}")/../vec_deque_tests_port/prep_expand.py" "$SRC"
