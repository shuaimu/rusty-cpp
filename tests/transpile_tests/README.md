# Transpiler Integration Tests

Tests the rusty-cpp transpiler against real-world Rust crates.

## Usage

```bash
# Run all tests (downloads crates on first run)
./run_tests.sh

# Run a specific test
./run_tests.sh either
./run_tests.sh semver
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

## What the tests check

1. Transpilation succeeds (no panics, no errors)
2. At least one `.cppm` file is generated per source file
3. `CMakeLists.txt` is generated
4. File count and line counts are reported

## Notes

- Crate sources are downloaded via `git clone --depth 1` on first run
- Downloaded sources are gitignored (only the test script is tracked)
- Transpiled output goes to `<crate>/cpp_out/` (also gitignored)
