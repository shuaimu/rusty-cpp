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
# drop the test/bench sources.
#
# But NOT an EMPTY dev-dependency list. `rm -rf tests benches` removes the
# external test dirs; it does not touch the in-src `#[cfg(test)]` modules, and
# Stage A builds the lib WITH cfg(test) (a dependency never is — which is
# exactly why this only bites standalone). Those modules need two things, and
# dropping them left Stage A unable to compile at all, so the crate never
# reached the transpiler:
#   * `allocator-api2` — upstream has it in `default`; without it
#     `self::inner::AllocError` (src/alloc.rs) does not resolve;
#   * `rand` — src/map.rs's test module imports Rng/SeedableRng/SmallRng;
#   * `libc` (cfg(unix)) — the same module spells `libc::size_t` inline.
# Measured: those two are the ONLY externals the in-src tests use (`stdalloc`
# is hashbrown's own alias for the `alloc` crate). criterion/rayon/bumpalo/
# serde_test were only ever needed by the deleted tests/ and benches/ dirs, so
# the heavy tree this prep exists to avoid stays avoided.
set -euo pipefail
crate_dir="${1:?usage: hashbrown.sh <crate_dir>}"
cd "${crate_dir}"
rm -rf tests benches
cat > Cargo.toml <<'TOML'
[package]
name = "hashbrown"
version = "0.17.1"
edition = "2021"

# Doc examples are never transpiled, so they cannot take part in a parity
# comparison — and two of them (`HashMap::new_in`, `with_capacity_in`) pull
# `bumpalo`, which is exactly the heavy tree this manifest exists to avoid.
# The 105 in-src unit tests remain as the behavioral oracle.
[lib]
doctest = false

[dependencies]
foldhash = { version = "0.2.0", default-features = false, optional = true }
equivalent = { version = "1.0", default-features = false, optional = true }
allocator-api2 = { version = "0.2.9", default-features = false, features = ["alloc"], optional = true }

# Only what the in-src #[cfg(test)] modules import. The external tests/ and
# benches/ dirs are deleted above, so their criterion/rayon/bumpalo/serde_test
# tree is deliberately absent.
[dev-dependencies]
# default-features=false drops `getrandom` (hence `libc`, which the transpiler
# cannot lower) — the tests only ever build `SmallRng::seed_from_u64`, so no
# OS entropy source is needed.
rand = { version = "0.9.0", default-features = false, features = ["small_rng"] }

# src/map.rs's allocator test spells `libc::size_t` inline (no `use`), so a
# grep for `use <crate>::` in the test modules misses it — upstream carries it
# under the same cfg.
[target.'cfg(unix)'.dev-dependencies]
libc = "0.2.155"

[features]
default = ["default-hasher", "inline-more", "allocator-api2", "equivalent", "raw-entry"]
default-hasher = ["dep:foldhash"]
equivalent = ["dep:equivalent"]
allocator-api2 = ["dep:allocator-api2"]
raw-entry = []
inline-more = []

[workspace]
TOML
echo "[hashbrown prep] replaced manifest with minimal default-feature lib-only Cargo.toml"
