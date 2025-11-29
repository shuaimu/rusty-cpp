/// Lambda capture safety checking for @safe code
///
/// In @safe code:
/// - Reference captures ([&], [&x]) are FORBIDDEN - can create dangling references
/// - Copy captures ([x], [=]) are ALLOWED - safe copy semantics
/// - Move captures ([x = std::move(y)]) are ALLOWED - ownership transfer is safe
/// - 'this' capture is FORBIDDEN - 'this' is a raw pointer

use crate::parser::{Function, Statement, Expression};
use crate::parser::ast_visitor::LambdaCaptureKind;
use crate::parser::safety_annotations::SafetyMode;
use crate::debug_println;

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

    // Check all statements in the function body
    check_statements(&function.body, &function.name, &mut errors, &mut unsafe_depth);

    errors
}

fn check_statements(
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
                // Check if RHS is a lambda with reference captures
                if *unsafe_depth == 0 {
                    check_expression_for_lambda(rhs, function_name, location, errors);
                }
            }
            Statement::ExpressionStatement { expr, location } => {
                // Check standalone expressions (e.g., lambda invocations)
                if *unsafe_depth == 0 {
                    check_expression_for_lambda(expr, function_name, location, errors);
                }
            }
            Statement::Return(Some(expr)) => {
                // Check if return expression is a lambda with reference captures
                // Return statement doesn't have a location in the parser, use a default
                if *unsafe_depth == 0 {
                    let default_location = crate::parser::ast_visitor::SourceLocation {
                        file: "unknown".to_string(),
                        line: 0,
                        column: 0,
                    };
                    check_expression_for_lambda(expr, function_name, &default_location, errors);
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                check_statements(then_branch, function_name, errors, unsafe_depth);
                if let Some(else_stmts) = else_branch {
                    check_statements(else_stmts, function_name, errors, unsafe_depth);
                }
            }
            _ => {}
        }
    }
}

fn check_expression_for_lambda(
    expr: &Expression,
    function_name: &str,
    location: &crate::parser::ast_visitor::SourceLocation,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::Lambda { captures } => {
            debug_println!("DEBUG LAMBDA: Found lambda with {} captures in function '{}'",
                captures.len(), function_name);

            for capture in captures {
                match capture {
                    LambdaCaptureKind::DefaultRef => {
                        errors.push(format!(
                            "Reference capture in @safe code at {}:{}: Default reference capture [&] is forbidden in @safe code - use copy capture [=] instead",
                            location.file, location.line
                        ));
                    }
                    LambdaCaptureKind::ByRef(var_name) => {
                        errors.push(format!(
                            "Reference capture in @safe code at {}:{}: Reference capture of '{}' ([&{}]) is forbidden in @safe code - use copy capture [{}] instead",
                            location.file, location.line, var_name, var_name, var_name
                        ));
                    }
                    LambdaCaptureKind::This => {
                        errors.push(format!(
                            "Reference capture in @safe code at {}:{}: Capturing 'this' is forbidden in @safe code - 'this' is a raw pointer that can dangle",
                            location.file, location.line
                        ));
                    }
                    // Copy captures, init captures, and *this captures are safe
                    LambdaCaptureKind::DefaultCopy |
                    LambdaCaptureKind::ByCopy(_) |
                    LambdaCaptureKind::Init { .. } |
                    LambdaCaptureKind::ThisCopy => {
                        debug_println!("DEBUG LAMBDA: Safe capture: {:?}", capture);
                    }
                }
            }
        }
        // Recursively check nested expressions
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expression_for_lambda(arg, function_name, location, errors);
            }
        }
        Expression::Move(inner) |
        Expression::Dereference(inner) |
        Expression::AddressOf(inner) => {
            check_expression_for_lambda(inner, function_name, location, errors);
        }
        Expression::BinaryOp { left, right, .. } => {
            check_expression_for_lambda(left, function_name, location, errors);
            check_expression_for_lambda(right, function_name, location, errors);
        }
        Expression::MemberAccess { object, .. } => {
            check_expression_for_lambda(object, function_name, location, errors);
        }
        _ => {}
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
    fn test_ref_capture_in_safe_is_error() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::ByRef("x".to_string())],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Reference capture"));
    }

    #[test]
    fn test_default_ref_capture_in_safe_is_error() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::DefaultRef],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("[&]"));
    }

    #[test]
    fn test_this_capture_in_safe_is_error() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::This],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("this"));
    }

    #[test]
    fn test_copy_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::ByCopy("x".to_string())],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert!(errors.is_empty());
    }

    #[test]
    fn test_default_copy_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::DefaultCopy],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert!(errors.is_empty());
    }

    #[test]
    fn test_init_capture_in_safe_is_ok() {
        let lambda = Expression::Lambda {
            captures: vec![LambdaCaptureKind::Init {
                name: "y".to_string(),
                is_move: true,
            }],
        };

        let mut errors = Vec::new();
        check_expression_for_lambda(&lambda, "test", &make_location(), &mut errors);

        assert!(errors.is_empty());
    }
}
