//! Struct Pointer Member Safety Analysis
//!
//! This module checks that pointer members in @safe structs/classes are properly
//! initialized to non-null values.
//!
//! Rules:
//! 1. In @safe structs, pointer members must have initialization
//! 2. Pointer members cannot be initialized to nullptr/NULL/0
//! 3. Constructor initializer lists cannot use nullptr for pointer members
//! 4. Default member initializers cannot use nullptr
//!
//! This ensures pointers behave like Rust references - always valid.

use crate::parser::ast_visitor::Class;
use crate::parser::safety_annotations::SafetyMode;

/// Check all classes for pointer member safety
pub fn check_struct_pointer_safety(classes: &[Class]) -> Vec<String> {
    let mut errors = Vec::new();

    for class in classes {
        // Only check @safe classes
        let class_safety = class.safety_annotation.unwrap_or(SafetyMode::Unsafe);
        if class_safety != SafetyMode::Safe {
            continue;
        }

        // Check each member for pointer issues
        for member in &class.members {
            // Skip non-pointer members
            if !member.is_pointer {
                continue;
            }

            // Skip smart pointers - they handle null safely
            if is_smart_pointer_type(&member.type_name) {
                continue;
            }

            // Check if the type_name contains an initializer with nullptr
            // This handles default member initializers like `int* ptr = nullptr;`
            if has_nullptr_initializer(&member.type_name) {
                errors.push(format!(
                    "In @safe struct '{}': Pointer member '{}' cannot be initialized to nullptr. \
                     Use a valid pointer or wrap in Option<T*>.",
                    class.name, member.name
                ));
                continue;
            }

            // For pointer members without default initializers in @safe structs,
            // they must be initialized in ALL constructors.
            // If there are no constructors, the pointer would be uninitialized.
            if !has_default_member_initializer(&member.type_name) {
                // Check if all constructors initialize this member
                let all_ctors_init = check_constructors_initialize_member(class, &member.name);

                if !all_ctors_init {
                    errors.push(format!(
                        "In @safe struct '{}': Pointer member '{}' must be initialized to a non-null value. \
                         Add a default member initializer or ensure all constructors initialize it.",
                        class.name, member.name
                    ));
                }
            }
        }

        // Check constructor bodies and initializer lists for nullptr assignments
        for method in &class.methods {
            if is_constructor(&method.name, &class.name) {
                let ctor_errors = check_constructor_for_nullptr(method, class);
                errors.extend(ctor_errors);
            }
        }
    }

    errors
}

/// Check if a type name is a smart pointer type
fn is_smart_pointer_type(type_name: &str) -> bool {
    type_name.contains("unique_ptr") ||
    type_name.contains("shared_ptr") ||
    type_name.contains("weak_ptr") ||
    type_name.contains("Box<") ||
    type_name.contains("Arc<") ||
    type_name.contains("Rc<")
}

/// Check if the type string contains a nullptr initializer
/// e.g., "int* = nullptr" or "char* ptr = nullptr"
fn has_nullptr_initializer(type_name: &str) -> bool {
    // The parser may include the initializer in the type name for default member initializers
    type_name.contains("= nullptr") ||
    type_name.contains("= NULL") ||
    type_name.contains("= 0") ||
    type_name.contains("=nullptr") ||
    type_name.contains("=NULL") ||
    type_name.contains("=0")
}

/// Check if a member has a default member initializer
fn has_default_member_initializer(type_name: &str) -> bool {
    type_name.contains('=')
}

/// Check if a method is a constructor for the given class
/// Handles both qualified names (Container::Container) and unqualified names (Container)
fn is_constructor(method_name: &str, class_name: &str) -> bool {
    // Direct match
    if method_name == class_name {
        return true;
    }
    // Qualified match: "ClassName::ClassName"
    let qualified = format!("{}::{}", class_name, class_name);
    if method_name == qualified {
        return true;
    }
    // Check if method_name ends with "::ClassName" (e.g., "Namespace::ClassName::ClassName")
    if method_name.ends_with(&format!("::{}", class_name)) {
        // Extract the part before the last "::"
        if let Some(base) = method_name.rsplit_once("::") {
            // Check if the base also ends with the class name
            if base.0.ends_with(class_name) || base.0 == class_name {
                return true;
            }
        }
    }
    false
}

