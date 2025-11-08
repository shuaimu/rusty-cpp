#!/bin/bash

# Script to compile and run all rusty type tests

echo "Building and running Rusty types tests..."
echo "========================================="

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Track failures
FAILED=0
TOTAL=0

# Compile flags
CXX="clang++"
CXXFLAGS="-std=c++17 -Wall -Wextra -I../include"

# List of test files
TESTS=(
    "rusty_box_test"
    "rusty_arc_test"
    "rusty_rc_test"
    "rusty_vec_test"
    "rusty_option_test"
    "rusty_result_test"
    "rusty_rwlock_test"
    "rusty_condvar_test"
    "rusty_barrier_test"
    "rusty_once_test"
)

# Create build directory if it doesn't exist
mkdir -p build

# Compile and run each test
for test in "${TESTS[@]}"; do
    echo ""
    echo "Building $test..."
    
    if $CXX $CXXFLAGS "$test.cpp" -o "build/$test" -pthread 2>/dev/null; then
        echo "Running $test..."
        if "./build/$test"; then
            echo -e "${GREEN}✓ $test passed${NC}"
        else
            echo -e "${RED}✗ $test failed${NC}"
            ((FAILED++))
        fi
    else
        echo -e "${RED}✗ $test compilation failed${NC}"
        ((FAILED++))
    fi
    ((TOTAL++))
done

echo ""
echo "========================================="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All $TOTAL tests passed!${NC}"
    exit 0
else
    echo -e "${RED}$FAILED out of $TOTAL tests failed${NC}"
    exit 1
fi