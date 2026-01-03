use crate::parser::{Statement, Expression, Function, MoveKind};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashSet;

/// Check if a type is a char pointer type (char*, const char*, wchar_t*, etc.)
/// These types are unsafe in @safe code because they represent raw pointers
/// that can dangle or be misused.
fn is_char_pointer_type(type_name: &str) -> bool {
    let normalized = type_name.replace(" ", "").to_lowercase();

    // Check for various char pointer patterns
    normalized == "char*" ||
    normalized == "constchar*" ||
    normalized == "charconst*" ||
    normalized == "wchar_t*" ||
    normalized == "constwchar_t*" ||
    normalized == "wchar_tconst*" ||
    normalized == "char16_t*" ||
    normalized == "char32_t*" ||
    // Handle qualified names (e.g., std::char*, ::char*)
    normalized.ends_with("::char*") ||
    normalized.ends_with("::constchar*") ||
    // Handle spacing variations
    type_name.contains("char *") ||
    type_name.contains("char const *") ||
    type_name.contains("const char *") ||
    type_name.contains("wchar_t *") ||
    type_name.contains("wchar_t const *") ||
    type_name.contains("const wchar_t *")
}

/// Check for unsafe pointer operations in a function's AST
#[allow(dead_code)]
pub fn check_function_for_pointers(_function: &crate::ir::IrFunction) -> Result<Vec<String>, String> {
    // For now, return empty - we need to check at AST level
    // The IR doesn't preserve all the pointer operations
    Ok(Vec::new())
}

