# Transpiler Integration Tests

Tests the rusty-cpp transpiler against real-world Rust crates.

## Usage

```bash
# Run all tests (downloads crates on first run)
./run_tests.sh

# Run a specific test
./run_tests.sh either
./run_tests.sh semver

# Run parity matrix (baseline + expand + transpile + build + run)
./run_parity_matrix.sh

# Run parity matrix for one crate
./run_parity_matrix.sh --crate either

# Run cpp::std complex compile-stage fixture check
./run_cpp_std_complex_compile.sh
```

## Test Crates

### Tier 1: Trivial (~100 LOC)
| Crate | What it tests |
|-------|--------------|
| `either` | Enums with data, generics, trait impls, closures |
| `tap` | Trait definitions, generics, closures, method chaining |
| `cfg-if` | Conditional compilation, minimal macro-heavy crate |

### Tier 2: Small (~200-500 LOC)
| Crate | What it tests |
|-------|--------------|
| `take_mut` | Unsafe blocks, move semantics |
| `arrayvec` | Generics, arrays, operator overloading |

### Tier 3: Medium (~500-1500 LOC)
| Crate | What it tests |
|-------|--------------|
| `semver` | Structs, enums, Display, FromStr, comparison operators |
| `bitflags` | Operator overloading, derive-like patterns, macros |

### Tier 4: Complex (~1500+ LOC / heavier feature mix)
| Crate | What it tests |
|-------|--------------|
| `smallvec` | Const generics, inline storage, unsafe internals, drop/move semantics |
| `itertools` | Iterator adapter chains, closures, trait-heavy generic APIs |
| `once_cell` | Static initialization, interior mutability, sync and thread-safe paths |

### Tier 5: Async runtime surface (focused)
| Crate | What it tests |
|-------|--------------|
| `pollster` | Minimal `block_on` executor, `Future`/`Poll`/waker interaction |

### Tier 6: Serialization ecosystem (workspace crates)
| Crate | What it tests |
|-------|--------------|
| `serde_core` | Core serialization traits, derives expansions, no-std oriented surfaces |
| `serde` | Full serde crate wiring on top of `serde_core` with broader API coverage |
| `toml` | Serde-backed TOML parser/serializer with medium-complex real-world data model coverage |

## What the tests check

1. Transpilation succeeds (no panics, no errors)
2. At least one `.cppm` file is generated per source file
3. `CMakeLists.txt` is generated
4. File count and line counts are reported

## Notes

- Crate sources are downloaded via `git clone --depth 1` on first run
- Downloaded sources are gitignored (only the test script is tracked)
- Transpiled output goes to `<crate>/cpp_out/` (also gitignored)
- Parity matrix outputs go to `.rusty-parity-matrix/<crate>/` by default
