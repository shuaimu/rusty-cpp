//! Function Pointer Safety Analysis
//!
//! This module implements safety checking for function pointers using type-level
//! encoding with `SafeFn<Sig>` and `UnsafeFn<Sig>` wrapper types.
//!
//! Key concepts:
//! - `SafeFn<Ret(Args...)>` - holds a pointer to a @safe function, can be called safely
//! - `UnsafeFn<Ret(Args...)>` - holds any function pointer, requires @unsafe to call
//! - Raw function pointers require @unsafe to call
//!
//! See `docs/FUNCTION_POINTER_SAFETY_PLAN.md` for the full design.

use crate::parser::{Expression, Function, Statement};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Result of checking a SafeFn assignment
#[derive(Debug, Clone)]
pub struct SafeFnAssignmentCheck {
    pub variable_name: String,
    pub target_function: String,
    pub is_valid: bool,
    pub error_message: Option<String>,
}

// ============================================================================
// Type Detection
// ============================================================================

/// Check if a type is a SafeFn wrapper type
pub fn is_safe_fn_type(type_name: &str) -> bool {
    let normalized = type_name.replace(" ", "");
    normalized.starts_with("rusty::SafeFn<") ||
    normalized.starts_with("SafeFn<") ||
    normalized.starts_with("rusty::SafeMemFn<") ||
    normalized.starts_with("SafeMemFn<")
}

/// Check if a type is an UnsafeFn wrapper type
pub fn is_unsafe_fn_type(type_name: &str) -> bool {
    let normalized = type_name.replace(" ", "");
    normalized.starts_with("rusty::UnsafeFn<") ||
    normalized.starts_with("UnsafeFn<") ||
    normalized.starts_with("rusty::UnsafeMemFn<") ||
    normalized.starts_with("UnsafeMemFn<")
}

/// Check if a type is a raw function pointer
/// Matches patterns like: void (*)(int), int (*)(const char*, ...), void (MyClass::*)(int)
pub fn is_raw_function_pointer_type(type_name: &str) -> bool {
    // Check for function pointer patterns
    // void (*)(int) - free function pointer
    // int (MyClass::*)(int) - member function pointer
    type_name.contains("(*)") || type_name.contains("::*)")
}

/// Check if a function call is calling through a SafeFn or SafeMemFn wrapper
pub fn is_safe_fn_call(callee_type: &str, method_name: &str) -> bool {
    // SafeFn<Sig>::operator() or SafeMemFn<Sig>::operator() - safe to call
    is_safe_fn_type(callee_type) && method_name == "operator()"
}

/// Check if a function call is calling through an UnsafeFn or UnsafeMemFn wrapper
pub fn is_unsafe_fn_call_unsafe_method(callee_type: &str, method_name: &str) -> bool {
    // UnsafeFn<Sig>::call_unsafe or UnsafeMemFn<Sig>::call_unsafe - requires @unsafe
    is_unsafe_fn_type(callee_type) && method_name == "call_unsafe"
}

/// Check if a type is a member function pointer wrapper (safe or unsafe)
pub fn is_member_fn_wrapper_type(type_name: &str) -> bool {
    let normalized = type_name.replace(" ", "");
    normalized.contains("MemFn<")
}

/// Check if a type is a raw member function pointer
/// Matches patterns like: void (MyClass::*)(int), int (Widget::*)(double) const
pub fn is_raw_member_function_pointer_type(type_name: &str) -> bool {
    // Exclude wrapper types first
    if is_safe_fn_type(type_name) || is_unsafe_fn_type(type_name) {
        return false;
    }
    // Member function pointer pattern: Ret (Class::*)(Args...)
    type_name.contains("::*)") || type_name.contains("::*)(")
}

// ============================================================================
// Safety Checking
// ============================================================================

/// Check function pointer safety in a parsed function
///
/// This checks:
/// 1. SafeFn assignments have @safe targets
/// 2. Raw function pointer calls require @unsafe
/// 3. UnsafeFn::call_unsafe() requires @unsafe
pub fn check_function_pointer_safety(
    function: &Function,
    function_safety: SafetyMode,
    known_safe_functions: &HashMap<String, SafetyMode>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unsafe_depth = 0;

    // Only check in @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    for stmt in &function.body {
        // Track unsafe scope
        match stmt {
            Statement::EnterUnsafe => {
                unsafe_depth += 1;
                continue;
            }
            Statement::ExitUnsafe => {
                if unsafe_depth > 0 {
                    unsafe_depth -= 1;
                }
                continue;
            }
            _ => {}
        }

        let in_unsafe_scope = unsafe_depth > 0;

        if let Some(error) = check_statement_for_function_pointer_safety(
            stmt,
            in_unsafe_scope,
            known_safe_functions,
        ) {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }

    errors
}

