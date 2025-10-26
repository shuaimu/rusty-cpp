use crate::parser::{Statement, Expression, Function};

/// Check for unsafe pointer operations in a function's AST
#[allow(dead_code)]
pub fn check_function_for_pointers(_function: &crate::ir::IrFunction) -> Result<Vec<String>, String> {
    // For now, return empty - we need to check at AST level
    // The IR doesn't preserve all the pointer operations
    Ok(Vec::new())
}

/// Check for unsafe pointer operations in a parsed function
pub fn check_parsed_function_for_pointers(function: &Function) -> Vec<String> {
    let mut errors = Vec::new();
    
    for stmt in &function.body {
        if let Some(error) = check_parsed_statement_for_pointers(stmt) {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }
    
    errors
}

/// Check if a parsed statement contains pointer operations
pub fn check_parsed_statement_for_pointers(stmt: &Statement) -> Option<String> {
    use crate::parser::Statement;
    
    match stmt {
        Statement::Assignment { rhs, location, .. } => {
            if let Some(op) = contains_pointer_operation(rhs) {
                return Some(format!(
                    "Unsafe pointer {} at line {}: pointer operations require unsafe context",
                    op, location.line
                ));
            }
        }
        Statement::VariableDecl(var) if var.is_pointer => {
            // Raw pointer declaration is allowed, but dereferencing isn't
            return None;
        }
        Statement::FunctionCall { args, location, .. } => {
            for arg in args {
                if let Some(op) = contains_pointer_operation(arg) {
                    return Some(format!(
                        "Unsafe pointer {} in function call at line {}: pointer operations require unsafe context",
                        op, location.line
                    ));
                }
            }
        }
        Statement::Return(Some(expr)) => {
            if let Some(op) = contains_pointer_operation(expr) {
                return Some(format!(
                    "Unsafe pointer {} in return statement: pointer operations require unsafe context",
                    op
                ));
            }
        }
        Statement::If { condition, location, .. } => {
            if let Some(op) = contains_pointer_operation(condition) {
                return Some(format!(
                    "Unsafe pointer {} in condition at line {}: pointer operations require unsafe context", 
                    op, location.line
                ));
            }
        }
        _ => {}
    }
    
    None
}

fn contains_pointer_operation(expr: &Expression) -> Option<&'static str> {
    use crate::parser::Expression;
    
    match expr {
        Expression::Dereference(_) => Some("dereference"),
        Expression::AddressOf(_) => {
            // Taking address is generally safe in Rust, but we can make it require unsafe too
            Some("address-of")
        }
        Expression::FunctionCall { args, .. } => {
            // Check arguments recursively
            for arg in args {
                if let Some(op) = contains_pointer_operation(arg) {
                    return Some(op);
                }
            }
            None
        }
        Expression::BinaryOp { left, right, .. } => {
            // Check both sides
            if let Some(op) = contains_pointer_operation(left) {
                return Some(op);
            }
            contains_pointer_operation(right)
        }
        _ => None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Expression, Statement, SourceLocation, Variable};
    
    #[test]
    fn test_detect_dereference() {
        let expr = Expression::Dereference(Box::new(Expression::Variable("ptr".to_string())));
        assert_eq!(contains_pointer_operation(&expr), Some("dereference"));
    }
    
    #[test]
    fn test_detect_address_of() {
        let expr = Expression::AddressOf(Box::new(Expression::Variable("x".to_string())));
        assert_eq!(contains_pointer_operation(&expr), Some("address-of"));
    }
    
    #[test]
    fn test_safe_expression() {
        let expr = Expression::Variable("x".to_string());
        assert_eq!(contains_pointer_operation(&expr), None);
    }
    
    #[test]
    fn test_pointer_in_assignment() {
        let stmt = Statement::Assignment {
            lhs: crate::parser::Expression::Variable("x".to_string()),
            rhs: Expression::Dereference(Box::new(Expression::Variable("ptr".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };
        
        let error = check_parsed_statement_for_pointers(&stmt);
        assert!(error.is_some());
        assert!(error.unwrap().contains("dereference"));
    }
    
    #[test]
    fn test_address_of_in_assignment() {
        let stmt = Statement::Assignment {
            lhs: crate::parser::Expression::Variable("ptr".to_string()),
            rhs: Expression::AddressOf(Box::new(Expression::Variable("x".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 20,
                column: 5,
            },
        };
        
        let error = check_parsed_statement_for_pointers(&stmt);
        assert!(error.is_some());
        assert!(error.unwrap().contains("address-of"));
    }
    
    #[test]
    fn test_pointer_in_function_call() {
        let stmt = Statement::FunctionCall {
            name: "process".to_string(),
            args: vec![
                Expression::Dereference(Box::new(Expression::Variable("ptr".to_string())))
            ],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 15,
                column: 5,
            },
        };
        
        let error = check_parsed_statement_for_pointers(&stmt);
        assert!(error.is_some());
        let error_msg = error.unwrap();
        assert!(error_msg.contains("function call"));
        assert!(error_msg.contains("dereference"));
    }
    
    #[test]
    fn test_nested_pointer_operations() {
        // Test *(&x) - dereference of address-of
        let expr = Expression::Dereference(Box::new(
            Expression::AddressOf(Box::new(Expression::Variable("x".to_string())))
        ));
        assert_eq!(contains_pointer_operation(&expr), Some("dereference"));
    }
    
    #[test]
    fn test_pointer_in_binary_op() {
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Dereference(Box::new(Expression::Variable("p1".to_string())))),
            op: "+".to_string(),
            right: Box::new(Expression::Variable("x".to_string())),
        };
        assert_eq!(contains_pointer_operation(&expr), Some("dereference"));
    }
    
    #[test]
    fn test_pointer_declaration_allowed() {
        // Declaring a pointer variable should not trigger an error (only operations do)
        let stmt = Statement::VariableDecl(Variable {
            name: "ptr".to_string(),
            type_name: "int*".to_string(),
            is_reference: false,
            is_pointer: true,
            is_const: false,
            is_unique_ptr: false,
            is_shared_ptr: false,
            is_static: false,
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 5,
                column: 5,
            },
        });
        
        let error = check_parsed_statement_for_pointers(&stmt);
        assert!(error.is_none(), "Pointer declaration should be allowed");
    }
}