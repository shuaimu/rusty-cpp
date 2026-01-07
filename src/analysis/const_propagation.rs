//! Const Propagation Through Pointer Members
//!
//! In C++, `const` doesn't propagate through pointer members:
//! ```cpp
//! struct Outer { Inner* ptr; };
//! void foo(const Outer* outer) {
//!     outer->ptr->mutate();  // C++ allows this!
//! }
//! ```
//!
//! In @safe code, we enforce that const propagates:
//! - If you access a pointer member through a const object, the pointer is treated as const
//! - Non-const method calls through such pointers are forbidden
//! - Assignments through such pointers are forbidden

use crate::parser::{Function, Statement, Expression, Variable};
use crate::parser::ast_visitor::Class;
use crate::parser::safety_annotations::SafetyMode;
use std::collections::{HashMap, HashSet};

/// Information about a class's pointer members and their types
#[derive(Debug, Clone)]
struct ClassInfo {
    /// Map of member name -> member info
    pointer_members: HashSet<String>,
}

/// Check for const propagation violations in @safe functions
pub fn check_const_propagation(
    functions: &[Function],
    classes: &[Class],
) -> Vec<String> {
    let mut errors = Vec::new();

    // Build map of class name -> pointer members
    let class_info = build_class_info(classes);

    for function in functions {
        // Only check @safe functions
        let func_safety = function.safety_annotation.unwrap_or(SafetyMode::Unsafe);
        if func_safety != SafetyMode::Safe {
            continue;
        }

        // Build map of const variables (variables that point to const objects)
        let const_vars = find_const_pointer_variables(function);

        // Check for const propagation violations
        let func_errors = check_function_for_const_violations(
            function,
            &const_vars,
            &class_info,
        );

        for error in func_errors {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }

    errors
}

/// Build a map of class names to their pointer members
fn build_class_info(classes: &[Class]) -> HashMap<String, ClassInfo> {
    let mut info = HashMap::new();

    for class in classes {
        let pointer_members: HashSet<String> = class.members.iter()
            .filter(|m| m.is_pointer)
            .map(|m| m.name.clone())
            .collect();

        if !pointer_members.is_empty() {
            info.insert(class.name.clone(), ClassInfo { pointer_members });
        }
    }

    info
}

/// Find variables that are const pointers/references to objects
/// Returns a set of variable names that should propagate const to their members
fn find_const_pointer_variables(function: &Function) -> HashSet<String> {
    let mut const_vars = HashSet::new();

    // Check parameters for const pointers/references
    for param in &function.parameters {
        if is_const_pointer_or_ref(param) {
            const_vars.insert(param.name.clone());
        }
    }

    // Check if this is a const method (implicit 'this' is const)
    if let Some(qualifier) = &function.method_qualifier {
        if *qualifier == crate::parser::ast_visitor::MethodQualifier::Const {
            const_vars.insert("this".to_string());
        }
    }

    // TODO: Also track local const pointer/ref variables
    // This would require analyzing variable declarations in the body

    const_vars
}

/// Check if a variable is a const pointer or reference
fn is_const_pointer_or_ref(var: &Variable) -> bool {
    // Check if the variable is a pointer or reference to const
    // For `const T*` or `const T&`, is_const should be true
    if (var.is_pointer || var.is_reference) && var.is_const {
        return true;
    }

    // Also check type_name for patterns like "const X *" or "const X &"
    let type_lower = var.type_name.to_lowercase();
    if type_lower.starts_with("const ") &&
       (type_lower.contains('*') || type_lower.contains('&')) {
        return true;
    }

    false
}

/// Check a function body for const propagation violations
fn check_function_for_const_violations(
    function: &Function,
    const_vars: &HashSet<String>,
    class_info: &HashMap<String, ClassInfo>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut unsafe_depth = 0;

    for stmt in &function.body {
        match stmt {
            Statement::EnterUnsafe => {
                unsafe_depth += 1;
            }
            Statement::ExitUnsafe => {
                if unsafe_depth > 0 {
                    unsafe_depth -= 1;
                }
            }
            _ if unsafe_depth > 0 => {
                // Skip checks inside @unsafe blocks
                continue;
            }
            Statement::FunctionCall { name, args, location, .. } => {
                // Check if this is a method call through a const-propagated path
                if let Some(error) = check_method_call_const_propagation(
                    name, args, location.line, const_vars, class_info
                ) {
                    errors.push(error);
                }
            }
            Statement::Assignment { lhs, rhs: _, location } => {
                // Check if assigning through a const-propagated path
                if let Some(error) = check_assignment_const_propagation(
                    lhs, location.line, const_vars, class_info
                ) {
                    errors.push(error);
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                // Recursively check branches
                let branch_errors = check_statements_for_const_violations(
                    then_branch, const_vars, class_info, unsafe_depth
                );
                errors.extend(branch_errors);

                if let Some(else_stmts) = else_branch {
                    let else_errors = check_statements_for_const_violations(
                        else_stmts, const_vars, class_info, unsafe_depth
                    );
                    errors.extend(else_errors);
                }
            }
            Statement::Block(stmts) => {
                let block_errors = check_statements_for_const_violations(
                    stmts, const_vars, class_info, unsafe_depth
                );
                errors.extend(block_errors);
            }
            _ => {}
        }
    }

    errors
}

/// Helper to check a list of statements
fn check_statements_for_const_violations(
    statements: &[Statement],
    const_vars: &HashSet<String>,
    class_info: &HashMap<String, ClassInfo>,
    mut unsafe_depth: usize,
) -> Vec<String> {
    let mut errors = Vec::new();

    for stmt in statements {
        match stmt {
            Statement::EnterUnsafe => {
                unsafe_depth += 1;
            }
            Statement::ExitUnsafe => {
                if unsafe_depth > 0 {
                    unsafe_depth -= 1;
                }
            }
            _ if unsafe_depth > 0 => {
                continue;
            }
            Statement::FunctionCall { name, args, location, .. } => {
                if let Some(error) = check_method_call_const_propagation(
                    name, args, location.line, const_vars, class_info
                ) {
                    errors.push(error);
                }
            }
            Statement::Assignment { lhs, location, .. } => {
                if let Some(error) = check_assignment_const_propagation(
                    lhs, location.line, const_vars, class_info
                ) {
                    errors.push(error);
                }
            }
            _ => {}
        }
    }

    errors
}

/// Check if a method call violates const propagation
/// Pattern: const_var->ptr_member->non_const_method()
fn check_method_call_const_propagation(
    func_name: &str,
    args: &[Expression],
    line: u32,
    const_vars: &HashSet<String>,
    _class_info: &HashMap<String, ClassInfo>,
) -> Option<String> {
    // Method calls in our parser look like "Class::method" with receiver in args[0]
    // Or they might be parsed as "receiver.method" patterns

    // Check if the first argument (receiver) is accessed through a const path
    if args.is_empty() {
        return None;
    }

    let receiver = &args[0];

    // Check if receiver is a const-propagated access chain
    if let Some(const_source) = get_const_source_in_chain(receiver, const_vars) {
        // Check if the method is non-const
        // For now, we assume methods not ending with "const" are non-const
        // A more robust solution would check the actual method declaration
        if !is_likely_const_method(func_name) {
            return Some(format!(
                "Const propagation violation at line {}: calling non-const method '{}' \
                 through const object '{}'. In @safe code, const propagates through pointer members.",
                line, func_name, const_source
            ));
        }
    }

    None
}

/// Check if an assignment violates const propagation
fn check_assignment_const_propagation(
    lhs: &Expression,
    line: u32,
    const_vars: &HashSet<String>,
    _class_info: &HashMap<String, ClassInfo>,
) -> Option<String> {
    // Check if lhs is accessed through a const-propagated path
    if let Some(const_source) = get_const_source_in_chain(lhs, const_vars) {
        return Some(format!(
            "Const propagation violation at line {}: cannot assign through const object '{}'. \
             In @safe code, const propagates through pointer members.",
            line, const_source
        ));
    }

    None
}

/// Check if an expression chain starts with a const variable and goes through pointer members
/// Returns the name of the const source if found
fn get_const_source_in_chain(expr: &Expression, const_vars: &HashSet<String>) -> Option<String> {
    match expr {
        Expression::Variable(name) => {
            if const_vars.contains(name) {
                Some(name.clone())
            } else {
                None
            }
        }
        Expression::MemberAccess { object, field: _ } => {
            // Check if the object is accessed through a const path
            get_const_source_in_chain(object, const_vars)
        }
        Expression::Dereference(inner) => {
            // Dereferencing a pointer accessed through const path
            get_const_source_in_chain(inner, const_vars)
        }
        Expression::FunctionCall { args, .. } => {
            // For chained method calls, check the receiver (first arg)
            if let Some(receiver) = args.first() {
                get_const_source_in_chain(receiver, const_vars)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Heuristic to determine if a method is likely const
/// A proper implementation would check the actual method declaration
fn is_likely_const_method(func_name: &str) -> bool {
    // Common const method patterns
    // Include both snake_case (is_) and camelCase (is) variants
    let const_patterns = [
        "get", "size", "length", "empty", "is", "has", "can",
        "begin", "end", "cbegin", "cend", "front", "back",
        "at", "find", "count", "contains", "data",
        "c_str", "str", "to_string", "to_",
    ];

    let name_lower = func_name.to_lowercase();

    // Extract just the method name (after last ::)
    let method_name = if let Some(pos) = name_lower.rfind("::") {
        &name_lower[pos + 2..]
    } else {
        &name_lower
    };

    // Check for common const method patterns
    for pattern in const_patterns {
        if method_name.starts_with(pattern) || method_name == pattern {
            return true;
        }
    }

    // Methods ending in "_const" or containing "const" are likely const
    if method_name.contains("const") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_const_pointer_or_ref() {
        let const_ptr = Variable {
            name: "outer".to_string(),
            type_name: "const Outer *".to_string(),
            is_reference: false,
            is_pointer: true,
            is_const: true,
            is_unique_ptr: false,
            is_shared_ptr: false,
            is_static: false,
            is_mutable: false,
            location: crate::parser::SourceLocation {
                file: "test.cpp".to_string(),
                line: 1,
                column: 1,
            },
            is_pack: false,
            pack_element_type: None,
        };
        assert!(is_const_pointer_or_ref(&const_ptr));

        let non_const_ptr = Variable {
            name: "outer".to_string(),
            type_name: "Outer *".to_string(),
            is_reference: false,
            is_pointer: true,
            is_const: false,
            is_unique_ptr: false,
            is_shared_ptr: false,
            is_static: false,
            is_mutable: false,
            location: crate::parser::SourceLocation {
                file: "test.cpp".to_string(),
                line: 1,
                column: 1,
            },
            is_pack: false,
            pack_element_type: None,
        };
        assert!(!is_const_pointer_or_ref(&non_const_ptr));
    }

    #[test]
    fn test_is_likely_const_method() {
        assert!(is_likely_const_method("get_value"));
        assert!(is_likely_const_method("size"));
        assert!(is_likely_const_method("isEmpty"));
        assert!(is_likely_const_method("Container::begin"));
        assert!(!is_likely_const_method("mutate"));
        assert!(!is_likely_const_method("set_value"));
        assert!(!is_likely_const_method("push_back"));
    }
}
