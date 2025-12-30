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
    let mut unsafe_depth = 0;

    // Collect callable parameters - parameters whose type is or contains a template type parameter
    // e.g., for template<typename F> void foo(F&& write_fn), "write_fn" is a callable parameter
    let callable_params = get_callable_parameters(&function.parameters, &function.template_parameters);

    // Check each statement in the function
    for stmt in &function.body {
        // Track unsafe scope depth
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

        // Skip checking if we're in an unsafe block
        let in_unsafe_scope = unsafe_depth > 0;

        if let Some(error) = check_statement_for_unsafe_calls_with_external(
            stmt, safety_context, known_safe_functions, external_annotations,
            &function.template_parameters, &callable_params, in_unsafe_scope
        ) {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }

    errors
}

/// Get list of parameter names that are callable (their type is/contains a template parameter)
/// For example: template<typename F> void foo(F&& write_fn) -> returns ["write_fn"]
fn get_callable_parameters(parameters: &[crate::parser::Variable], template_params: &[String]) -> HashSet<String> {
    let mut callable_params = HashSet::new();

    for param in parameters {
        // Check if the parameter's type contains any template type parameter
        // This handles: F, F&&, F&, const F&, std::function<...> where ... contains F, etc.
        let type_name = &param.type_name;

        for template_param in template_params {
            // Check if the type contains the template parameter
            // Handle cases like: F, F&&, F&, const F&, F *, etc.
            if type_contains_template_param(type_name, template_param) {
                callable_params.insert(param.name.clone());
                break;
            }
        }
    }

    callable_params
}

/// Check if a type name contains a template parameter
/// Handles: F, F&&, F&, const F&, F const&, etc.
fn type_contains_template_param(type_name: &str, template_param: &str) -> bool {
    // Simple word boundary check - the template param should appear as a whole word
    // not as part of another identifier
    let type_clean = type_name.replace("const", "").replace("&&", "").replace("&", "")
                              .replace("*", "").replace(" ", "");

    // Check for exact match or template param at word boundary
    if type_clean == template_param {
        return true;
    }

    // Check if template param appears as a word in the type
    // e.g., "F" in "F&&" or "F &" or "const F&"
    let words: Vec<&str> = type_name.split(|c: char| !c.is_alphanumeric() && c != '_')
                                     .filter(|s| !s.is_empty())
                                     .collect();
    words.contains(&template_param.as_ref())
}

