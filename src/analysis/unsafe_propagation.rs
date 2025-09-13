use crate::parser::{Function, Statement, Expression};
use crate::parser::safety_annotations::SafetyContext;
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
            // Check if the called function is safe
            if !is_function_safe_with_external(name, safety_context, known_safe_functions, external_annotations) {
                return Some(format!(
                    "Calling unsafe function '{}' at line {} requires unsafe context",
                    name, location.line
                ));
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
            // Check if this function is safe
            if !is_function_safe_with_external(name, safety_context, known_safe_functions, external_annotations) {
                return Some(name.clone());
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

fn is_function_safe_with_external(
    func_name: &str,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> bool {
    // Check if it's explicitly marked as safe
    // Only functions with explicit @safe annotation are considered safe
    if known_safe_functions.contains(func_name) {
        return true;
    }
    
    // Check external annotations if provided
    if let Some(annotations) = external_annotations {
        if let Some(is_safe) = annotations.is_function_safe(func_name) {
            return is_safe;
        }
    }
    
    // Check for standard library functions we consider safe
    if is_standard_safe_function(func_name) {
        return true;
    }
    
    // Check if it's explicitly marked as unsafe - still unsafe
    for (name, mode) in &safety_context.function_overrides {
        if name == func_name {
            use crate::parser::safety_annotations::SafetyMode;
            return *mode == SafetyMode::Safe;
        }
    }
    
    // If no explicit annotation, it's unsafe (even in safe namespace/file)
    false
}

fn is_standard_safe_function(func_name: &str) -> bool {
    // Whitelist of standard functions considered safe
    matches!(func_name, 
        "printf" | "scanf" | "puts" | "gets" |  // I/O (though gets is actually unsafe!)
        "malloc" | "free" | "new" | "delete" |  // Memory (debatable)
        "memcpy" | "memset" | "strcpy" |        // String ops (many are actually unsafe!)
        "sin" | "cos" | "sqrt" | "pow" |        // Math
        "move" | "std::move" |                  // Move semantics
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
            lhs: "x".to_string(),
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