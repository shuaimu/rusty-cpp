#!/usr/bin/env bash
# Post-clone prep for the hashbrown matrix crate.
#
# hashbrown's upstream Cargo.toml declares no [workspace], so when cloned under
# tests/transpile_tests/ it nests into rusty-cpp's own cargo workspace. cargo
# then resolves `-p hashbrown` ambiguously (rusty-cpp pulls hashbrown in
# transitively), which breaks the `cargo expand` step
# ("specification `hashbrown` is ambiguous"). Declaring an empty [workspace]
# makes the clone its own workspace root — exactly what indexmap's upstream
# Cargo.toml already does, which is why indexmap needs no such prep.
set -euo pipefail
crate_dir="${1:?usage: hashbrown.sh <crate_dir>}"
if ! grep -q '^\[workspace\]' "${crate_dir}/Cargo.toml"; then
    printf '\n[workspace]\n' >> "${crate_dir}/Cargo.toml"
    echo "[hashbrown prep] added standalone [workspace] (de-nest from rusty-cpp)"
fi
