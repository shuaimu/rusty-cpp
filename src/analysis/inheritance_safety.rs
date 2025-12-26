//! Inheritance Safety Analysis
//!
//! This module implements Rust-inspired safety checks for C++ inheritance.
//!
//! Core principle: Inheritance is @unsafe by default, except when inheriting from @interface.
//!
//! An @interface is a pure virtual class (like a Rust trait):
//! - All methods are pure virtual (= 0)
//! - No non-static data members
//! - Virtual destructor required
//! - Can only inherit from other @interfaces
//!
//! Interface methods can be marked @safe or @unsafe. Implementations must:
//! 1. Match the safety annotation (if explicitly annotated)
//! 2. Inherit the safety annotation (if not explicitly annotated)
//! 3. Be validated for safety if marked @safe

use std::collections::{HashMap, HashSet};
use crate::parser::ast_visitor::Class;
use crate::parser::safety_annotations::SafetyMode;
use crate::debug_println;

/// Validate that a class marked as @interface is truly a pure interface
pub fn validate_interface(class: &Class) -> Vec<String> {
    let mut errors = Vec::new();

    if !class.is_interface {
        return errors;
    }

    debug_println!("INHERITANCE: Validating @interface '{}'", class.name);

    // Check 1: No data members (excluding static members)
    let non_static_members: Vec<_> = class.members.iter()
        .filter(|m| !m.is_static)
        .collect();

    if !non_static_members.is_empty() {
        let member_names: Vec<_> = non_static_members.iter()
            .map(|m| m.name.as_str())
            .collect();
        errors.push(format!(
            "@interface '{}' cannot have data members: {:?}",
            class.name, member_names
        ));
    }

    // Check 2: All methods must be pure virtual
    if !class.all_methods_pure_virtual {
        errors.push(format!(
            "@interface '{}' must have all pure virtual methods (= 0)",
            class.name
        ));
    }

    // Check 3: Must have virtual destructor (or no destructor at all for implicit virtual)
    // If class has a destructor, it must be virtual
    if class.has_destructor && !class.has_virtual_destructor {
        errors.push(format!(
            "@interface '{}' must have a virtual destructor",
            class.name
        ));
    }

    // Check 4: No non-virtual methods (excluding destructor)
    if class.has_non_virtual_methods {
        errors.push(format!(
            "@interface '{}' cannot have non-virtual methods",
            class.name
        ));
    }

    errors
}

/// Check that @interface classes only inherit from other @interfaces
pub fn validate_interface_inheritance(
    class: &Class,
    interfaces: &HashSet<String>,
) -> Vec<String> {
    let mut errors = Vec::new();

    if !class.is_interface {
        return errors;
    }

    for base in &class.base_classes {
        // Strip any template parameters for lookup
        let base_name = strip_template_params(base);

        if !interfaces.contains(&base_name) && !interfaces.contains(base) {
            errors.push(format!(
                "@interface '{}' can only inherit from other @interface classes, not '{}'",
                class.name, base
            ));
        }
    }

    errors
}

/// Check that classes in @safe context only inherit from @interface classes
pub fn check_safe_inheritance(
    class: &Class,
    interfaces: &HashSet<String>,
    class_safety: SafetyMode,
) -> Vec<String> {
    let mut errors = Vec::new();

    // Skip if class is in @unsafe context or has no safety annotation
    if class_safety != SafetyMode::Safe {
        return errors;
    }

    // Skip if class has no base classes
    if class.base_classes.is_empty() {
        return errors;
    }

    debug_println!("INHERITANCE: Checking safe inheritance for class '{}'", class.name);

    for base in &class.base_classes {
        // Strip template parameters for lookup
        let base_name = strip_template_params(base);

        if !interfaces.contains(&base_name) && !interfaces.contains(base) {
            errors.push(format!(
                "In @safe code, class '{}' can only inherit from @interface classes. \
                 '{}' is not an @interface. Use @unsafe context for regular inheritance.",
                class.name, base
            ));
        }
    }

    errors
}

/// Check that method implementations honor interface method safety contracts
///
/// This handles both:
/// 1. Explicit annotations - must match the interface
/// 2. Implicit inheritance - inherits from interface
pub fn check_method_safety_contracts(
    class: &Class,
    interfaces: &HashMap<String, Class>,
) -> Vec<String> {
    let mut errors = Vec::new();

    for base_name in &class.base_classes {
        let base_stripped = strip_template_params(base_name);

        let interface = match interfaces.get(&base_stripped).or_else(|| interfaces.get(base_name)) {
            Some(i) => i,
            None => continue, // Not an interface, skip
        };

        debug_println!("INHERITANCE: Checking method safety contracts for '{}' implementing '{}'",
            class.name, base_name);

        // For each method in the interface, find the implementation and check safety
        for interface_method in &interface.methods {
            // Skip destructors and constructors
            if interface_method.name.starts_with('~') ||
               interface_method.name == interface.name {
                continue;
            }

            // Find the implementation in the derived class
            let impl_method = class.methods.iter()
                .find(|m| m.name == interface_method.name);

            let Some(_impl_method) = impl_method else { continue };

            // TODO: Check if implementation has explicit safety annotation
            // TODO: If explicit, verify it matches interface
            // TODO: If implicit, inherit from interface
            // TODO: If @safe (explicit or inherited), validate the method body

            // For now, we just log that we found the implementation
            debug_println!("INHERITANCE: Found implementation of '{}' in '{}'",
                interface_method.name, class.name);
        }
    }

    errors
}

