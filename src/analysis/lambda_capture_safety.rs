/// Lambda capture safety checking for @safe code with escape analysis
///
/// In @safe code:
/// - Reference captures ([&], [&x]) are ALLOWED if the lambda doesn't escape
/// - Reference captures that ESCAPE are FORBIDDEN - can create dangling references
/// - Copy captures ([x], [=]) are ALWAYS ALLOWED - safe copy semantics
/// - Move captures ([x = std::move(y)]) are ALWAYS ALLOWED - ownership transfer is safe
/// - 'this' capture is FORBIDDEN - 'this' is a raw pointer that can dangle
///
/// Escape means:
/// - Lambda is returned from function
/// - Lambda is stored in a variable/container that outlives captured variables
/// - Lambda is passed to a function that takes ownership (stores it)

use crate::parser::{Function, Statement, Expression};
use crate::parser::ast_visitor::LambdaCaptureKind;
use crate::parser::safety_annotations::SafetyMode;
use crate::debug_println;
use std::collections::{HashMap, HashSet};

/// Context for tracking lambdas and their escape status
#[derive(Debug)]
struct LambdaContext {
    /// Map from variable name to captured references
    lambda_ref_captures: HashMap<String, Vec<String>>,
    /// Map from variable name to the capture kind (for error messages)
    lambda_has_default_ref: HashMap<String, bool>,
    /// Set of lambdas that have escaped
    escaped_lambdas: HashSet<String>,
    /// Current scope depth
    scope_depth: usize,
    /// Scope depth when each lambda was created
    lambda_scopes: HashMap<String, usize>,
    /// Variables and their scope depths
    variable_scopes: HashMap<String, usize>,
}

impl LambdaContext {
    fn new() -> Self {
        Self {
            lambda_ref_captures: HashMap::new(),
            lambda_has_default_ref: HashMap::new(),
            escaped_lambdas: HashSet::new(),
            scope_depth: 0,
            lambda_scopes: HashMap::new(),
            variable_scopes: HashMap::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn exit_scope(&mut self) {
        if self.scope_depth > 0 {
            self.scope_depth -= 1;
        }
    }

    fn register_variable(&mut self, name: &str) {
        self.variable_scopes.insert(name.to_string(), self.scope_depth);
    }

    fn register_lambda(&mut self, name: &str, ref_captures: Vec<String>, has_default_ref: bool) {
        self.lambda_ref_captures.insert(name.to_string(), ref_captures);
        self.lambda_has_default_ref.insert(name.to_string(), has_default_ref);
        self.lambda_scopes.insert(name.to_string(), self.scope_depth);
    }

    fn mark_escaped(&mut self, name: &str) {
        self.escaped_lambdas.insert(name.to_string());
    }

    fn get_escaped_lambdas_with_ref_captures(&self) -> Vec<(String, Vec<String>, bool)> {
        self.escaped_lambdas
            .iter()
            .filter_map(|name| {
                let captures = self.lambda_ref_captures.get(name)?;
                if captures.is_empty() && !self.lambda_has_default_ref.get(name).unwrap_or(&false) {
                    None
                } else {
                    Some((
                        name.clone(),
                        captures.clone(),
                        *self.lambda_has_default_ref.get(name).unwrap_or(&false),
                    ))
                }
            })
            .collect()
    }
}

/// Check a parsed function for lambda capture safety violations
pub fn check_lambda_capture_safety(
    function: &Function,
    function_safety: SafetyMode,
) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        debug_println!("DEBUG LAMBDA: Skipping function '{}' (not @safe)", function.name);
        return errors;
    }

    debug_println!("DEBUG LAMBDA: Checking function '{}' for lambda capture safety", function.name);

    // Track if we're inside an @unsafe block
    let mut unsafe_depth = 0;
    let mut lambda_context = LambdaContext::new();

    // First pass: collect all lambda definitions and track escapes
    collect_lambdas_and_escapes(&function.body, &function.name, &mut lambda_context, &mut unsafe_depth);

    // Check for 'this' captures (always forbidden)
    check_this_captures_errors(&function.body, &function.name, &mut errors, &mut 0);

    // Report errors for escaped lambdas with reference captures
    for (lambda_name, ref_captures, has_default_ref) in lambda_context.get_escaped_lambdas_with_ref_captures() {
        if has_default_ref {
            errors.push(format!(
                "Reference capture in @safe code: Lambda '{}' escapes but uses default reference capture [&] which can create dangling references - use copy capture [=] instead",
                lambda_name
            ));
        } else {
            for capture in ref_captures {
                errors.push(format!(
                    "Reference capture in @safe code: Lambda '{}' escapes but captures '{}' by reference ([&{}]) which can create dangling references - use copy capture [{}] instead",
                    lambda_name, capture, capture, capture
                ));
            }
        }
    }

    errors
}

