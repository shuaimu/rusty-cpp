#!/bin/bash

# Script to run all tests for new Rusty types

echo "========================================="
echo "Running All Rusty C++ New Types Tests"
echo "========================================="
echo

FAILED=0
PASSED=0

# Test String
echo "1. Testing rusty::String..."
if ./test/rusty_string_test > /dev/null 2>&1; then
    echo "   ✅ String tests passed"
    ((PASSED++))
else
    echo "   ❌ String tests failed"
    ((FAILED++))
fi

# Test HashMap
echo "2. Testing rusty::HashMap..."
if ./test/rusty_hashmap_test > /dev/null 2>&1; then
    echo "   ✅ HashMap tests passed"
    ((PASSED++))
else
    echo "   ❌ HashMap tests failed"
    ((FAILED++))
fi

# Test HashSet
echo "3. Testing rusty::HashSet..."
if ./test/rusty_hashset_test > /dev/null 2>&1; then
    echo "   ✅ HashSet tests passed"
    ((PASSED++))
else
    echo "   ❌ HashSet tests failed"
    ((FAILED++))
fi

# Test BTreeMap
echo "4. Testing rusty::BTreeMap..."
if ./test/rusty_btreemap_test > /dev/null 2>&1; then
    echo "   ✅ BTreeMap tests passed"
    ((PASSED++))
else
    echo "   ❌ BTreeMap tests failed"
    ((FAILED++))
fi

# Test BTreeSet
echo "5. Testing rusty::BTreeSet..."
if ./test/rusty_btreeset_test > /dev/null 2>&1; then
    echo "   ✅ BTreeSet tests passed"
    ((PASSED++))
else
    echo "   ❌ BTreeSet tests failed"
    ((FAILED++))
fi

echo
echo "========================================="
echo "Test Summary:"
echo "  Passed: $PASSED/5"
echo "  Failed: $FAILED/5"

if [ $FAILED -eq 0 ]; then
    echo
    echo "🎉 All new Rusty types tests passed successfully!"
    exit 0
else
    echo
    echo "❌ Some tests failed. Please check the individual test outputs."
    exit 1
fi