/// Build a set of interface class names from the parsed classes
pub fn collect_interfaces(classes: &[Class]) -> HashSet<String> {
    classes.iter()
        .filter(|c| c.is_interface)
        .map(|c| c.name.clone())
        .collect()
}

/// Build a map of interface classes for method safety checking
pub fn collect_interface_map(classes: &[Class]) -> HashMap<String, Class> {
    classes.iter()
        .filter(|c| c.is_interface)
        .map(|c| (c.name.clone(), c.clone()))
        .collect()
}

/// Run all inheritance safety checks
pub fn check_inheritance_safety(classes: &[Class]) -> Vec<String> {
    let mut errors = Vec::new();

    // Step 1: Collect all interfaces
    let interfaces = collect_interfaces(classes);
    let interface_map = collect_interface_map(classes);

    debug_println!("INHERITANCE: Found {} @interface classes: {:?}",
        interfaces.len(), interfaces);

    // Step 2: Validate all @interface annotations
    for class in classes {
        errors.extend(validate_interface(class));
    }

    // Step 3: Check that @interfaces only inherit from other @interfaces
    for class in classes {
        errors.extend(validate_interface_inheritance(class, &interfaces));
    }

    // Step 4: Check inheritance safety in @safe classes
    for class in classes {
        // Determine class safety - use class annotation or default to Unsafe
        let class_safety = class.safety_annotation.unwrap_or(SafetyMode::Unsafe);
        errors.extend(check_safe_inheritance(class, &interfaces, class_safety));
    }

    // Step 5: Check method safety contracts
    for class in classes {
        errors.extend(check_method_safety_contracts(class, &interface_map));
    }

    errors
}

/// Strip template parameters from a type name
/// e.g., "IContainer<int>" -> "IContainer"
fn strip_template_params(type_name: &str) -> String {
    if let Some(pos) = type_name.find('<') {
        type_name[..pos].to_string()
    } else {
        type_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_visitor::{Class, SourceLocation};

    fn make_location() -> SourceLocation {
        SourceLocation {
            file: "test.cpp".to_string(),
            line: 1,
            column: 1,
        }
    }

    fn make_interface(name: &str) -> Class {
        Class {
            name: name.to_string(),
            template_parameters: Vec::new(),
            is_template: false,
            members: Vec::new(),
            methods: Vec::new(),
            base_classes: Vec::new(),
            location: make_location(),
            has_destructor: true,
            is_interface: true,
            has_virtual_destructor: true,
            all_methods_pure_virtual: true,
            has_non_virtual_methods: false,
            safety_annotation: None,
        }
    }

    fn make_class(name: &str, base_classes: Vec<String>) -> Class {
        Class {
            name: name.to_string(),
            template_parameters: Vec::new(),
            is_template: false,
            members: Vec::new(),
            methods: Vec::new(),
            base_classes,
            location: make_location(),
            has_destructor: false,
            is_interface: false,
            has_virtual_destructor: false,
            all_methods_pure_virtual: false,
            has_non_virtual_methods: false,
            safety_annotation: None,
        }
    }

    #[test]
    fn test_valid_interface() {
        let interface = make_interface("IDrawable");
        let errors = validate_interface(&interface);
        assert!(errors.is_empty(), "Valid interface should have no errors");
    }

    #[test]
    fn test_interface_with_data_member() {
        let mut interface = make_interface("IBadInterface");
        interface.members.push(crate::parser::ast_visitor::Variable {
            name: "data".to_string(),
            type_name: "int".to_string(),
            is_reference: false,
            is_pointer: false,
            is_const: false,
            is_unique_ptr: false,
            is_shared_ptr: false,
            is_static: false,
            is_mutable: false,
            location: make_location(),
            is_pack: false,
            pack_element_type: None,
        });

        let errors = validate_interface(&interface);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("cannot have data members"));
    }

    #[test]
    fn test_interface_without_virtual_destructor() {
        let mut interface = make_interface("IBadInterface");
        interface.has_virtual_destructor = false;

        let errors = validate_interface(&interface);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("virtual destructor"));
    }

    #[test]
    fn test_safe_inheritance_from_interface() {
        let interface = make_interface("IDrawable");
        let mut derived = make_class("Circle", vec!["IDrawable".to_string()]);
        derived.safety_annotation = Some(SafetyMode::Safe);

        let interfaces: HashSet<String> = vec!["IDrawable".to_string()].into_iter().collect();

        let errors = check_safe_inheritance(&derived, &interfaces, SafetyMode::Safe);
        assert!(errors.is_empty(), "Safe inheritance from interface should be allowed");
    }

    #[test]
    fn test_safe_inheritance_from_non_interface() {
        let base = make_class("Base", Vec::new());
        let mut derived = make_class("Derived", vec!["Base".to_string()]);
        derived.safety_annotation = Some(SafetyMode::Safe);

        let interfaces: HashSet<String> = HashSet::new();

        let errors = check_safe_inheritance(&derived, &interfaces, SafetyMode::Safe);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("can only inherit from @interface"));
    }
}
