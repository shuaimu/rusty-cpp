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
            stmt, safety_context, known_safe_functions, external_annotations, &function.template_parameters
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
    check_statement_for_unsafe_calls_with_external(stmt, safety_context, known_safe_functions, None, &[])
}

/// Check if a name looks like a template type parameter (including variadic pack parameters)
/// This includes:
/// - Exact matches: "T", "Args"
/// - Pack patterns: "Args...", "Rest..."
/// - Element types: "T&&", "Args&&" (forwarding references in packs)
/// - Generic names: short uppercase-starting names that look like template params
fn is_template_parameter_like(name: &str, template_params: &[String]) -> bool {
    // Exact match
    if template_params.contains(&name.to_string()) {
        return true;
    }

    // Phase 1: Recognize pack-related patterns
    // Pattern 1: Name ends with "..." (pack expansion)
    if name.ends_with("...") {
        let base_name = name.trim_end_matches("...").trim();
        if template_params.contains(&base_name.to_string()) {
            return true;
        }
    }

    // Pattern 2: Name with && (forwarding reference, common in pack element types)
    // e.g., "Args&&" where "Args" is a template parameter
    if name.ends_with("&&") || name.ends_with("&") {
        let base_name = name.trim_end_matches('&').trim();
        if template_params.contains(&base_name.to_string()) {
            return true;
        }
    }

    // Pattern 3: Generic template-like names (short, uppercase start)
    // This catches variations that the parser might produce
    if name.len() <= 8 && name.len() > 0 {
        if let Some(first_char) = name.chars().next() {
            if first_char.is_uppercase() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                // Looks like a template parameter name
                return true;
            }
        }
    }

    false
}