fn collect_lambdas_and_escapes(
    statements: &[Statement],
    function_name: &str,
    ctx: &mut LambdaContext,
    unsafe_depth: &mut usize,
) {
    for stmt in statements {
        match stmt {
            Statement::EnterUnsafe => {
                *unsafe_depth += 1;
            }
            Statement::ExitUnsafe => {
                if *unsafe_depth > 0 {
                    *unsafe_depth -= 1;
                }
            }
            Statement::EnterScope => {
                ctx.enter_scope();
            }
            Statement::ExitScope => {
                ctx.exit_scope();
            }
            Statement::VariableDecl(var) => {
                ctx.register_variable(&var.name);
            }
            Statement::Assignment { lhs, rhs, .. } => {
                if *unsafe_depth == 0 {
                    // Check if RHS is a lambda
                    if let Some((ref_captures, has_default_ref)) = extract_lambda_captures(rhs) {
                        // Extract variable name from lhs expression
                        if let Some(lhs_name) = extract_variable_name(lhs) {
                            ctx.register_lambda(&lhs_name, ref_captures, has_default_ref);
                            debug_println!("DEBUG LAMBDA: Registered lambda '{}' in function '{}'", lhs_name, function_name);
                        }
                    }
                }
            }
            Statement::Return(Some(expr)) => {
                // Returning a lambda = escape
                if *unsafe_depth == 0 {
                    if let Some(var_name) = extract_variable_name(expr) {
                        ctx.mark_escaped(&var_name);
                        debug_println!("DEBUG LAMBDA: Lambda '{}' escapes via return in '{}'", var_name, function_name);
                    }
                    // Check if returning a lambda expression directly
                    if let Some((ref_captures, has_default_ref)) = extract_lambda_captures(expr) {
                        // Anonymous lambda being returned - this is an escape
                        let lambda_name = format!("_anon_lambda_{}", statements.len());
                        ctx.register_lambda(&lambda_name, ref_captures, has_default_ref);
                        ctx.mark_escaped(&lambda_name);
                        debug_println!("DEBUG LAMBDA: Anonymous lambda escapes via return in '{}'", function_name);
                    }
                }
            }
            Statement::ExpressionStatement { expr, .. } => {
                if *unsafe_depth == 0 {
                    // Check for function calls that might store the lambda
                    check_for_escape_via_call(expr, ctx, function_name);
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                collect_lambdas_and_escapes(then_branch, function_name, ctx, unsafe_depth);
                if let Some(else_stmts) = else_branch {
                    collect_lambdas_and_escapes(else_stmts, function_name, ctx, unsafe_depth);
                }
            }
            Statement::Block(inner_stmts) => {
                collect_lambdas_and_escapes(inner_stmts, function_name, ctx, unsafe_depth);
            }
            _ => {}
        }
    }
}

fn check_this_captures_errors(
    statements: &[Statement],
    function_name: &str,
    errors: &mut Vec<String>,
    unsafe_depth: &mut usize,
) {
    for stmt in statements {
        match stmt {
            Statement::EnterUnsafe => {
                *unsafe_depth += 1;
            }
            Statement::ExitUnsafe => {
                if *unsafe_depth > 0 {
                    *unsafe_depth -= 1;
                }
            }
            Statement::Assignment { rhs, location, .. } => {
                if *unsafe_depth == 0 {
                    check_expression_for_this_capture(rhs, function_name, location, errors);
                }
            }
            Statement::ExpressionStatement { expr, location } => {
                if *unsafe_depth == 0 {
                    check_expression_for_this_capture(expr, function_name, location, errors);
                }
            }
            Statement::Return(Some(expr)) => {
                if *unsafe_depth == 0 {
                    let default_location = crate::parser::ast_visitor::SourceLocation {
                        file: "unknown".to_string(),
                        line: 0,
                        column: 0,
                    };
                    check_expression_for_this_capture(expr, function_name, &default_location, errors);
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                check_this_captures_errors(then_branch, function_name, errors, unsafe_depth);
                if let Some(else_stmts) = else_branch {
                    check_this_captures_errors(else_stmts, function_name, errors, unsafe_depth);
                }
            }
            Statement::Block(inner_stmts) => {
                check_this_captures_errors(inner_stmts, function_name, errors, unsafe_depth);
            }
            _ => {}
        }
    }
}

fn check_expression_for_this_capture(
    expr: &Expression,
    _function_name: &str,
    location: &crate::parser::ast_visitor::SourceLocation,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::Lambda { captures } => {
            for capture in captures {
                if matches!(capture, LambdaCaptureKind::This) {
                    errors.push(format!(
                        "Reference capture in @safe code at {}:{}: Capturing 'this' is forbidden in @safe code - 'this' is a raw pointer that can dangle",
                        location.file, location.line
                    ));
                }
            }
        }
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expression_for_this_capture(arg, _function_name, location, errors);
            }
        }
        Expression::Move(inner) |
        Expression::Dereference(inner) |
        Expression::AddressOf(inner) => {
            check_expression_for_this_capture(inner, _function_name, location, errors);
        }
        Expression::BinaryOp { left, right, .. } => {
            check_expression_for_this_capture(left, _function_name, location, errors);
            check_expression_for_this_capture(right, _function_name, location, errors);
        }
        Expression::MemberAccess { object, .. } => {
            check_expression_for_this_capture(object, _function_name, location, errors);
        }
        _ => {}
    }
}