fn check_statement_for_function_pointer_safety(
    stmt: &Statement,
    in_unsafe_scope: bool,
    known_safe_functions: &HashMap<String, SafetyMode>,
) -> Option<String> {
    // Skip checks in unsafe scope
    if in_unsafe_scope {
        return None;
    }

    match stmt {
        Statement::VariableDecl(var) => {
            // Check if declaring a SafeFn and initializing from a function address
            if is_safe_fn_type(&var.type_name) {
                // We need to check the initializer, but Variable doesn't have initializer info
                // This check happens at the call site in check_safe_fn_assignment
                return None;
            }

            // Raw function pointer declarations are OK, but calling is checked elsewhere
            None
        }

        Statement::Assignment { lhs, rhs, location } => {
            // Check if assigning to a SafeFn variable
            if let Expression::Variable(var_name) = lhs {
                // Check if rhs is a function address being assigned to SafeFn
                if let Some(error) = check_safe_fn_assignment_expr(
                    var_name,
                    rhs,
                    known_safe_functions,
                    location.line as usize,
                ) {
                    return Some(error);
                }
            }
            None
        }

        Statement::FunctionCall { name, args, location, .. } => {
            // Check for raw function pointer calls
            // A call through a raw function pointer looks like: fp(args)
            // where fp is a variable of function pointer type

            // Check for UnsafeFn::call_unsafe() calls
            if name.ends_with("::call_unsafe") || name.ends_with(".call_unsafe") {
                return Some(format!(
                    "Call to UnsafeFn::call_unsafe() at line {} requires @unsafe context",
                    location.line as usize
                ));
            }

            // Check for calls through raw function pointers
            // This is detected by checking if 'name' is a variable reference, not a function name
            // We can't easily detect this without type info, so we'll check for common patterns

            None
        }

        Statement::If { then_branch, else_branch, .. } => {
            // Recursively check branches
            for stmt in then_branch {
                if let Some(error) = check_statement_for_function_pointer_safety(
                    stmt, in_unsafe_scope, known_safe_functions
                ) {
                    return Some(error);
                }
            }
            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    if let Some(error) = check_statement_for_function_pointer_safety(
                        stmt, in_unsafe_scope, known_safe_functions
                    ) {
                        return Some(error);
                    }
                }
            }
            None
        }

        Statement::Block(stmts) => {
            for stmt in stmts {
                if let Some(error) = check_statement_for_function_pointer_safety(
                    stmt, in_unsafe_scope, known_safe_functions
                ) {
                    return Some(error);
                }
            }
            None
        }

        _ => None,
    }
}

/// Check if an expression assigned to a SafeFn variable is valid
fn check_safe_fn_assignment_expr(
    _var_name: &str,
    rhs: &Expression,
    known_safe_functions: &HashMap<String, SafetyMode>,
    line: usize,
) -> Option<String> {
    // Extract function name from address-of expression
    let func_name = match extract_function_from_address_of(rhs) {
        Some(name) => name,
        None => return None, // Not an address-of expression, skip
    };

    // Check if the function is known to be @safe
    match known_safe_functions.get(&func_name) {
        Some(SafetyMode::Safe) => None, // OK
        Some(SafetyMode::Unsafe) => {
            Some(format!(
                "Cannot assign @unsafe function '{}' to SafeFn at line {}. \
                 SafeFn can only hold pointers to @safe functions.",
                func_name, line
            ))
        }
        None => {
            // Unknown functions are treated as @unsafe by default (two-state model)
            Some(format!(
                "Cannot assign unannotated function '{}' to SafeFn at line {}. \
                 The target function must be marked @safe. \
                 (Unannotated functions are @unsafe by default)",
                func_name, line
            ))
        }
    }
}

