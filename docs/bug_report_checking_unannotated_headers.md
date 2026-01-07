# Bug Report: Checker Analyzes Unannotated Third-Party Header Code

## Summary

The rusty-cpp checker is analyzing functions from third-party headers (like yaml-cpp) that have no `@safe` annotations. It should only check code that is explicitly marked `@safe`.

## Expected Behavior

The checker should only analyze:
1. Functions explicitly marked `@safe`
2. Functions in namespaces/classes/files marked `@safe`

Third-party headers without any annotations should be completely skipped.

## Actual Behavior

When checking a file like `communicator.cc` that includes yaml-cpp headers, the checker reports violations from yaml-cpp code:

```
Cannot modify field 'm_pNode' in const method
Cannot move out of 'rhs' because it is behind a reference
```

These violations come from `yaml-cpp/include/yaml-cpp/node/node.h` and `yaml-cpp/include/yaml-cpp/node/impl.h`, which have zero `@safe` annotations.

## Minimal Reproduction

```cpp
// test_third_party_headers.cpp
#include <yaml-cpp/yaml.h>  // Third-party, no @safe annotations

// @safe
void my_function() {
    YAML::Node node;
    node["key"] = "value";  // This triggers violations from yaml-cpp internals
}
```

## Analysis

When the checker analyzes `my_function()`, it follows calls into yaml-cpp code:
- `YAML::Node::operator[]`
- `YAML::Node::operator=`

These yaml-cpp functions use patterns that trigger violations:
1. `mutable` fields modified in const methods (`m_pNode`)
2. Copy assignment with `const&` parameters (misinterpreted as move)

But yaml-cpp is not annotated with `@safe`, so the checker should not be analyzing it.

## Root Cause

The checker appears to be analyzing ALL functions encountered during analysis, not just those marked `@safe`. When a `@safe` function calls into unannotated code, the checker follows the call and checks the unannotated code.

## Proposed Fix

1. When entering a function, check if it (or its containing class/namespace) is marked `@safe`
2. If not annotated, skip analysis of that function entirely
3. Only report violations for code that is explicitly in the `@safe` scope

This matches Rust's model: unsafe code is not checked by the borrow checker, only safe code is.

## Impact

This bug makes it impractical to use rusty-cpp on any codebase that includes third-party headers, as violations from those headers pollute the output.
