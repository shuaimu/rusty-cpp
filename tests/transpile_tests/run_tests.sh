#!/bin/bash
# Transpile test suite: downloads popular Rust crates and tests our transpiler on them.
# Usage: ./run_tests.sh [crate_name]
#   No args: run all tests
#   With arg: run only the named test

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TRANSPILER="cargo run -p rusty-cpp-transpiler --"

cd "$SCRIPT_DIR"

PASS=0
FAIL=0
SKIP=0
TOTAL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

run_test() {
    local name="$1"
    local repo="$2"
    local version="$3"
    local description="$4"

    TOTAL=$((TOTAL + 1))

    # If a specific test was requested, skip others
    if [ -n "$TARGET_TEST" ] && [ "$name" != "$TARGET_TEST" ]; then
        SKIP=$((SKIP + 1))
        return
    fi

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Testing: $name ($description)"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Download if not present
    if [ ! -d "$name" ]; then
        echo "  Downloading $name $version..."
        git clone --depth 1 --branch "$version" "$repo" "$name" 2>/dev/null || {
            echo -e "  ${YELLOW}SKIP${NC}: Failed to download $name"
            SKIP=$((SKIP + 1))
            return
        }
    fi

    # Check Cargo.toml exists
    if [ ! -f "$name/Cargo.toml" ]; then
        echo -e "  ${YELLOW}SKIP${NC}: No Cargo.toml found"
        SKIP=$((SKIP + 1))
        return
    fi

    # Run transpiler in --crate mode
    local out_dir="$name/cpp_out"
    rm -rf "$out_dir"

    echo "  Transpiling..."
    if cd "$REPO_ROOT" && $TRANSPILER --crate "$SCRIPT_DIR/$name/Cargo.toml" --output-dir "$SCRIPT_DIR/$out_dir" 2>&1; then
        # Check that at least one .cppm file was generated
        local cppm_count=$(find "$SCRIPT_DIR/$out_dir" -name "*.cppm" 2>/dev/null | wc -l)
        local cmake_exists=$([ -f "$SCRIPT_DIR/$out_dir/CMakeLists.txt" ] && echo "yes" || echo "no")

        if [ "$cppm_count" -gt 0 ] && [ "$cmake_exists" = "yes" ]; then
            echo -e "  ${GREEN}PASS${NC}: $cppm_count .cppm files generated, CMakeLists.txt present"

            # Show generated files
            echo "  Generated files:"
            find "$SCRIPT_DIR/$out_dir" -name "*.cppm" -o -name "CMakeLists.txt" | sort | while read f; do
                local rel="${f#$SCRIPT_DIR/$out_dir/}"
                local lines=$(wc -l < "$f")
                echo "    $rel ($lines lines)"
            done

            PASS=$((PASS + 1))
        else
            echo -e "  ${RED}FAIL${NC}: Expected .cppm files and CMakeLists.txt"
            echo "    cppm files: $cppm_count, CMakeLists.txt: $cmake_exists"
            FAIL=$((FAIL + 1))
        fi
    else
        echo -e "  ${RED}FAIL${NC}: Transpilation failed"
        FAIL=$((FAIL + 1))
    fi

    cd "$SCRIPT_DIR"
    echo ""
}

# Parse args
TARGET_TEST="${1:-}"

echo "╔═══════════════════════════════════════════════════╗"
echo "║    Rusty-CPP Transpiler Integration Tests         ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""

# ── Tier 1: Trivial crates ──────────────────────────────

run_test "either" \
    "https://github.com/rayon-rs/either.git" \
    "1.13.0" \
    "Either<L,R> enum — enums, generics, traits"

run_test "tap" \
    "https://github.com/myrrlyn/tap.git" \
    "0.4.0" \
    "Method chaining — traits, generics, closures"

run_test "cfg-if" \
    "https://github.com/alexcrichton/cfg-if.git" \
    "1.0.0" \
    "Conditional compilation — minimal, macro-heavy"

# ── Tier 2: Small crates ────────────────────────────────

run_test "take_mut" \
    "https://github.com/Sgeo/take_mut.git" \
    "v0.2.2" \
    "Temporary ownership — unsafe, moves"

run_test "arrayvec" \
    "https://github.com/bluss/arrayvec.git" \
    "0.7.6" \
    "Fixed-capacity vec — generics, arrays, operators"

# ── Tier 3: Medium crates ───────────────────────────────

run_test "semver" \
    "https://github.com/dtolnay/semver.git" \
    "1.0.24" \
    "Version parsing — structs, enums, Display, Ord"

run_test "bitflags" \
    "https://github.com/bitflags/bitflags.git" \
    "2.6.0" \
    "Bit flag sets — operators, derives, macros"

# ── Summary ─────────────────────────────────────────────

echo "╔═══════════════════════════════════════════════════╗"
echo "║    Results                                        ║"
echo "╠═══════════════════════════════════════════════════╣"
printf "║  Total: %-3d  Pass: %-3d  Fail: %-3d  Skip: %-3d   ║\n" "$TOTAL" "$PASS" "$FAIL" "$SKIP"
echo "╚═══════════════════════════════════════════════════╝"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