/// Check if all constructors initialize a given member
/// Returns true if:
/// - Default constructor is deleted or doesn't exist (no implicit default)
/// - AND all existing non-deleted constructors initialize the member to non-null
fn check_constructors_initialize_member(class: &Class, member_name: &str) -> bool {
    // Get all user-defined constructors (not deleted)
    let constructors: Vec<_> = class.methods.iter()
        .filter(|m| is_constructor(&m.name, &class.name) && !m.is_deleted)
        .collect();

    // Check if default constructor exists and could leave pointer uninitialized
    // If has_default_constructor is true but NOT deleted, there's a way to create
    // an uninitialized instance (unless all ctors init properly)
    if class.has_default_constructor && !class.default_constructor_deleted {
        // Check if the default constructor (if user-defined) initializes this member
        let default_ctor = constructors.iter()
            .find(|m| m.parameters.is_empty());

        if let Some(ctor) = default_ctor {
            // User-defined default constructor - check it initializes the member
            if !constructor_initializes_member_with_init_list(ctor, member_name) {
                return false;
            }
        } else if !class.has_user_defined_constructor {
            // Implicit default constructor - won't initialize pointer members
            return false;
        }
    }

    // If there are no constructors at all and no implicit default, that's an error case
    // (handled by C++ compiler, but we shouldn't allow it either)
    if constructors.is_empty() && !class.has_default_constructor {
        // No way to construct this class - C++ would error, so this is fine
        return true;
    }

    // If there are no user-defined constructors but default is deleted,
    // that means no way to create an instance (OK - can't have uninitialized pointer)
    if constructors.is_empty() && class.default_constructor_deleted {
        return true;
    }

    // Check each non-deleted constructor initializes this member
    for ctor in &constructors {
        // Skip deleted constructors - they can't be called
        if ctor.is_deleted {
            continue;
        }

        // Check if the constructor's safety allows skipping
        let ctor_safety = ctor.safety_annotation.unwrap_or(SafetyMode::Unsafe);
        if ctor_safety != SafetyMode::Safe {
            // Unsafe constructors can do whatever they want
            // But we still need to ensure pointer is initialized somehow
            // For safety, check if it initializes the member
        }

        // Check if constructor initializes this member (via init list or body)
        if !constructor_initializes_member_with_init_list(ctor, member_name) {
            return false;
        }
    }

    true
}

/// Check if a constructor initializes a specific member (checking init list first, then body)
fn constructor_initializes_member_with_init_list(ctor: &crate::parser::Function, member_name: &str) -> bool {
    // First check member initializer list (`: ptr(&value)`)
    for init in &ctor.member_initializers {
        if init.member_name == member_name {
            // Found in init list - check if it's nullptr
            // If nullptr, it's "initialized" but to null (will be caught by other check)
            // Return true because it IS initialized (just maybe to wrong value)
            return true;
        }
    }

    // If not in init list, check constructor body
    constructor_initializes_member_in_body(ctor, member_name)
}

/// Check if a constructor initializes a specific member in its body
fn constructor_initializes_member_in_body(ctor: &crate::parser::Function, member_name: &str) -> bool {
    use crate::parser::Statement;

    // Check statements in constructor body
    for stmt in &ctor.body {
        match stmt {
            Statement::Assignment { lhs, rhs, .. } => {
                // Check if assigning to this member
                if let Some(assigned_member) = extract_member_name(lhs) {
                    if assigned_member == member_name {
                        // Check if assigning nullptr
                        if is_nullptr_expression(rhs) {
                            // Initialized to nullptr - will be caught by other check
                            return true; // Technically initialized, just to nullptr
                        }
                        return true;
                    }
                }
            }
            _ => {}
        }
    }

    false
}

