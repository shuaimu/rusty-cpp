#!/usr/bin/env bash
# Post-clone prep for the hashbrown matrix crate.
#
# hashbrown's real Cargo.toml carries a heavy dev-dependency tree (criterion ->
# cast, rayon, serde_test, bumpalo, ...) plus a tangled feature web
# (nightly <-> rustc-dep-of-std <-> bumpalo) that make the *standalone* crate
# impossible to transpile within a matrix timeout — even though hashbrown's LIB
# transpiles fine (a dependency only ever pulls the lib, not the dev tree, which
# is why hashbrown comes through cleanly inside the serde_yaml / indexmap
# builds). So replace the manifest with a minimal one that builds only the
# default-feature lib — exactly the shape that already transpiles as a dep — and
# drop the test/bench sources. Standalone hashbrown then reaches Stage C fast and
# surfaces its own lib codegen bugs (currently the same HashMap<auto,..>
# element-inference leak that indexmap hits with IndexMap<auto,..>).
set -euo pipefail
crate_dir="${1:?usage: hashbrown.sh <crate_dir>}"
cd "${crate_dir}"
rm -rf tests benches
cat > Cargo.toml <<'TOML'
[package]
name = "hashbrown"
version = "0.17.1"
edition = "2021"

[dependencies]
foldhash = { version = "0.2.0", default-features = false, optional = true }
equivalent = { version = "1.0", default-features = false, optional = true }

[features]
default = ["default-hasher", "inline-more", "equivalent", "raw-entry"]
default-hasher = ["dep:foldhash"]
equivalent = ["dep:equivalent"]
raw-entry = []
inline-more = []

[workspace]
TOML
echo "[hashbrown prep] replaced manifest with minimal default-feature lib-only Cargo.toml"
