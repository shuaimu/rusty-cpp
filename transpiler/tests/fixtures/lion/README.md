# Lion snapshot

A copy of the Lion crates SRPC compiles, used by two transpiler tests:

- `src/verus_differential_tests.rs` (unit tests, in `verus_exec`): the
  differential erasure check. `--verus-exec` stage 1 runs on every source file
  under `tree/` and is compared, item by item, with `cargo expand --lib` of the
  same crate (`expanded/`).
- `tests/lion_parity.rs`: `parity/`'s insert, remove, advance and fire cases
  over lion-slab and lion-timer-wheel, transpiled with
  `--crate-graph --verus-exec`, compiled and run as C++, and compared with the
  Rust output recorded in `MANIFEST.toml`.

Neither test runs `cargo expand` or compiles vstd. Both read only this
directory, plus the Verus git checkout that Cargo needs to resolve `vstd` for
`lion_parity.rs` (the same checkout the `verus-erase` helper is built from).

## Contents

| Path | What it is |
| --- | --- |
| `tree/` | `Cargo.toml`, `src/` and `tests/` of the eight Lion crates in SRPC's dependency graph, verbatim. Lion is MIT-licensed; `LICENSE` is Lion's. |
| `expand/` | A workspace that depends on `lion-reactor` and `lion-executor` exactly as `srpc/Cargo.toml` does (`default-features = false`), with a lockfile derived from SRPC's. |
| `expanded/` | `cargo expand --lib -p <crate>` from `expand/`, unedited. |
| `parity/` | The parity cases (`src/lib.rs`), their Rust runner (`examples/run.rs`) and lockfile. |
| `MANIFEST.toml` | Generated: Lion revision, vstd source and Verus revision, rustc and cargo-expand versions, each crate's resolved features, and the parity cases' Rust output with a digest of their inputs. |
| `regen.sh` | Regenerates everything above except `parity/src` and `parity/examples`. |

## Regenerating

After a Lion pin bump in SRPC, or a Verus bump in `verus-erase`:

```sh
transpiler/tests/fixtures/lion/regen.sh <srpc>/third-party/lion <srpc>/Cargo.lock
```

It needs `cargo-expand`, `python3`, `rsync`, and the crates SRPC's lockfile
names in the local Cargo cache (or network access). It builds with rustup's
default `stable`, as SRPC's Rust lane does, not with this repository's
`rust-toolchain.toml`. It refuses a resolution that differs from SRPC's
lockfile. To check a new Lion revision before replacing the snapshot, pass a
third argument (an output directory) and run the tests against it with
`RUSTY_CPP_LION_FIXTURE=<dir>`.

The differential test pins its per-crate counts (`EXPECTED` in
`verus_differential_tests.rs`); a regenerated snapshot with a different
number of `verus!` items fails until those counts are updated deliberately.
The test also requires `MANIFEST.toml`'s `verus_git_rev` to equal the revision
the transpiler vendors, so a Verus bump without a regenerated snapshot fails.
