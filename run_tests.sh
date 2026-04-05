#!/bin/bash
# Script to run tests with environment defaults that work on both macOS and Linux.
set -euo pipefail

# Keep user-provided overrides if valid; otherwise avoid forcing missing paths.
if [[ -n "${Z3_SYS_Z3_HEADER:-}" && ! -f "${Z3_SYS_Z3_HEADER}" ]]; then
  unset Z3_SYS_Z3_HEADER
fi

if [[ "$(uname -s)" == "Darwin" ]]; then
  if [[ -z "${Z3_SYS_Z3_HEADER:-}" ]]; then
    for candidate in /opt/homebrew/include/z3.h /usr/local/include/z3.h; do
      if [[ -f "${candidate}" ]]; then
        export Z3_SYS_Z3_HEADER="${candidate}"
        break
      fi
    done
  fi

  llvm_lib_dir="/opt/homebrew/Cellar/llvm/19.1.7/lib"
  if [[ -d "${llvm_lib_dir}" ]]; then
    export DYLD_LIBRARY_PATH="${llvm_lib_dir}:${DYLD_LIBRARY_PATH:-}"
  fi
fi

cargo test "$@"