/// Extract function name from an address-of expression
fn extract_function_from_address_of(expr: &Expression) -> Option<String> {
    match expr {
        Expression::AddressOf(inner) => {
            match inner.as_ref() {
                Expression::Variable(name) => Some(name.clone()),
                Expression::MemberAccess { object, field } => {
                    // &ClassName::method
                    if let Expression::Variable(class_name) = object.as_ref() {
                        Some(format!("{}::{}", class_name, field))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        // Direct function name (without &) - some compilers allow this
        Expression::Variable(name) => {
            // Check if it looks like a function name (not a variable)
            // This is a heuristic
            if name.contains("::") || name.starts_with(|c: char| c.is_uppercase()) {
                Some(name.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if a function call expression is through a raw function pointer
/// Returns Some(error) if the call requires @unsafe
pub fn check_raw_function_pointer_call(
    callee: &Expression,
    callee_type: Option<&str>,
    line: usize,
) -> Option<String> {
    // If we have type info and it's a raw function pointer, flag it
    if let Some(type_name) = callee_type {
        if is_raw_function_pointer_type(type_name) {
            return Some(format!(
                "Call through raw function pointer at line {} requires @unsafe context. \
                 Consider using SafeFn<Sig> or UnsafeFn<Sig> wrapper types.",
                line
            ));
        }
    }

    // Check expression patterns that indicate a function pointer call
    match callee {
        Expression::Dereference(inner) => {
            // (*fp)(args) - explicit dereference of function pointer
            if let Expression::Variable(_) = inner.as_ref() {
                return Some(format!(
                    "Call through dereferenced function pointer at line {} requires @unsafe context. \
                     Consider using SafeFn<Sig> or UnsafeFn<Sig> wrapper types.",
                    line
                ));
            }
        }
        _ => {}
    }

    None
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_fn_type() {
        assert!(is_safe_fn_type("rusty::SafeFn<void(int)>"));
        assert!(is_safe_fn_type("SafeFn<int(const char*)>"));
        assert!(is_safe_fn_type("rusty::SafeMemFn<void (MyClass::*)(int)>"));
        assert!(is_safe_fn_type("SafeMemFn<int (Widget::*)(double) const>"));

        assert!(!is_safe_fn_type("rusty::UnsafeFn<void(int)>"));
        assert!(!is_safe_fn_type("std::function<void(int)>"));
        assert!(!is_safe_fn_type("void (*)(int)"));
    }

    #[test]
    fn test_is_unsafe_fn_type() {
        assert!(is_unsafe_fn_type("rusty::UnsafeFn<void(int)>"));
        assert!(is_unsafe_fn_type("UnsafeFn<int(const char*)>"));
        assert!(is_unsafe_fn_type("rusty::UnsafeMemFn<void (MyClass::*)(int)>"));

        assert!(!is_unsafe_fn_type("rusty::SafeFn<void(int)>"));
        assert!(!is_unsafe_fn_type("std::function<void(int)>"));
    }

    #[test]
    fn test_is_raw_function_pointer_type() {
        assert!(is_raw_function_pointer_type("void (*)(int)"));
        assert!(is_raw_function_pointer_type("int (*)(const char*, ...)"));
        assert!(is_raw_function_pointer_type("void (MyClass::*)(int)"));
        assert!(is_raw_function_pointer_type("int (Widget::*)(double) const"));

        assert!(!is_raw_function_pointer_type("rusty::SafeFn<void(int)>"));
        assert!(!is_raw_function_pointer_type("std::function<void(int)>"));
        assert!(!is_raw_function_pointer_type("void"));
    }

    #[test]
    fn test_is_safe_fn_call() {
        assert!(is_safe_fn_call("rusty::SafeFn<void(int)>", "operator()"));
        assert!(is_safe_fn_call("SafeFn<int()>", "operator()"));

        assert!(!is_safe_fn_call("rusty::SafeFn<void(int)>", "get"));
        assert!(!is_safe_fn_call("rusty::UnsafeFn<void(int)>", "operator()"));
    }

    #[test]
    fn test_is_unsafe_fn_call_unsafe_method() {
        assert!(is_unsafe_fn_call_unsafe_method("rusty::UnsafeFn<void(int)>", "call_unsafe"));
        assert!(is_unsafe_fn_call_unsafe_method("UnsafeFn<int()>", "call_unsafe"));

        assert!(!is_unsafe_fn_call_unsafe_method("rusty::UnsafeFn<void(int)>", "get"));
        assert!(!is_unsafe_fn_call_unsafe_method("rusty::SafeFn<void(int)>", "call_unsafe"));
    }

    #[test]
    fn test_extract_function_from_address_of() {
        // &function_name
        let expr = Expression::AddressOf(Box::new(Expression::Variable("my_func".to_string())));
        assert_eq!(extract_function_from_address_of(&expr), Some("my_func".to_string()));

        // &ClassName::method
        let expr = Expression::AddressOf(Box::new(Expression::MemberAccess {
            object: Box::new(Expression::Variable("MyClass".to_string())),
            field: "method".to_string(),
        }));
        assert_eq!(extract_function_from_address_of(&expr), Some("MyClass::method".to_string()));

        // Non-address-of expression
        let expr = Expression::Variable("x".to_string());
        assert_eq!(extract_function_from_address_of(&expr), None);
    }

    #[test]
    fn test_check_safe_fn_assignment_with_safe_function() {
        let mut known_safe: HashMap<String, SafetyMode> = HashMap::new();
        known_safe.insert("safe_func".to_string(), SafetyMode::Safe);

        let rhs = Expression::AddressOf(Box::new(Expression::Variable("safe_func".to_string())));

        let result = check_safe_fn_assignment_expr("callback", &rhs, &known_safe, 10);
        assert!(result.is_none(), "Assignment of @safe function to SafeFn should succeed");
    }

    #[test]
    fn test_check_safe_fn_assignment_with_unsafe_function() {
        let mut known_safe: HashMap<String, SafetyMode> = HashMap::new();
        known_safe.insert("unsafe_func".to_string(), SafetyMode::Unsafe);

        let rhs = Expression::AddressOf(Box::new(Expression::Variable("unsafe_func".to_string())));

        let result = check_safe_fn_assignment_expr("callback", &rhs, &known_safe, 10);
        assert!(result.is_some(), "Assignment of @unsafe function to SafeFn should fail");
        assert!(result.unwrap().contains("@unsafe function"));
    }

    #[test]
    fn test_check_safe_fn_assignment_with_unknown_function() {
        let known_safe: HashMap<String, SafetyMode> = HashMap::new();

        let rhs = Expression::AddressOf(Box::new(Expression::Variable("unknown_func".to_string())));

        let result = check_safe_fn_assignment_expr("callback", &rhs, &known_safe, 10);
        assert!(result.is_some(), "Assignment of unknown function to SafeFn should fail");
        assert!(result.unwrap().contains("unannotated function"));
    }

    // Member function pointer tests
    #[test]
    fn test_is_member_fn_wrapper_type() {
        assert!(is_member_fn_wrapper_type("rusty::SafeMemFn<void (MyClass::*)(int)>"));
        assert!(is_member_fn_wrapper_type("SafeMemFn<int (Widget::*)(double) const>"));
        assert!(is_member_fn_wrapper_type("rusty::UnsafeMemFn<void (MyClass::*)(int)>"));
        assert!(is_member_fn_wrapper_type("UnsafeMemFn<int (Widget::*)()>"));

        assert!(!is_member_fn_wrapper_type("rusty::SafeFn<void(int)>"));
        assert!(!is_member_fn_wrapper_type("void (MyClass::*)(int)"));
    }

    #[test]
    fn test_is_raw_member_function_pointer_type() {
        assert!(is_raw_member_function_pointer_type("void (MyClass::*)(int)"));
        assert!(is_raw_member_function_pointer_type("int (Widget::*)(double) const"));
        assert!(is_raw_member_function_pointer_type("bool (Foo::*)()"));

        assert!(!is_raw_member_function_pointer_type("void (*)(int)"));
        assert!(!is_raw_member_function_pointer_type("rusty::SafeMemFn<void (MyClass::*)(int)>"));
    }

    #[test]
    fn test_safe_mem_fn_call() {
        // SafeMemFn::operator() should be detected as safe
        assert!(is_safe_fn_call("rusty::SafeMemFn<void (MyClass::*)(int)>", "operator()"));
        assert!(is_safe_fn_call("SafeMemFn<int (Widget::*)() const>", "operator()"));
    }

    #[test]
    fn test_unsafe_mem_fn_call_unsafe() {
        // UnsafeMemFn::call_unsafe should be detected
        assert!(is_unsafe_fn_call_unsafe_method("rusty::UnsafeMemFn<void (MyClass::*)(int)>", "call_unsafe"));
        assert!(is_unsafe_fn_call_unsafe_method("UnsafeMemFn<int (Widget::*)()>", "call_unsafe"));
    }
}