/// Check for unsafe pointer operations in a parsed function
pub fn check_parsed_function_for_pointers(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unsafe_depth = 0;

    // Only @safe functions have pointer operations checked
    // Undeclared and @unsafe functions are allowed to do pointer operations
    let skip_pointer_checks = function_safety != SafetyMode::Safe;

    // Note: We do NOT check function parameters for char* types.
    // A @safe function CAN take const char* parameters and act as a safe wrapper.
    // The key rule is:
    // - Callers must pass string literals (not char* variables)
    // - The function can internally use @unsafe blocks
    // - Variable declarations of char* inside the function ARE flagged
    //
    // This enables the "safe wrapper" pattern:
    //   void Logger::log(const char* msg) { @unsafe { internal_log(msg); } }

    let stmts = &function.body;
    for (i, stmt) in stmts.iter().enumerate() {
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

        // Skip checking if we're in an unsafe block OR the function is not @safe
        let in_unsafe_scope = unsafe_depth > 0 || skip_pointer_checks;

        // Check for uninitialized pointer declarations in @safe code
        if !in_unsafe_scope {
            if let Some(error) = check_uninitialized_pointer(stmt, stmts.get(i + 1)) {
                errors.push(format!("In function '{}': {}", function.name, error));
            }
        }

        if let Some(error) = check_parsed_statement_for_pointers(stmt, in_unsafe_scope) {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }

    errors
}

/// Check if a pointer variable is declared without initialization
/// In @safe code, pointers must be initialized to prevent use of garbage values
fn check_uninitialized_pointer(current_stmt: &Statement, next_stmt: Option<&Statement>) -> Option<String> {
    use crate::parser::Statement;

    // Check if current statement is a pointer variable declaration
    if let Statement::VariableDecl(var) = current_stmt {
        if !var.is_pointer {
            return None;
        }

        // Skip char* types - they have their own check and may be used with string literals
        if is_char_pointer_type(&var.type_name) {
            return None;
        }

        // Check if next statement is an assignment to this variable
        // If so, the pointer is being initialized
        if let Some(Statement::Assignment { lhs, rhs, .. }) = next_stmt {
            if let Expression::Variable(name) = lhs {
                if name == &var.name {
                    // The pointer is being initialized - check if with nullptr
                    // (nullptr initialization is caught by the Assignment check)
                    if is_null_pointer_expr(rhs) {
                        return Some(format!(
                            "Pointer '{}' initialized with nullptr at line {}: null pointers are forbidden in @safe code. \
                             Use Option<T*> for nullable pointers.",
                            var.name, var.location.line
                        ));
                    }
                    return None; // Initialized with non-null value
                }
            }
        }

        // No initialization found - pointer is uninitialized
        return Some(format!(
            "Uninitialized pointer '{}' at line {}: pointers must be initialized in @safe code. \
             Uninitialized pointers may contain garbage values.",
            var.name, var.location.line
        ));
    }

    None
}

/// Check for std::move on references in @safe code
///
/// In @safe code, using std::move on a reference is forbidden because:
/// - std::move on a reference moves the underlying object, not the reference
/// - This differs from Rust's semantics where references are first-class types
/// - Users should use rusty::move for Rust-like reference move semantics
///
/// This check tracks reference variables and reports errors when std::move is called on them.
pub fn check_std_move_on_references(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    // Track reference variables (including function parameters)
    let mut reference_vars: HashSet<String> = HashSet::new();

    // Add reference parameters
    for param in &function.parameters {
        if param.is_reference {
            reference_vars.insert(param.name.clone());
        }
    }

    // Process function body
    let mut unsafe_depth = 0;
    check_statements_for_std_move_on_ref(
        &function.body,
        &function.name,
        &mut reference_vars,
        &mut unsafe_depth,
        &mut errors,
    );

    errors
}

fn check_statements_for_std_move_on_ref(
    statements: &[Statement],
    function_name: &str,
    reference_vars: &mut HashSet<String>,
    unsafe_depth: &mut usize,
    errors: &mut Vec<String>,
) {
    for stmt in statements {
        // Track unsafe scope depth
        match stmt {
            Statement::EnterUnsafe => {
                *unsafe_depth += 1;
                continue;
            }
            Statement::ExitUnsafe => {
                if *unsafe_depth > 0 {
                    *unsafe_depth -= 1;
                }
                continue;
            }
            _ => {}
        }

        // Skip checking if we're in an unsafe block
        if *unsafe_depth > 0 {
            // Still need to track variable declarations
            if let Statement::VariableDecl(var) = stmt {
                if var.is_reference {
                    reference_vars.insert(var.name.clone());
                }
            }
            continue;
        }

        match stmt {
            Statement::VariableDecl(var) => {
                // Track reference variable declarations
                if var.is_reference {
                    reference_vars.insert(var.name.clone());
                }
                // Note: Variable struct doesn't have an initializer field to check
                // The initialization is handled via Assignment or ReferenceBinding statements
            }
            Statement::Assignment { rhs, location, .. } => {
                if let Some(error) = check_expression_for_std_move_on_ref(rhs, reference_vars, location.line) {
                    errors.push(format!("In function '{}': {}", function_name, error));
                }
            }
            Statement::ReferenceBinding { target, location, .. } => {
                if let Some(error) = check_expression_for_std_move_on_ref(target, reference_vars, location.line) {
                    errors.push(format!("In function '{}': {}", function_name, error));
                }
            }
            Statement::FunctionCall { args, location, .. } => {
                for arg in args {
                    if let Some(error) = check_expression_for_std_move_on_ref(arg, reference_vars, location.line) {
                        errors.push(format!("In function '{}': {}", function_name, error));
                    }
                }
            }
            Statement::Return(Some(expr)) => {
                // Use line 0 for returns (we don't have location info here)
                if let Some(error) = check_expression_for_std_move_on_ref(expr, reference_vars, 0) {
                    errors.push(format!("In function '{}': {}", function_name, error));
                }
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                // Check condition
                if let Some(error) = check_expression_for_std_move_on_ref(condition, reference_vars, 0) {
                    errors.push(format!("In function '{}': {}", function_name, error));
                }

                // Check branches
                check_statements_for_std_move_on_ref(then_branch, function_name, reference_vars, unsafe_depth, errors);
                if let Some(else_stmts) = else_branch {
                    check_statements_for_std_move_on_ref(else_stmts, function_name, reference_vars, unsafe_depth, errors);
                }
            }
            Statement::Block(statements) => {
                check_statements_for_std_move_on_ref(statements, function_name, reference_vars, unsafe_depth, errors);
            }
            Statement::ExpressionStatement { expr, location } => {
                if let Some(error) = check_expression_for_std_move_on_ref(expr, reference_vars, location.line) {
                    errors.push(format!("In function '{}': {}", function_name, error));
                }
            }
            _ => {}
        }
    }
}

fn check_expression_for_std_move_on_ref(
    expr: &Expression,
    reference_vars: &HashSet<String>,
    line: u32,
) -> Option<String> {
    match expr {
        Expression::Move { inner, kind } => {
            // Only check std::move, not rusty::move
            if *kind == MoveKind::StdMove {
                // Check if the inner expression is a reference variable
                if let Expression::Variable(var_name) = inner.as_ref() {
                    if reference_vars.contains(var_name) {
                        return Some(format!(
                            "std::move on reference '{}' at line {}: \
                             In @safe code, std::move on references is forbidden because it moves the underlying object, not the reference. \
                             Use rusty::move for Rust-like reference semantics, or use @unsafe block if you need C++ behavior.",
                            var_name, line
                        ));
                    }
                }
            }
            // Recursively check inner expression
            check_expression_for_std_move_on_ref(inner, reference_vars, line)
        }
        Expression::FunctionCall { args, .. } => {
            for arg in args {
                if let Some(error) = check_expression_for_std_move_on_ref(arg, reference_vars, line) {
                    return Some(error);
                }
            }
            None
        }
        Expression::BinaryOp { left, right, .. } => {
            if let Some(error) = check_expression_for_std_move_on_ref(left, reference_vars, line) {
                return Some(error);
            }
            check_expression_for_std_move_on_ref(right, reference_vars, line)
        }
        Expression::Dereference(inner) | Expression::AddressOf(inner) => {
            check_expression_for_std_move_on_ref(inner, reference_vars, line)
        }
        Expression::MemberAccess { object, .. } => {
            check_expression_for_std_move_on_ref(object, reference_vars, line)
        }
        Expression::Cast(inner) => {
            check_expression_for_std_move_on_ref(inner, reference_vars, line)
        }
        _ => None,
    }
}

/// Process a list of statements while tracking unsafe depth for pointer safety
fn check_statements_for_pointers_with_unsafe_tracking(
    statements: &[Statement],
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

        if let Some(error) = check_parsed_statement_for_pointers(stmt, in_unsafe_scope) {
            errors.push(error);
        }
    }

    errors
}

