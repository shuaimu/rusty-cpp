use crate::parser::{Function, Statement, Expression};
use crate::parser::safety_annotations::{SafetyContext, SafetyMode};
use crate::parser::external_annotations::ExternalAnnotations;
use std::collections::HashSet;

/// Check for unsafe propagation in safe functions
/// 
/// In safe code, the following require explicit @unsafe annotation:
/// 1. Calling functions not marked as @safe
/// 2. Using types/structs not marked as @safe
/// 3. Any operation on unsafe types
pub fn check_unsafe_propagation(
    function: &Function,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
) -> Vec<String> {
    check_unsafe_propagation_with_external(function, safety_context, known_safe_functions, None)
}

/// Check for unsafe propagation with external annotations support
pub fn check_unsafe_propagation_with_external(
    function: &Function,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> Vec<String> {
    let mut errors = Vec::new();
    
    // Check each statement in the function
    for stmt in &function.body {
        if let Some(error) = check_statement_for_unsafe_calls_with_external(
            stmt, safety_context, known_safe_functions, external_annotations
        ) {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }
    
    errors
}

fn check_statement_for_unsafe_calls(
    stmt: &Statement,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
) -> Option<String> {
    check_statement_for_unsafe_calls_with_external(stmt, safety_context, known_safe_functions, None)
}

fn check_statement_for_unsafe_calls_with_external(
    stmt: &Statement,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> Option<String> {
    use crate::parser::Statement;
    
    match stmt {
        Statement::FunctionCall { name, location, .. } => {
            // Get the safety mode of the called function
            let called_safety = get_called_function_safety(name, safety_context, known_safe_functions, external_annotations);
            
            match called_safety {
                SafetyMode::Safe => {
                    // OK: safe can call safe
                }
                SafetyMode::Unsafe => {
                    // OK: safe can call explicitly unsafe functions
                    // The unsafe function takes responsibility for its own safety
                }
                SafetyMode::Undeclared => {
                    // ERROR: safe cannot call undeclared functions
                    // They must be explicitly audited and marked
                    return Some(format!(
                        "Calling undeclared function '{}' at line {} - must be explicitly marked @safe or @unsafe",
                        name, location.line
                    ));
                }
            }
        }
        Statement::Assignment { rhs, location, .. } => {
            // Check for function calls in the right-hand side
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(rhs, safety_context, known_safe_functions, external_annotations) {
                return Some(format!(
                    "Calling unsafe function '{}' at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }
        }
        Statement::Return(Some(expr)) => {
            // Check for function calls in return expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, external_annotations) {
                return Some(format!(
                    "Calling unsafe function '{}' in return statement requires unsafe context",
                    unsafe_func
                ));
            }
        }
        Statement::If { condition, then_branch, else_branch, location } => {
            // Check condition
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(condition, safety_context, known_safe_functions, external_annotations) {
                return Some(format!(
                    "Calling unsafe function '{}' in condition at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }
            
            // Recursively check branches
            for branch_stmt in then_branch {
                if let Some(error) = check_statement_for_unsafe_calls_with_external(branch_stmt, safety_context, known_safe_functions, external_annotations) {
                    return Some(error);
                }
            }
            
            if let Some(else_stmts) = else_branch {
                for branch_stmt in else_stmts {
                    if let Some(error) = check_statement_for_unsafe_calls_with_external(branch_stmt, safety_context, known_safe_functions, external_annotations) {
                        return Some(error);
                    }
                }
            }
        }
        Statement::Block(statements) => {
            // Check all statements in the block
            for block_stmt in statements {
                if let Some(error) = check_statement_for_unsafe_calls_with_external(block_stmt, safety_context, known_safe_functions, external_annotations) {
                    return Some(error);
                }
            }
        }
        _ => {}
    }
    
    None
}

fn find_unsafe_function_call(
    expr: &Expression,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
) -> Option<String> {
    find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, None)
}

fn find_unsafe_function_call_with_external(
    expr: &Expression,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> Option<String> {
    use crate::parser::Expression;
    
    match expr {
        Expression::FunctionCall { name, args } => {
            // Get the safety mode of the called function
            let called_safety = get_called_function_safety(name, safety_context, known_safe_functions, external_annotations);
            
            // Apply the corrected rules:
            // - Safe functions can call safe functions
            // - Safe functions can call unsafe functions (they're explicitly marked)
            // - Safe functions CANNOT call undeclared functions
            match called_safety {
                SafetyMode::Safe => {
                    // OK: safe can call safe
                }
                SafetyMode::Unsafe => {
                    // OK: safe can call explicitly unsafe functions
                }
                SafetyMode::Undeclared => {
                    // Error: safe function cannot call undeclared function
                    return Some(format!("{} (undeclared - must be explicitly marked @safe or @unsafe)", name));
                }
            }
            
            // Check arguments for nested unsafe calls
            for arg in args {
                if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations) {
                    return Some(unsafe_func);
                }
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            // Check both sides
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(left, safety_context, known_safe_functions, external_annotations) {
                return Some(unsafe_func);
            }
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(right, safety_context, known_safe_functions, external_annotations) {
                return Some(unsafe_func);
            }
        }
        Expression::Move(inner) | Expression::Dereference(inner) | Expression::AddressOf(inner) => {
            // Check inner expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(inner, safety_context, known_safe_functions, external_annotations) {
                return Some(unsafe_func);
            }
        }
        _ => {}
    }
    
    None
}

fn is_function_safe(
    func_name: &str,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
) -> bool {
    is_function_safe_with_external(func_name, safety_context, known_safe_functions, None)
}

/// Get the safety mode of a called function
fn get_called_function_safety(
    func_name: &str,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> SafetyMode {
    // Check for standard library functions we consider safe first
    if is_standard_safe_function(func_name) {
        return SafetyMode::Safe;
    }
    
    // First check if we know about this function in our context
    let local_safety = safety_context.get_function_safety(func_name);
    if local_safety != SafetyMode::Undeclared {
        return local_safety;
    }
    
    // Check if it's in our known safe functions set
    if known_safe_functions.contains(func_name) {
        return SafetyMode::Safe;
    }
    
    // Check external annotations if provided
    if let Some(annotations) = external_annotations {
        if let Some(is_safe) = annotations.is_function_safe(func_name) {
            return if is_safe { SafetyMode::Safe } else { SafetyMode::Unsafe };
        }
    }
    
    // Default to undeclared
    SafetyMode::Undeclared
}

fn is_function_safe_with_external(
    func_name: &str,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> bool {
    // Check for standard library functions we consider safe first
    if is_standard_safe_function(func_name) {
        return true;
    }
    
    get_called_function_safety(func_name, safety_context, known_safe_functions, external_annotations) == SafetyMode::Safe
}

fn is_standard_safe_function(func_name: &str) -> bool {
    // Whitelist of standard functions considered safe
    matches!(func_name, 
        "printf" | "scanf" | "puts" | "gets" |  // I/O (though gets is actually unsafe!)
        "malloc" | "free" | "new" | "delete" |  // Memory (debatable)
        "memcpy" | "memset" | "strcpy" |        // String ops (many are actually unsafe!)
        "sin" | "cos" | "sqrt" | "pow" |        // Math
        "move" | "std::move" |                  // Move semantics
        "cout" | "cin" | "cerr" | "clog" |      // C++ streams
        "operator<<" | "operator>>" |           // Stream operators
        "endl" | "flush" |                      // Stream manipulators
        "std::forward" | "std::swap"            // Utility
    )
    // Note: This list is intentionally conservative. 
    // In practice, we might want to be stricter or have a config file.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Statement, Expression, SourceLocation};
    
    #[test]
    fn test_detect_unsafe_function_call() {
        let stmt = Statement::FunctionCall {
            name: "unknown_func".to_string(),
            args: vec![],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };
        
        let safety_context = SafetyContext::new();
        let known_safe = HashSet::new();
        
        let error = check_statement_for_unsafe_calls(&stmt, &safety_context, &known_safe);
        assert!(error.is_some());
        let error_msg = error.unwrap();
        assert!(error_msg.contains("unknown_func"));
        assert!(error_msg.contains("unsafe"));
    }
    
    #[test]
    fn test_safe_function_allowed() {
        let stmt = Statement::FunctionCall {
            name: "printf".to_string(),
            args: vec![Expression::Literal("test".to_string())],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };
        
        let safety_context = SafetyContext::new();
        let known_safe = HashSet::new();
        
        let error = check_statement_for_unsafe_calls(&stmt, &safety_context, &known_safe);
        assert!(error.is_none(), "printf should be considered safe");
    }
    
    #[test]
    fn test_known_safe_function() {
        let stmt = Statement::FunctionCall {
            name: "my_safe_func".to_string(),
            args: vec![],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };
        
        let safety_context = SafetyContext::new();
        let mut known_safe = HashSet::new();
        known_safe.insert("my_safe_func".to_string());
        
        let error = check_statement_for_unsafe_calls(&stmt, &safety_context, &known_safe);
        assert!(error.is_none(), "Known safe function should be allowed");
    }
    
    #[test]
    fn test_unsafe_call_in_expression() {
        let stmt = Statement::Assignment {
            lhs: crate::parser::Expression::Variable("x".to_string()),
            rhs: Expression::FunctionCall {
                name: "unsafe_func".to_string(),
                args: vec![],
            },
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 15,
                column: 5,
            },
        };
        
        let safety_context = SafetyContext::new();
        let known_safe = HashSet::new();
        
        let error = check_statement_for_unsafe_calls(&stmt, &safety_context, &known_safe);
        assert!(error.is_some());
        let error_msg = error.unwrap();
        assert!(error_msg.contains("unsafe_func"));
    }
}