fn check_statement_for_unsafe_calls(
    stmt: &Statement,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
) -> Option<String> {
    check_statement_for_unsafe_calls_with_external(stmt, safety_context, known_safe_functions, None, &[], &HashSet::new(), false)
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

/// Process a list of statements while tracking unsafe depth, returning all errors found
fn check_statements_with_unsafe_tracking(
    statements: &[Statement],
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
    template_params: &[String],
    callable_params: &HashSet<String>,
    initial_unsafe_depth: usize,
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unsafe_depth = initial_unsafe_depth;

    for stmt in statements {
        // Track unsafe scope depth
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

        if let Some(error) = check_statement_for_unsafe_calls_with_external(
            stmt, safety_context, known_safe_functions, external_annotations,
            template_params, callable_params, in_unsafe_scope
        ) {
            errors.push(error);
        }
    }

    errors
}

fn check_statement_for_unsafe_calls_with_external(
    stmt: &Statement,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
    template_params: &[String],
    callable_params: &HashSet<String>,
    in_unsafe_scope: bool,
) -> Option<String> {
    use crate::parser::Statement;

    // Skip all checks if we're in an unsafe block
    if in_unsafe_scope {
        return None;
    }

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

            // Special case: Lambda operator() calls
            // Lambdas defined in @safe context have already been checked for safety
            // Their operator() is safe to call
            if name == "operator()" || name.contains("operator()") {
                return None; // Lambda calls are safe - their body was already checked
            }

            // Special case: Callable template parameters
            // e.g., template<typename F> void foo(F&& write_fn) { write_fn(42); }
            // Calling write_fn is safe because it's a callable passed by the caller
            // Note: In class methods, the name might be prefixed with class name (e.g., "Class::handler")
            if callable_params.contains(name) {
                return None; // Callable parameters are safe to invoke
            }
            // Also check for class-prefixed version (e.g., "Class::handler" -> check "handler")
            if let Some(simple_name) = name.rsplit("::").next() {
                if callable_params.contains(simple_name) {
                    return None; // Callable parameters are safe to invoke
                }
            }

            // Get the safety mode of the called function
            let called_safety = get_called_function_safety(name, safety_context, known_safe_functions, external_annotations);

            match called_safety {
                SafetyMode::Safe => {
                    // OK: safe can call safe
                }
                SafetyMode::Unsafe => {
                    // ERROR: safe cannot call unsafe/unannotated functions directly
                    // Must wrap in @unsafe { } block
                    return Some(format!(
                        "Calling non-safe function '{}' at line {} requires @unsafe {{ }} block",
                        name, location.line
                    ));
                }
            }
        }
        Statement::Assignment { rhs, location, .. } => {
            // Check for function calls in the right-hand side
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(rhs, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                return Some(format!(
                    "Calling unsafe function '{}' at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }
        }
        Statement::Return(Some(expr)) => {
            // Check for function calls in return expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                return Some(format!(
                    "Calling unsafe function '{}' in return statement requires unsafe context",
                    unsafe_func
                ));
            }
        }
        Statement::If { condition, then_branch, else_branch, location } => {
            // Check condition
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(condition, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                return Some(format!(
                    "Calling unsafe function '{}' in condition at line {} requires unsafe context",
                    unsafe_func, location.line
                ));
            }

            // Recursively check branches with proper unsafe depth tracking
            // Start with unsafe_depth=0 since in_unsafe_scope=false here (we return early if true)
            let then_errors = check_statements_with_unsafe_tracking(
                then_branch, safety_context, known_safe_functions, external_annotations,
                template_params, callable_params, 0
            );
            if !then_errors.is_empty() {
                return Some(then_errors.into_iter().next().unwrap());
            }

            if let Some(else_stmts) = else_branch {
                let else_errors = check_statements_with_unsafe_tracking(
                    else_stmts, safety_context, known_safe_functions, external_annotations,
                    template_params, callable_params, 0
                );
                if !else_errors.is_empty() {
                    return Some(else_errors.into_iter().next().unwrap());
                }
            }
        }
        Statement::Block(statements) => {
            // Check all statements in the block with proper unsafe depth tracking
            let block_errors = check_statements_with_unsafe_tracking(
                statements, safety_context, known_safe_functions, external_annotations,
                template_params, callable_params, 0
            );
            if !block_errors.is_empty() {
                return Some(block_errors.into_iter().next().unwrap());
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
    find_unsafe_function_call_with_external(expr, safety_context, known_safe_functions, None, &[], &HashSet::new())
}

fn find_unsafe_function_call_with_external(
    expr: &Expression,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
    template_params: &[String],
    callable_params: &HashSet<String>,
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
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
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
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                        return Some(unsafe_func);
                    }
                }
                return None; // Allow unknown function calls in template context
            }

            // Special case: Lambda operator() calls
            // Lambdas defined in @safe context have already been checked for safety
            if name == "operator()" || name.contains("operator()") {
                // Just check the arguments
                for arg in args {
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                        return Some(unsafe_func);
                    }
                }
                return None; // Lambda calls are safe - their body was already checked
            }

            // Special case: Callable template parameters
            // e.g., template<typename F> void foo(F&& write_fn) { write_fn(42); }
            // Note: In class methods, the name might be prefixed with class name (e.g., "Class::handler")
            let is_callable_param = callable_params.contains(name) ||
                name.rsplit("::").next().map(|s| callable_params.contains(s)).unwrap_or(false);
            if is_callable_param {
                // Just check the arguments
                for arg in args {
                    if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                        return Some(unsafe_func);
                    }
                }
                return None; // Callable parameters are safe to invoke
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
                    // Error: safe function cannot call unsafe function directly
                    return Some(format!("{} (non-safe - use @unsafe block)", name));
                }
            }

            // Check arguments for nested unsafe calls
            for arg in args {
                if let Some(unsafe_func) = find_unsafe_function_call_with_external(arg, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                    return Some(unsafe_func);
                }
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            // Check both sides
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(left, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                return Some(unsafe_func);
            }
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(right, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
                return Some(unsafe_func);
            }
        }
        Expression::Move { inner, .. } | Expression::Dereference(inner) | Expression::AddressOf(inner) => {
            // Check inner expression
            if let Some(unsafe_func) = find_unsafe_function_call_with_external(inner, safety_context, known_safe_functions, external_annotations, template_params, callable_params) {
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
    // First check if we know about this function in our context
    let local_safety = safety_context.get_function_safety(func_name);
    if local_safety != SafetyMode::Unsafe {
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

    // Default to unsafe - all unannotated functions are unsafe
    SafetyMode::Unsafe
}

fn is_function_safe_with_external(
    func_name: &str,
    safety_context: &SafetyContext,
    known_safe_functions: &HashSet<String>,
    external_annotations: Option<&ExternalAnnotations>,
) -> bool {
    get_called_function_safety(func_name, safety_context, known_safe_functions, external_annotations) == SafetyMode::Safe
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
    fn test_stl_functions_require_unsafe() {
        // With the new two-state model, ALL non-safe functions (including STL) require @unsafe blocks
        let stmt = Statement::FunctionCall {
            name: "std::move".to_string(),
            args: vec![Expression::Variable("x".to_string())],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let safety_context = SafetyContext::new();
        let known_safe = HashSet::new();

        let error = check_statement_for_unsafe_calls(&stmt, &safety_context, &known_safe);
        assert!(error.is_some(), "std::move should require @unsafe block in safe code");
        let error_msg = error.unwrap();
        assert!(error_msg.contains("std::move"));
        assert!(error_msg.contains("@unsafe"));
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