/// Check if an expression is a null pointer (nullptr, NULL, 0 in pointer context)
fn is_null_pointer_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Nullptr => true,
        // Integer literal 0 can be null in pointer context - but we'll be conservative
        // and only flag explicit nullptr for now
        _ => false,
    }
}

/// Check if a function name is an unsafe memory management function
/// These are forbidden in @safe code - use smart pointers instead
fn is_unsafe_memory_function(name: &str) -> bool {
    // Get the base function name (strip namespace qualifiers)
    let base_name = name.rsplit("::").next().unwrap_or(name);

    matches!(base_name,
        // C memory functions
        "malloc" | "calloc" | "realloc" | "free" |
        // C aligned memory functions
        "aligned_alloc" | "posix_memalign" | "memalign" |
        // C++ sized deallocation (rare but possible)
        "operator new" | "operator delete" |
        // Platform-specific allocators
        "_aligned_malloc" | "_aligned_free" | "_aligned_realloc"
    )
}

/// Check if a parsed statement contains pointer operations
pub fn check_parsed_statement_for_pointers(stmt: &Statement, in_unsafe_scope: bool) -> Option<String> {
    use crate::parser::Statement;

    // Skip all checks if we're in an unsafe block
    if in_unsafe_scope {
        return None;
    }

    match stmt {
        Statement::Assignment { lhs, rhs, location } => {
            // Check for null pointer assignment: p = nullptr is forbidden in @safe
            if is_null_pointer_expr(rhs) {
                // Check if lhs is a pointer variable (simplified check)
                if let Expression::Variable(_) = lhs {
                    return Some(format!(
                        "Null pointer assignment at line {}: null pointers are forbidden in @safe code. \
                         Use Option<T*> for nullable pointers.",
                        location.line
                    ));
                }
            }

            // Check BOTH lhs and rhs for pointer operations
            // e.g., `n->value_ = val` has the dereference on lhs
            // e.g., `x = *ptr` has the dereference on rhs
            if let Some(op) = contains_pointer_operation(lhs) {
                return Some(format!(
                    "Unsafe pointer {} at line {}: pointer operations require unsafe context",
                    op, location.line
                ));
            }
            if let Some(op) = contains_pointer_operation(rhs) {
                return Some(format!(
                    "Unsafe pointer {} at line {}: pointer operations require unsafe context",
                    op, location.line
                ));
            }
        }
        Statement::VariableDecl(var) if var.is_pointer => {
            // Check if this is a char* type declaration
            // char* and const char* are unsafe in @safe code because they are raw pointers
            // String literals are safe (handled separately), but explicit char* variables are not
            if is_char_pointer_type(&var.type_name) {
                return Some(format!(
                    "Cannot declare '{}' with type '{}' in @safe code at line {}. \
                     Use @unsafe block or a safe wrapper type. \
                     (String literals like \"hello\" are safe; explicit char* variables are not)",
                    var.name, var.type_name, var.location.line
                ));
            }
            // Other raw pointer declarations are allowed (dereferencing is still checked)
            return None;
        }
        Statement::FunctionCall { name, args, location, .. } => {
            // Check for forbidden memory management functions
            if is_unsafe_memory_function(name) {
                return Some(format!(
                    "Unsafe memory function '{}' at line {}: manual memory management is forbidden in @safe code. \
                     Use smart pointers (Box, unique_ptr) instead.",
                    name, location.line
                ));
            }

            for arg in args {
                // Check for nullptr passed as argument - forbidden in @safe code
                if is_null_pointer_expr(arg) {
                    return Some(format!(
                        "Null pointer passed as argument at line {}: null pointers are forbidden in @safe code. \
                         Use Option<T*> for nullable pointers.",
                        location.line
                    ));
                }
                if let Some(op) = contains_pointer_operation(arg) {
                    return Some(format!(
                        "Unsafe pointer {} in function call at line {}: pointer operations require unsafe context",
                        op, location.line
                    ));
                }
            }
        }
        Statement::Return(Some(expr)) => {
            // Check for returning nullptr - forbidden in @safe code
            if is_null_pointer_expr(expr) {
                return Some(
                    "Cannot return nullptr in @safe code: null pointers are forbidden. \
                     Use Option<T*> for nullable pointers.".to_string()
                );
            }
            if let Some(op) = contains_pointer_operation(expr) {
                return Some(format!(
                    "Unsafe pointer {} in return statement: pointer operations require unsafe context",
                    op
                ));
            }
        }
        Statement::If { condition, then_branch, else_branch, location } => {
            if let Some(op) = contains_pointer_operation(condition) {
                return Some(format!(
                    "Unsafe pointer {} in condition at line {}: pointer operations require unsafe context",
                    op, location.line
                ));
            }

            // Recursively check branches with proper unsafe depth tracking
            let then_errors = check_statements_for_pointers_with_unsafe_tracking(then_branch, 0);
            if !then_errors.is_empty() {
                return Some(then_errors.into_iter().next().unwrap());
            }

            if let Some(else_stmts) = else_branch {
                let else_errors = check_statements_for_pointers_with_unsafe_tracking(else_stmts, 0);
                if !else_errors.is_empty() {
                    return Some(else_errors.into_iter().next().unwrap());
                }
            }
        }
        Statement::Block(statements) => {
            // Check all statements in the block with proper unsafe depth tracking
            let block_errors = check_statements_for_pointers_with_unsafe_tracking(statements, 0);
            if !block_errors.is_empty() {
                return Some(block_errors.into_iter().next().unwrap());
            }
        }
        Statement::ExpressionStatement { expr, location } => {
            // Check for pointer operations in standalone expressions (e.g., `delete p;`)
            if let Some(op) = contains_pointer_operation(expr) {
                return Some(format!(
                    "Unsafe pointer {} at line {}: pointer operations require unsafe context",
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
        Expression::Dereference(inner) => {
            // *this is safe in member functions - this pointer is guaranteed valid
            if let Expression::Variable(name) = inner.as_ref() {
                if name == "this" {
                    return None;  // *this is safe
                }
            }
            Some("dereference")
        }
        Expression::AddressOf(inner) => {
            // Check what we're taking the address of
            match inner.as_ref() {
                // For MemberAccess, recursively check if the object contains unsafe operations
                // e.g., &(ptr->field) has a Dereference inside which is unsafe
                // e.g., &(static_cast<T*>(p)->field) has both Cast and Dereference
                Expression::MemberAccess { object, .. } => contains_pointer_operation(object),
                // &ClassName::method often appears as Variable("ClassName::method")
                // due to how C++ qualified names are parsed - this is safe (member function pointer)
                Expression::Variable(name) if name.contains("::") => None,
                // &variable - taking address of a local variable is unsafe (could create dangling pointers)
                _ => Some("address-of")
            }
        }
        Expression::Variable(name) if name == "this" => {
            // Passing 'this' as a raw pointer is unsafe - the callee might store it
            // and cause dangling pointer issues later. While 'this' is valid during
            // the call, we can't guarantee how the callee uses it.
            // Note: *this (dereference) is safe, but passing 'this' itself is not.
            Some("'this' pointer")
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
        Expression::MemberAccess { object, .. } => {
            // this->member is safe - just accessing a member through the implicit this pointer
            // ptr->field (dereference through pointer) is handled by the parser wrapping object in Dereference
            if let Expression::Variable(name) = object.as_ref() {
                if name == "this" {
                    return None;  // this->member is safe
                }
            }
            // For other cases, check object for pointer operations
            contains_pointer_operation(object)
        }
        Expression::Cast(inner) => {
            // C++ casts (static_cast, dynamic_cast, reinterpret_cast, const_cast, C-style)
            // are all considered unsafe operations in @safe code
            // Return "cast" as the operation type, but also check inner for other violations
            Some("cast")
        }
        Expression::StringLiteral(_) => {
            // String literals have static lifetime and cannot dangle
            // They are stored in the .rodata segment and are always safe
            None
        }
        Expression::Literal(_) => {
            // Numeric and other literals are safe
            None
        }
        Expression::Lambda { .. } => {
            // Lambda safety is checked elsewhere (capture analysis)
            None
        }
        Expression::Move { inner, .. } => {
            // Check inner expression for pointer operations
            contains_pointer_operation(inner)
        }
        Expression::Variable(_) => {
            // Regular variable references (not 'this') are safe
            // Note: 'this' is handled above with a guard
            None
        }
        Expression::Nullptr => {
            // Nullptr literal - not a pointer operation itself
            // (null checks are handled separately by is_null_pointer_expr)
            None
        }
        Expression::New(_) => {
            // new expression - manual memory management is unsafe
            // Use smart pointers (Box, unique_ptr) instead
            Some("new")
        }
        Expression::Delete(_) => {
            // delete expression - manual memory management is unsafe
            // Use smart pointers that handle deallocation automatically
            Some("delete")
        }
        Expression::PointerArithmetic { .. } => {
            // Pointer arithmetic is unsafe - can cause out-of-bounds access
            // Use iterators or safe containers instead
            Some("pointer arithmetic")
        }
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
        
        let error = check_parsed_statement_for_pointers(&stmt, false);
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

        let error = check_parsed_statement_for_pointers(&stmt, false);
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

        let error = check_parsed_statement_for_pointers(&stmt, false);
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
            is_mutable: false,
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 5,
                column: 5,
            },
            is_pack: false,
            pack_element_type: None,
        });

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_none(), "Pointer declaration should be allowed");
    }

    #[test]
    fn test_this_pointer_in_function_call() {
        // Passing 'this' as an argument should be flagged as unsafe
        let stmt = Statement::FunctionCall {
            name: "register".to_string(),
            args: vec![
                Expression::Variable("this".to_string())
            ],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 25,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "Passing 'this' as argument should be flagged");
        let error_msg = error.unwrap();
        assert!(error_msg.contains("'this' pointer"), "Error should mention 'this' pointer");
    }

    #[test]
    fn test_this_dereference_is_safe() {
        // *this is safe - dereferencing this in a member function is valid
        let expr = Expression::Dereference(Box::new(Expression::Variable("this".to_string())));
        assert_eq!(contains_pointer_operation(&expr), None, "*this should be safe");
    }

    #[test]
    fn test_this_as_variable_is_unsafe() {
        // 'this' by itself (passed as pointer) is unsafe
        let expr = Expression::Variable("this".to_string());
        assert_eq!(contains_pointer_operation(&expr), Some("'this' pointer"));
    }

    #[test]
    fn test_this_member_access_is_safe() {
        // this->member is safe - just accessing a member through the implicit this pointer
        let expr = Expression::MemberAccess {
            object: Box::new(Expression::Variable("this".to_string())),
            field: "value_".to_string(),
        };
        assert_eq!(contains_pointer_operation(&expr), None, "this->member should be safe");
    }

    #[test]
    fn test_member_function_pointer_is_safe() {
        // &ClassName::method is safe - member function pointers don't involve object lifetimes
        // This tests the MemberAccess variant
        let expr = Expression::AddressOf(Box::new(Expression::MemberAccess {
            object: Box::new(Expression::Variable("TestService".to_string())),
            field: "echo_wrapper".to_string(),
        }));
        assert_eq!(contains_pointer_operation(&expr), None, "&ClassName::method (MemberAccess) should be safe");
    }

    #[test]
    fn test_qualified_member_function_pointer_is_safe() {
        // &ClassName::method as Variable("ClassName::method") is safe
        // The parser produces this form for qualified member function pointers
        let expr = Expression::AddressOf(Box::new(Expression::Variable("TestService::echo_wrapper".to_string())));
        assert_eq!(contains_pointer_operation(&expr), None, "&ClassName::method (qualified Variable) should be safe");
    }

    #[test]
    fn test_address_of_variable_is_unsafe() {
        // &x is unsafe - taking address of a local variable
        let expr = Expression::AddressOf(Box::new(Expression::Variable("x".to_string())));
        assert_eq!(contains_pointer_operation(&expr), Some("address-of"), "&variable should be unsafe");
    }

    // Tests for is_char_pointer_type
    #[test]
    fn test_char_ptr_detection() {
        assert!(super::is_char_pointer_type("char*"));
        assert!(super::is_char_pointer_type("char *"));
        assert!(super::is_char_pointer_type("const char*"));
        assert!(super::is_char_pointer_type("const char *"));
        assert!(super::is_char_pointer_type("char const*"));
        assert!(super::is_char_pointer_type("char const *"));
    }

    #[test]
    fn test_wchar_ptr_detection() {
        assert!(super::is_char_pointer_type("wchar_t*"));
        assert!(super::is_char_pointer_type("wchar_t *"));
        assert!(super::is_char_pointer_type("const wchar_t*"));
        assert!(super::is_char_pointer_type("const wchar_t *"));
    }

    #[test]
    fn test_char16_char32_ptr_detection() {
        assert!(super::is_char_pointer_type("char16_t*"));
        assert!(super::is_char_pointer_type("char32_t*"));
    }

    #[test]
    fn test_non_char_ptr_not_detected() {
        assert!(!super::is_char_pointer_type("int*"));
        assert!(!super::is_char_pointer_type("void*"));
        assert!(!super::is_char_pointer_type("std::string"));
        assert!(!super::is_char_pointer_type("char")); // Not a pointer
    }

    #[test]
    fn test_string_literal_expression_is_safe() {
        let expr = Expression::StringLiteral("hello".to_string());
        assert_eq!(contains_pointer_operation(&expr), None, "String literal should be safe");
    }

    #[test]
    fn test_new_expression_is_unsafe() {
        let expr = Expression::New(Box::new(Expression::Literal("int".to_string())));
        assert_eq!(contains_pointer_operation(&expr), Some("new"), "new expression should be unsafe");
    }

    #[test]
    fn test_delete_expression_is_unsafe() {
        let expr = Expression::Delete(Box::new(Expression::Variable("ptr".to_string())));
        assert_eq!(contains_pointer_operation(&expr), Some("delete"), "delete expression should be unsafe");
    }

    #[test]
    fn test_new_in_statement() {
        let stmt = Statement::ExpressionStatement {
            expr: Expression::New(Box::new(Expression::Literal("int".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "new in statement should be detected");
        assert!(error.unwrap().contains("new"), "Error should mention 'new'");
    }

    #[test]
    fn test_delete_in_statement() {
        let stmt = Statement::ExpressionStatement {
            expr: Expression::Delete(Box::new(Expression::Variable("ptr".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 20,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "delete in statement should be detected");
        assert!(error.unwrap().contains("delete"), "Error should mention 'delete'");
    }

    #[test]
    fn test_new_delete_allowed_in_unsafe() {
        // new in unsafe context should be allowed
        let stmt = Statement::ExpressionStatement {
            expr: Expression::New(Box::new(Expression::Literal("int".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, true);  // in_unsafe_scope = true
        assert!(error.is_none(), "new should be allowed in unsafe context");

        // delete in unsafe context should be allowed
        let stmt2 = Statement::ExpressionStatement {
            expr: Expression::Delete(Box::new(Expression::Variable("ptr".to_string()))),
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 20,
                column: 5,
            },
        };

        let error2 = check_parsed_statement_for_pointers(&stmt2, true);  // in_unsafe_scope = true
        assert!(error2.is_none(), "delete should be allowed in unsafe context");
    }

    #[test]
    fn test_pointer_arithmetic_is_unsafe() {
        let expr = Expression::PointerArithmetic {
            pointer: Box::new(Expression::Variable("ptr".to_string())),
            op: "+".to_string(),
        };
        assert_eq!(contains_pointer_operation(&expr), Some("pointer arithmetic"), "Pointer arithmetic should be unsafe");
    }

    #[test]
    fn test_pointer_arithmetic_in_statement() {
        let stmt = Statement::ExpressionStatement {
            expr: Expression::PointerArithmetic {
                pointer: Box::new(Expression::Variable("ptr".to_string())),
                op: "++".to_string(),
            },
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "Pointer arithmetic should be detected");
        assert!(error.unwrap().contains("pointer arithmetic"), "Error should mention pointer arithmetic");
    }

    #[test]
    fn test_pointer_arithmetic_allowed_in_unsafe() {
        let stmt = Statement::ExpressionStatement {
            expr: Expression::PointerArithmetic {
                pointer: Box::new(Expression::Variable("ptr".to_string())),
                op: "++".to_string(),
            },
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, true);  // in_unsafe_scope = true
        assert!(error.is_none(), "Pointer arithmetic should be allowed in unsafe context");
    }

    #[test]
    fn test_malloc_is_unsafe() {
        let stmt = Statement::FunctionCall {
            name: "malloc".to_string(),
            args: vec![Expression::Literal("100".to_string())],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "malloc should be detected as unsafe");
        assert!(error.unwrap().contains("malloc"), "Error should mention malloc");
    }

    #[test]
    fn test_free_is_unsafe() {
        let stmt = Statement::FunctionCall {
            name: "free".to_string(),
            args: vec![Expression::Variable("ptr".to_string())],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "free should be detected as unsafe");
        assert!(error.unwrap().contains("free"), "Error should mention free");
    }

    #[test]
    fn test_calloc_is_unsafe() {
        let stmt = Statement::FunctionCall {
            name: "calloc".to_string(),
            args: vec![
                Expression::Literal("10".to_string()),
                Expression::Literal("4".to_string()),
            ],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "calloc should be detected as unsafe");
        assert!(error.unwrap().contains("calloc"), "Error should mention calloc");
    }

    #[test]
    fn test_realloc_is_unsafe() {
        let stmt = Statement::FunctionCall {
            name: "realloc".to_string(),
            args: vec![
                Expression::Variable("ptr".to_string()),
                Expression::Literal("200".to_string()),
            ],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, false);
        assert!(error.is_some(), "realloc should be detected as unsafe");
        assert!(error.unwrap().contains("realloc"), "Error should mention realloc");
    }

    #[test]
    fn test_malloc_allowed_in_unsafe() {
        let stmt = Statement::FunctionCall {
            name: "malloc".to_string(),
            args: vec![Expression::Literal("100".to_string())],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 10,
                column: 5,
            },
        };

        let error = check_parsed_statement_for_pointers(&stmt, true);  // in_unsafe_scope = true
        assert!(error.is_none(), "malloc should be allowed in unsafe context");
    }

    #[test]
    fn test_is_unsafe_memory_function() {
        // Test all the memory functions
        assert!(is_unsafe_memory_function("malloc"));
        assert!(is_unsafe_memory_function("calloc"));
        assert!(is_unsafe_memory_function("realloc"));
        assert!(is_unsafe_memory_function("free"));
        assert!(is_unsafe_memory_function("aligned_alloc"));
        assert!(is_unsafe_memory_function("posix_memalign"));
        assert!(is_unsafe_memory_function("memalign"));

        // Test with namespace prefix
        assert!(is_unsafe_memory_function("std::malloc"));
        assert!(is_unsafe_memory_function("::free"));

        // Test non-memory functions
        assert!(!is_unsafe_memory_function("printf"));
        assert!(!is_unsafe_memory_function("std::vector::push_back"));
        assert!(!is_unsafe_memory_function("new_function"));  // Not "new"
        assert!(!is_unsafe_memory_function("allocate"));  // Similar but not a memory function
    }
}