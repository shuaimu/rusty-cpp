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

/// Placeholder for class pointer member tracking (for future expansion)
type ClassInfo = ();

/// Check for const propagation violations in @safe functions
pub fn check_const_propagation(
    functions: &[Function],
    classes: &[Class],
) -> Vec<String> {
    let mut errors = Vec::new();

    // Build map of class name -> pointer members
    let class_info = build_class_info(classes);

    // Build set of @safe function names (for checking if callee is safe)
    let safe_functions = build_safe_function_set(functions);

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
            &safe_functions,
        );

        for error in func_errors {
            errors.push(format!("In function '{}': {}", function.name, error));
        }
    }

    errors
}

/// Build a set of function names that are marked @safe
fn build_safe_function_set(functions: &[Function]) -> HashSet<String> {
    let mut safe_funcs = HashSet::new();

    for func in functions {
        if func.safety_annotation == Some(SafetyMode::Safe) {
            safe_funcs.insert(func.name.clone());
            // Also add normalized version for template matching
            let normalized = normalize_template_name(&func.name);
            if normalized != func.name {
                safe_funcs.insert(normalized);
            }
        }
    }

    safe_funcs
}

/// Build a map of class names to their pointer members (placeholder for future expansion)
fn build_class_info(classes: &[Class]) -> HashMap<String, ClassInfo> {
    let mut info = HashMap::new();

    for class in classes {
        let has_pointer_members = class.members.iter()
            .any(|m| m.is_pointer);

        if has_pointer_members {
            info.insert(class.name.clone(), ());
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
    safe_functions: &HashSet<String>,
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
                    name, args, location.line, const_vars, class_info, safe_functions
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
                    then_branch, const_vars, class_info, unsafe_depth, safe_functions
                );
                errors.extend(branch_errors);

                if let Some(else_stmts) = else_branch {
                    let else_errors = check_statements_for_const_violations(
                        else_stmts, const_vars, class_info, unsafe_depth, safe_functions
                    );
                    errors.extend(else_errors);
                }
            }
            Statement::Block(stmts) => {
                let block_errors = check_statements_for_const_violations(
                    stmts, const_vars, class_info, unsafe_depth, safe_functions
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
    safe_functions: &HashSet<String>,
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
                    name, args, location.line, const_vars, class_info, safe_functions
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
    safe_functions: &HashSet<String>,
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
        // If the callee is marked @safe, trust it - the callee's own file will check
        // that any mutations are inside @unsafe blocks
        if is_callee_safe(func_name, safe_functions) {
            return None;
        }

        // If the callee is not @safe (or unknown), report a violation
        return Some(format!(
            "Const propagation violation at line {}: calling method '{}' \
             through const object '{}'. In @safe code, const propagates through pointer members.",
            line, func_name, const_source
        ));
    }

    None
}

/// Check if a callee function is marked @safe
fn is_callee_safe(func_name: &str, safe_functions: &HashSet<String>) -> bool {
    // Direct match
    if safe_functions.contains(func_name) {
        return true;
    }

    // Try normalized version (without template parameters)
    let normalized = normalize_template_name(func_name);
    if safe_functions.contains(&normalized) {
        return true;
    }

    // Try matching by base class name + method
    if let Some(pos) = func_name.rfind("::") {
        let method_name = &func_name[pos..]; // e.g., "::set"
        for safe_func in safe_functions {
            if safe_func.ends_with(method_name) {
                let func_base = get_base_class_name(func_name);
                let safe_base = get_base_class_name(safe_func);
                if func_base == safe_base {
                    return true;
                }
            }
        }
    }

    false
}

/// Normalize a template name by removing template parameters
/// e.g., "rusty::Cell<int>::set" -> "rusty::Cell::set"
fn normalize_template_name(name: &str) -> String {
    let mut result = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            _ if depth == 0 => result.push(c),
            _ => {}
        }
    }
    result
}

/// Get the base class name without template parameters and method
/// e.g., "rusty::Cell<int>::set" -> "rusty::Cell"
fn get_base_class_name(name: &str) -> Option<String> {
    let normalized = normalize_template_name(name);
    if let Some(pos) = normalized.rfind("::") {
        Some(normalized[..pos].to_string())
    } else {
        None
    }
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
}