fn check_for_escape_via_call(
    expr: &Expression,
    ctx: &mut LambdaContext,
    function_name: &str,
) {
    if let Expression::FunctionCall { name, args, .. } = expr {
        // Check if passing a lambda to a function that stores it
        // For now, we consider push_back, emplace_back, insert, etc. as escaping
        let storing_methods = ["push_back", "emplace_back", "push_front", "emplace_front",
                              "insert", "emplace", "assign", "store"];

        let method_name = name.split("::").last().unwrap_or(name);
        let method_name = method_name.split('.').last().unwrap_or(method_name);

        if storing_methods.iter().any(|&m| method_name.contains(m)) {
            for arg in args {
                if let Some(var_name) = extract_variable_name(arg) {
                    ctx.mark_escaped(&var_name);
                    debug_println!("DEBUG LAMBDA: Lambda '{}' potentially escapes via {} in '{}'",
                        var_name, name, function_name);
                }
            }
        }

        // Recursively check nested calls
        for arg in args {
            check_for_escape_via_call(arg, ctx, function_name);
        }
    }
}

fn extract_lambda_captures(expr: &Expression) -> Option<(Vec<String>, bool)> {
    match expr {
        Expression::Lambda { captures } => {
            let mut ref_captures = Vec::new();
            let mut has_default_ref = false;

            for capture in captures {
                match capture {
                    LambdaCaptureKind::DefaultRef => {
                        has_default_ref = true;
                    }
                    LambdaCaptureKind::ByRef(var_name) => {
                        ref_captures.push(var_name.clone());
                    }
                    _ => {}
                }
            }

            Some((ref_captures, has_default_ref))
        }
        _ => None,
    }
}

fn extract_variable_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::Move(inner) => extract_variable_name(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_visitor::SourceLocation;

    fn make_location() -> SourceLocation {
        SourceLocation {
            file: "test.cpp".to_string(),
            line: 1,
            column: 1,
        }
    }

    #[test]
    fn test_this_capture_in_safe_is_error() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::This],
        };

        let mut errors = Vec::new();
        check_expression_for_this_capture(&lambda, "test", &make_location(), &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("this"));
    }

    #[test]
    fn test_copy_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::ByCopy("x".to_string())],
        };

        // Copy captures never cause errors
        let (ref_captures, has_default_ref) = extract_lambda_captures(&lambda).unwrap();
        assert!(ref_captures.is_empty());
        assert!(!has_default_ref);
    }

    #[test]
    fn test_default_copy_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::DefaultCopy],
        };

        let (ref_captures, has_default_ref) = extract_lambda_captures(&lambda).unwrap();
        assert!(ref_captures.is_empty());
        assert!(!has_default_ref);
    }

    #[test]
    fn test_init_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::Init {
                name: "y".to_string(),
                is_move: true,
            }],
        };

        let (ref_captures, has_default_ref) = extract_lambda_captures(&lambda).unwrap();
        assert!(ref_captures.is_empty());
        assert!(!has_default_ref);
    }

    #[test]
    fn test_ref_capture_extraction() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::ByRef("x".to_string())],
        };

        let (ref_captures, has_default_ref) = extract_lambda_captures(&lambda).unwrap();
        assert_eq!(ref_captures, vec!["x"]);
        assert!(!has_default_ref);
    }

    #[test]
    fn test_default_ref_capture_extraction() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::DefaultRef],
        };

        let (ref_captures, has_default_ref) = extract_lambda_captures(&lambda).unwrap();
        assert!(ref_captures.is_empty());
        assert!(has_default_ref);
    }

    #[test]
    fn test_lambda_context_escape_tracking() {
        let mut ctx = LambdaContext::new();

        // Register a lambda with ref captures
        ctx.register_lambda("lambda1", vec!["x".to_string()], false);

        // Not escaped yet
        assert!(ctx.get_escaped_lambdas_with_ref_captures().is_empty());

        // Mark as escaped
        ctx.mark_escaped("lambda1");

        // Now it should be reported
        let escaped = ctx.get_escaped_lambdas_with_ref_captures();
        assert_eq!(escaped.len(), 1);
        assert_eq!(escaped[0].0, "lambda1");
        assert_eq!(escaped[0].1, vec!["x"]);
    }

    #[test]
    fn test_lambda_context_no_escape_no_error() {
        let mut ctx = LambdaContext::new();

        // Register a lambda with ref captures but don't mark as escaped
        ctx.register_lambda("lambda1", vec!["x".to_string()], false);

        // Should not be reported
        let escaped = ctx.get_escaped_lambdas_with_ref_captures();
        assert!(escaped.is_empty());
    }

    #[test]
    fn test_lambda_context_escape_no_ref_capture() {
        let mut ctx = LambdaContext::new();

        // Register a lambda WITHOUT ref captures
        ctx.register_lambda("lambda1", vec![], false);
        ctx.mark_escaped("lambda1");

        // Should not be reported (no ref captures)
        let escaped = ctx.get_escaped_lambdas_with_ref_captures();
        assert!(escaped.is_empty());
    }
}