/// Extract member name from an expression like `ptr` or `this->ptr`
fn extract_member_name(expr: &crate::parser::Expression) -> Option<String> {
    use crate::parser::Expression;

    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::MemberAccess { object, field } => {
            // Check if accessing via 'this'
            if let Expression::Variable(obj_name) = object.as_ref() {
                if obj_name == "this" {
                    return Some(field.clone());
                }
            }
            Some(field.clone())
        }
        _ => None,
    }
}

/// Check if an expression is nullptr/NULL/0
fn is_nullptr_expression(expr: &crate::parser::Expression) -> bool {
    use crate::parser::Expression;

    match expr {
        Expression::Nullptr => true,
        Expression::Literal(lit) => {
            lit == "nullptr" || lit == "NULL" || lit == "0"
        }
        Expression::Variable(name) => {
            name == "nullptr" || name == "NULL"
        }
        _ => false,
    }
}

/// Check a constructor for nullptr assignments to pointer members
/// Checks both member initializer lists and constructor body
fn check_constructor_for_nullptr(
    ctor: &crate::parser::Function,
    class: &Class,
) -> Vec<String> {
    use crate::parser::Statement;

    let mut errors = Vec::new();

    // Only check @safe constructors
    let ctor_safety = ctor.safety_annotation.unwrap_or(SafetyMode::Unsafe);
    if ctor_safety != SafetyMode::Safe {
        return errors;
    }

    // Get set of pointer member names
    let pointer_members: std::collections::HashSet<_> = class.members.iter()
        .filter(|m| m.is_pointer && !is_smart_pointer_type(&m.type_name))
        .map(|m| m.name.clone())
        .collect();

    // Check member initializer list for nullptr
    for init in &ctor.member_initializers {
        if pointer_members.contains(&init.member_name) && init.is_nullptr {
            errors.push(format!(
                "In @safe constructor '{}::{}': \
                 Cannot initialize pointer member '{}' to nullptr in initializer list. \
                 Use a valid pointer or wrap in Option<T*>.",
                class.name, ctor.name, init.member_name
            ));
        }
    }

    // Check for nullptr assignments in constructor body
    let mut unsafe_depth = 0;
    for stmt in &ctor.body {
        match stmt {
            Statement::EnterUnsafe => {
                unsafe_depth += 1;
            }
            Statement::ExitUnsafe => {
                if unsafe_depth > 0 {
                    unsafe_depth -= 1;
                }
            }
            Statement::Assignment { lhs, rhs, location } if unsafe_depth == 0 => {
                // Check if assigning nullptr to a pointer member
                if let Some(member_name) = extract_member_name(lhs) {
                    if pointer_members.contains(&member_name) && is_nullptr_expression(rhs) {
                        errors.push(format!(
                            "In @safe constructor '{}::{}' at line {}: \
                             Cannot assign nullptr to pointer member '{}'. \
                             Use a valid pointer or wrap in Option<T*>.",
                            class.name, ctor.name, location.line, member_name
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_smart_pointer_type() {
        assert!(is_smart_pointer_type("std::unique_ptr<int>"));
        assert!(is_smart_pointer_type("std::shared_ptr<Foo>"));
        assert!(is_smart_pointer_type("Box<int>"));
        assert!(!is_smart_pointer_type("int*"));
        assert!(!is_smart_pointer_type("char*"));
    }

    #[test]
    fn test_has_nullptr_initializer() {
        assert!(has_nullptr_initializer("int* = nullptr"));
        assert!(has_nullptr_initializer("char* ptr = NULL"));
        assert!(has_nullptr_initializer("void* = 0"));
        assert!(!has_nullptr_initializer("int*"));
        assert!(!has_nullptr_initializer("int* = &x"));
    }

    #[test]
    fn test_is_nullptr_expression() {
        use crate::parser::Expression;

        assert!(is_nullptr_expression(&Expression::Nullptr));
        assert!(is_nullptr_expression(&Expression::Literal("nullptr".to_string())));
        assert!(is_nullptr_expression(&Expression::Literal("NULL".to_string())));
        assert!(is_nullptr_expression(&Expression::Literal("0".to_string())));
        assert!(!is_nullptr_expression(&Expression::Literal("42".to_string())));
        assert!(!is_nullptr_expression(&Expression::Variable("ptr".to_string())));
    }
}
