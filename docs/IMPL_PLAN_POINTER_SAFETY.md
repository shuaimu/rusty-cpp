# Implementation Plan: Pointer Safety Model Update

## Goal

Change the pointer safety model from:
- **Old**: Raw pointers mostly safe (like references), only address-of requires @unsafe
- **New**: Raw pointers fully unsafe, `rusty::Ptr<T>`/`rusty::MutPtr<T>` are safe wrappers

## Current State Analysis

Looking at `src/analysis/pointer_safety.rs`, the current implementation:

| Operation | Current Behavior | Target Behavior |
|-----------|------------------|-----------------|
| Raw pointer dereference (`*ptr`) | ❌ Requires @unsafe | ❌ Requires @unsafe |
| Address-of (`&x`) | ❌ Requires @unsafe | ❌ Requires @unsafe |
| Raw pointer declaration (`int* p`) | ✅ Allowed | ❌ Requires @unsafe |
| `rusty::Ptr<T>` dereference | ❌ Requires @unsafe | ✅ Safe |
| `rusty::addr_of()` | Not recognized | ✅ Safe |
| nullptr init/assign | ❌ Forbidden | ❌ Requires @unsafe |

**Key insight**: The analyzer already blocks dereference and address-of! The documentation (old pointer_safety.md) was wrong.

## Required Changes

### Phase 1: Add `rusty::Ptr<T>`/`rusty::MutPtr<T>` Detection

**File**: `src/analysis/pointer_safety.rs`

1. Add helper function to detect safe pointer types:

```rust
/// Check if a type is a safe rusty pointer type (Ptr<T> or MutPtr<T>)
fn is_rusty_safe_pointer_type(type_name: &str) -> bool {
    let normalized = type_name.replace(" ", "");

    // Check for rusty::Ptr<T> and rusty::MutPtr<T>
    normalized.starts_with("rusty::Ptr<") ||
    normalized.starts_with("rusty::MutPtr<") ||
    normalized.starts_with("Ptr<") ||  // Without namespace
    normalized.starts_with("MutPtr<") ||
    // Handle const variations
    normalized.starts_with("construsty::Ptr<") ||
    normalized.starts_with("construsty::MutPtr<")
}
```

2. Modify `contains_pointer_operation()` to whitelist `rusty::Ptr`/`rusty::MutPtr` dereferences:

```rust
Expression::Dereference(inner) => {
    // *this is safe in member functions
    if let Expression::Variable(name) = inner.as_ref() {
        if name == "this" {
            return None;
        }
    }

    // Check if we're dereferencing a rusty::Ptr or rusty::MutPtr
    // This requires type information - may need to track variable types
    // For now, check if the inner expression is a known safe pointer variable

    Some("dereference")
}
```

**Challenge**: The expression tree doesn't carry type information. We need to track variable types.

### Phase 2: Track Variable Types for Safe Pointer Detection

**File**: `src/analysis/pointer_safety.rs`

1. Add a set to track safe pointer variables:

```rust
pub fn check_parsed_function_for_pointers(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unsafe_depth = 0;
    let mut safe_pointer_vars: HashSet<String> = HashSet::new();

    // Collect safe pointer variables from parameters
    for param in &function.parameters {
        if is_rusty_safe_pointer_type(&param.type_name) {
            safe_pointer_vars.insert(param.name.clone());
        }
    }

    // ... rest of function
}
```

2. Update variable declaration handling:

```rust
Statement::VariableDecl(var) if var.is_pointer => {
    if is_rusty_safe_pointer_type(&var.type_name) {
        // Track as safe pointer variable
        safe_pointer_vars.insert(var.name.clone());
        return None;  // Safe pointer declaration allowed
    }

    // Raw pointer declarations are now forbidden in @safe code
    if !in_unsafe_scope {
        return Some(format!(
            "Raw pointer declaration '{}' at line {}: raw pointers are forbidden in @safe code. \
             Use rusty::Ptr<T> or rusty::MutPtr<T> instead.",
            var.name, var.location.line
        ));
    }
}
```

3. Pass safe pointer set to `contains_pointer_operation()`:

```rust
fn contains_pointer_operation(
    expr: &Expression,
    safe_pointer_vars: &HashSet<String>,
) -> Option<&'static str> {
    match expr {
        Expression::Dereference(inner) => {
            // *this is safe
            if let Expression::Variable(name) = inner.as_ref() {
                if name == "this" {
                    return None;
                }
                // Check if it's a safe pointer variable
                if safe_pointer_vars.contains(name) {
                    return None;  // Dereferencing Ptr<T>/MutPtr<T> is safe
                }
            }
            Some("dereference")
        }
        // ... rest of patterns
    }
}
```

### Phase 3: Whitelist `rusty::addr_of()` and `rusty::addr_of_mut()`

**File**: `src/analysis/pointer_safety.rs`

1. Add helper function:

```rust
/// Check if a function call is a safe rusty pointer function
fn is_rusty_safe_pointer_function(name: &str) -> bool {
    let base_name = name.rsplit("::").next().unwrap_or(name);
    let is_rusty_ns = name.starts_with("rusty::") || name.contains("::rusty::");

    // addr_of and addr_of_mut in rusty namespace
    if is_rusty_ns && (base_name == "addr_of" || base_name == "addr_of_mut") {
        return true;
    }

    // offset and offset_mut in rusty namespace
    if is_rusty_ns && (base_name == "offset" || base_name == "offset_mut") {
        return true;
    }

    // as_const is always safe (adding const)
    if is_rusty_ns && base_name == "as_const" {
        return true;
    }

    false
}
```