fn check_statement_for_unsafe_calls_with_external(
    stmt: &Statement,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
    template_params: &[String],
) -> Option<String> {
    use crate::parser::Statement;

    match stmt {
        Statement::FunctionCall { name, location, .. } => {
            // Check if this is a template type parameter (not a real function call)
            // Phase 1: Enhanced check for variadic pack parameters
            if is_template_parameter_like(name, template_params) {
                return None; // Template type parameters are safe to use
            }

            // Special case: "unknown" function calls in template context are likely template type constructors
            // e.g., T(), T(x), etc. where the parser couldn't determine the name
            if !template_params.is_empty() && name == "unknown" {
                return None; // Allow unknown function calls in template context
            }

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
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(rhs, safety_context, known_safe_functions, external_annotations, template_params) {
                return Some(format!(
                    "Calling unsafe function '{}' at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }
        }
        Statement::Return(Some(expr)) => {
            // Check for function calls in return expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, external_annotations, template_params) {
                return Some(format!(
                    "Calling unsafe function '{}' in return statement requires unsafe context",
                    unsafe_func
                ));
            }
        }
        Statement::If { condition, then_branch, else_branch, location } => {
            // Check condition
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(condition, safety_context, known_safe_functions, external_annotations, template_params) {
                return Some(format!(
                    "Calling unsafe function '{}' in condition at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }

            // Recursively check branches
            for branch_stmt in then_branch {
                if let Some(error) = check_statement_for_unsafe_calls_with_external(branch_stmt, safety_context, known_safe_functions, external_annotations, template_params) {
                    return Some(error);
                }
            }

            if let Some(else_stmts) = else_branch {
                for branch_stmt in else_stmts {
                    if let Some(error) = check_statement_for_unsafe_calls_with_external(branch_stmt, safety_context, known_safe_functions, external_annotations, template_params) {
                        return Some(error);
                    }
                }
            }
        }
        Statement::Block(statements) => {
            // Check all statements in the block
            for block_stmt in statements {
                if let Some(error) = check_statement_for_unsafe_calls_with_external(block_stmt, safety_context, known_safe_functions, external_annotations, template_params) {
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
    find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, None, &[])
}

fn find_unsafe_function_call_with_external(
    expr: &Expression,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
    template_params: &[String],
) -> Option<String> {
    use crate::parser::Expression;

    match expr {
        Expression::FunctionCall { name, args } => {
            // Check if this is a template type parameter (not a real function call)
            // Phase 1: Enhanced check for variadic pack parameters
            if is_template_parameter_like(name, template_params) {
                // Template type parameters are safe to use (e.g., T x = ...)
                // Just check the arguments
                for arg in args {
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params) {
                        return Some(unsafe_func);
                    }
                }
                return None;
            }

            // Special case: "unknown" function calls in template context are likely template type constructors
            // e.g., T(), T(x), etc. where the parser couldn't determine the name
            if !template_params.is_empty() && name == "unknown" {
                // Just check the arguments
                for arg in args {
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params) {
                        return Some(unsafe_func);
                    }
                }
                return None; // Allow unknown function calls in template context
            }

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
                if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params) {
                    return Some(unsafe_func);
                }
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            // Check both sides
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(left, safety_context, known_safe_functions, external_annotations, template_params) {
                return Some(unsafe_func);
            }
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(right, safety_context, known_safe_functions, external_annotations, template_params) {
                return Some(unsafe_func);
            }
        }
        Expression::Move(inner) | Expression::Dereference(inner) | Expression::AddressOf(inner) => {
            // Check inner expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(inner, safety_context, known_safe_functions, external_annotations, template_params) {
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

/// Strip std:: prefix from function name for matching
fn strip_std_prefix(func_name: &str) -> &str {
    func_name.strip_prefix("std::").unwrap_or(func_name)
}

/// Check if function is a safe C++ stream operation
fn is_safe_stream_function(name: &str) -> bool {
    matches!(name,
        "cout" | "cin" | "cerr" | "clog" |
        "endl" | "flush" | "getline"
    )
}

/// Check if function is a safe algorithm
fn is_safe_algorithm(name: &str) -> bool {
    matches!(name,
        // Searching
        "find" | "find_if" | "find_if_not" |
        "count" | "count_if" |
        "all_of" | "any_of" | "none_of" |
        "for_each" |
        // Modifying
        "copy" | "copy_if" | "copy_n" |
        "fill" | "fill_n" |
        "transform" | "generate" |
        "remove" | "remove_if" |  // Note: these are NOT moves!
        "replace" | "replace_if" |
        "reverse" | "rotate" | "unique" |
        // Sorting
        "sort" | "stable_sort" | "partial_sort" |
        "is_sorted" | "nth_element" |
        // Binary search
        "binary_search" | "lower_bound" | "upper_bound" | "equal_range" |
        // Min/max
        "min" | "max" | "minmax" |
        "min_element" | "max_element" |
        // Numeric
        "accumulate" | "inner_product" | "adjacent_difference" | "partial_sum"
    )
}

/// Check if function is a safe container method
fn is_safe_container_method(name: &str) -> bool {
    matches!(name,
        "push_back" | "pop_back" | "emplace_back" |
        "push_front" | "pop_front" | "emplace_front" |
        "insert" | "emplace" | "erase" | "clear" |
        "size" | "empty" | "capacity" | "reserve" | "resize" |
        "at" | "front" | "back" | "data" |
        "begin" | "end" | "rbegin" | "rend" |
        "cbegin" | "cend" | "crbegin" | "crend" |
        // Map/set specific
        "count" | "contains"
    )
}

/// Check if function is a safe operator
fn is_safe_operator(name: &str) -> bool {
    matches!(name,
        "operator+" | "operator-" | "operator*" | "operator/" | "operator%" |
        "operator++" | "operator--" |
        "operator==" | "operator!=" | "operator<" | "operator>" | "operator<=" | "operator>=" |
        "operator[]" | "operator()" |
        "operator=" | "operator+=" | "operator-=" | "operator*=" | "operator/=" |
        "operator<<" | "operator>>" |
        "operator!" | "operator&&" | "operator||" |
        "operator&" | "operator|" | "operator^" | "operator~" |
        "operator," | "operator->*" | "operator.*"
    )
}

fn is_standard_safe_function(func_name: &str) -> bool {
    // Strip std:: prefix for more general matching
    let stripped = strip_std_prefix(func_name);

    // Check operators first (they don't have std:: prefix)
    if is_safe_operator(func_name) {
        return true;
    }

    // Check categorized functions
    if is_safe_stream_function(stripped) ||
       is_safe_algorithm(stripped) ||
       is_safe_container_method(stripped) {
        return true;
    }

    // Remaining specific functions (not categorized above)
    matches!(stripped,
        // C I/O
        "printf" | "scanf" | "puts" | "gets" |
        "malloc" | "free" | "new" | "delete" |
        "memcpy" | "memset" | "strcpy" |

        // Math functions
        "sin" | "cos" | "sqrt" | "pow" | "abs" | "floor" | "ceil" | "round" |

        // C++ utility functions
        "move" | "forward" | "swap" | "exchange" |

        // Smart pointers (only operations that don't expose raw pointers)
        "make_unique" | "make_shared" |
        "reset" |  // Replaces pointer, safe if given smart pointer
        "use_count" | "unique" |  // Query operations, return integers
        // NOTE: get() and release() return raw pointers → UNSAFE

        // Type Utilities (only truly safe ones)
        "as_const" | "to_underlying" |

        // String methods
        "length" | "c_str" | "substr" | "append" |
        "compare" | "rfind" | "find_first_of" | "find_last_of" |

        // Utility
        "make_pair" | "make_tuple" | "get" |

        // Optional/variant (C++17)
        "make_optional" | "value" | "value_or" | "has_value" |
        "holds_alternative" | "visit" |

        // String conversion
        "to_string" | "stoi" | "stol" | "stod"
    )
    // Note: This whitelist allows common std:: functions to be used in @safe code
    // without requiring explicit @unsafe blocks. The strip_std_prefix() function
    // handles matching both "func" and "std::func" automatically.
    // Functions are included only if their safety can be verified by the borrow checker.
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