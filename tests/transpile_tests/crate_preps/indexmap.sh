#!/usr/bin/env bash
# Post-clone prep for the indexmap matrix crate.
#
# indexmap's `quick` integration test uses quickcheck, which pulls
# rand -> getrandom -> libc. libc is raw C FFI and cannot be transpiled, so the
# whole crate fails at Stage C ("unresolved external crate imports: libc").
# Drop that one test; the remaining tests (equivalent_trait, macros_full_path,
# tests) exercise IndexMap without any libc-dependent crate, and cargo then
# prunes quickcheck/rand/getrandom as unused.
set -euo pipefail
crate_dir="${1:?usage: indexmap.sh <crate_dir>}"
rm -f "${crate_dir}/tests/quick.rs"
echo "[indexmap prep] removed tests/quick.rs (quickcheck -> rand -> getrandom -> libc)"