2. Add check in function call handling:

```rust
Statement::FunctionCall { name, args, location, .. } => {
    // Check for safe rusty pointer functions first
    if is_rusty_safe_pointer_function(name) {
        return None;  // Safe pointer functions are allowed
    }

    // Check for forbidden memory management functions
    if is_unsafe_memory_function(name) {
        return Some(format!(...));
    }

    // ... rest of checks
}
```

### Phase 4: Handle `rusty::as_mut()` as @unsafe

1. Add to unsafe functions:

```rust
fn is_rusty_unsafe_pointer_function(name: &str) -> bool {
    let base_name = name.rsplit("::").next().unwrap_or(name);
    let is_rusty_ns = name.starts_with("rusty::") || name.contains("::rusty::");

    // as_mut is unsafe (casting away const)
    if is_rusty_ns && base_name == "as_mut" {
        return true;
    }

    false
}
```

2. Add check:

```rust
if is_rusty_unsafe_pointer_function(name) && !in_unsafe_scope {
    return Some(format!(
        "Unsafe function '{}' at line {}: rusty::as_mut() casts away const and requires @unsafe.",
        name, location.line
    ));
}
```

### Phase 5: Update Tests

**File**: `src/analysis/pointer_safety.rs` (tests module)

Add new tests:

```rust
#[test]
fn test_raw_pointer_declaration_forbidden() {
    let stmt = Statement::VariableDecl(Variable {
        name: "ptr".to_string(),
        type_name: "int*".to_string(),
        is_pointer: true,
        // ...
    });

    let error = check_parsed_statement_for_pointers(&stmt, false, &HashSet::new());
    assert!(error.is_some(), "Raw pointer declaration should be forbidden");
}

#[test]
fn test_rusty_ptr_declaration_allowed() {
    let stmt = Statement::VariableDecl(Variable {
        name: "ptr".to_string(),
        type_name: "rusty::Ptr<int>".to_string(),
        is_pointer: true,
        // ...
    });

    let error = check_parsed_statement_for_pointers(&stmt, false, &HashSet::new());
    assert!(error.is_none(), "rusty::Ptr declaration should be allowed");
}

#[test]
fn test_rusty_ptr_dereference_allowed() {
    let mut safe_vars = HashSet::new();
    safe_vars.insert("ptr".to_string());

    let expr = Expression::Dereference(Box::new(Expression::Variable("ptr".to_string())));
    assert_eq!(contains_pointer_operation(&expr, &safe_vars), None);
}

#[test]
fn test_addr_of_function_allowed() {
    let stmt = Statement::FunctionCall {
        name: "rusty::addr_of".to_string(),
        args: vec![Expression::Variable("x".to_string())],
        // ...
    };

    let error = check_parsed_statement_for_pointers(&stmt, false, &HashSet::new());
    assert!(error.is_none(), "rusty::addr_of should be allowed");
}

#[test]
fn test_as_mut_requires_unsafe() {
    let stmt = Statement::FunctionCall {
        name: "rusty::as_mut".to_string(),
        args: vec![Expression::Variable("ptr".to_string())],
        // ...
    };

    let error = check_parsed_statement_for_pointers(&stmt, false, &HashSet::new());
    assert!(error.is_some(), "rusty::as_mut should require @unsafe");
}
```

### Phase 6: Integration Tests

**File**: `tests/test_pointer_safety.rs` (new or existing)

Create comprehensive integration tests:

```cpp
// test_safe_pointer_usage.cpp
#include <rusty/ptr.hpp>

// @safe
void test_rusty_ptr() {
    int x = 42;
    rusty::Ptr<int> p = rusty::addr_of(x);  // OK
    int y = *p;                              // OK
}

// @safe
void test_raw_ptr_forbidden() {
    int x = 42;
    int* p = &x;  // ERROR: raw pointer declaration forbidden
    int y = *p;   // ERROR: raw pointer dereference forbidden
}
```

## Implementation Order

1. **Phase 1-2**: Safe pointer type detection and variable tracking
   - Modify `is_rusty_safe_pointer_type()`
   - Add `safe_pointer_vars` tracking
   - Update `check_parsed_function_for_pointers()`
   - Update `contains_pointer_operation()` signature

2. **Phase 3-4**: Function whitelisting
   - Add `is_rusty_safe_pointer_function()`
   - Add `is_rusty_unsafe_pointer_function()`
   - Update function call handling

3. **Phase 5**: Unit tests
   - Update existing tests for new signature
   - Add new tests for safe pointer types

4. **Phase 6**: Integration tests
   - Create test C++ files
   - Verify end-to-end behavior

## Breaking Changes

This change will cause existing @safe code with raw pointers to fail:

```cpp
// @safe
void old_code(int* data) {  // NOW ERROR: raw pointer parameter
    *data = 42;              // NOW ERROR: raw pointer dereference
}
```

Users must migrate to:

```cpp
// @safe
void new_code(rusty::Ptr<int> data) {  // OK: safe pointer
    *data = 42;                         // OK: safe dereference
}
```

Or use @unsafe:

```cpp
// @unsafe
void legacy_code(int* data) {
    *data = 42;  // OK in @unsafe
}
```

## Estimated Effort

- Phase 1-2: 2-3 hours
- Phase 3-4: 1-2 hours
- Phase 5: 1-2 hours
- Phase 6: 1-2 hours
- **Total**: 5-9 hours

## Dependencies

- No external dependencies
- Requires parser to recognize `rusty::Ptr<T>` and `rusty::MutPtr<T>` type names
- May need to update the parser if type names aren't being captured correctly
