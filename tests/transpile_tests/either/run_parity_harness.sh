#!/usr/bin/env bash
set -euo pipefail

# Thin compatibility wrapper — forwards to the generic `parity-test` subcommand.
# All either-specific logic has been moved into the transpiler binary.
#
# Usage:
#   ./run_parity_harness.sh [options]
#
# Options are forwarded to `rusty-cpp-transpiler parity-test`.
# See `rusty-cpp-transpiler parity-test --help` for full list.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
EITHER_MANIFEST="${SCRIPT_DIR}/Cargo.toml"

cd "${REPO_ROOT}"

exec cargo run -p rusty-cpp-transpiler -- \
    parity-test \
    --manifest-path "${EITHER_MANIFEST}" \
    "$@"
