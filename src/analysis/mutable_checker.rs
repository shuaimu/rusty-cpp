use crate::parser::CppAst;
use crate::parser::ast_visitor::Class;
use crate::parser::safety_annotations::SafetyContext;

/// Check for mutable fields in safe functions and classes
///
/// In Rust, the `mutable` keyword is considered unsafe because it breaks
/// the const contract and allows interior mutability without proper guarantees.
/// Instead, users should use `UnsafeCell<T>` and explicitly unsafe code.
pub fn check_mutable_fields(
    ast: &CppAst,
    safety_context: &SafetyContext,
) -> Result<Vec<String>, String> {
    use crate::debug_println;

    let mut errors = Vec::new();

    debug_println!("MUTABLE: Checking {} classes for mutable fields", ast.classes.len());

    // Check all classes for mutable fields
    for class in &ast.classes {
        debug_println!("MUTABLE: Checking class '{}' with {} members", class.name, class.members.len());

        // Determine if this class or any of its methods are in safe context
        let class_safe = is_class_safe(class, safety_context);

        debug_println!("MUTABLE: Class '{}' safe = {}", class.name, class_safe);

        if !class_safe {
            continue; // Skip unsafe classes
        }

        // Check all member fields
        for member in &class.members {
            debug_println!("MUTABLE: Checking member '{}' is_mutable = {}", member.name, member.is_mutable);
            if member.is_mutable {
                let error = format!(
                    "{}:{} - Mutable field '{}' not allowed in safe class '{}'. \
                    Use UnsafeCell<T> and unsafe blocks for interior mutability instead.",
                    member.location.file,
                    member.location.line,
                    member.name,
                    class.name
                );
                errors.push(error);
            }
        }
    }

    debug_println!("MUTABLE: Found {} mutable field errors", errors.len());

    Ok(errors)
}

/// Check if a class is marked as safe (either via annotation or file-level safety)
fn is_class_safe(class: &Class, safety_context: &SafetyContext) -> bool {
    use crate::parser::safety_annotations::SafetyMode;
    use crate::debug_println;

    // First check if the class itself has an @unsafe annotation
    // If so, skip checking (allow mutable fields in unsafe classes)
    // This takes HIGHEST priority - even if methods are @safe
    let class_safety = safety_context.get_class_safety(&class.name);
    debug_println!("MUTABLE: Class '{}' has safety mode: {:?}", class.name, class_safety);

    if class_safety == SafetyMode::Unsafe {
        debug_println!("MUTABLE: Class '{}' is explicitly marked @unsafe - skipping mutable field check (ignoring method annotations)", class.name);
        return false; // Unsafe class - skip mutable field checking, even if methods are safe
    }

    // If class is explicitly marked @safe, check it
    if class_safety == SafetyMode::Safe {
        debug_println!("MUTABLE: Class '{}' is marked @safe - checking for mutable fields", class.name);
        return true; // Safe class - check for mutable fields
    }

    // Class has no explicit annotation (Undeclared)
    // Check if any method in the class is marked safe
    let mut has_safe_methods = false;
    let mut has_any_methods = false;
    for method in &class.methods {
        has_any_methods = true;
        if safety_context.should_check_function(&method.name) {
            debug_println!("MUTABLE: Class '{}' has safe method '{}' - will check for mutable fields",
                class.name, method.name);
            has_safe_methods = true;
            break;
        }
    }

    if has_safe_methods {
        debug_println!("MUTABLE: Class '{}' has safe methods - checking for mutable fields", class.name);
        return true;
    }

    // If class has methods but none are safe (all are unsafe or undeclared), skip checking
    // This handles the case where a class is effectively @unsafe via all its methods being @unsafe
    if has_any_methods {
        debug_println!("MUTABLE: Class '{}' has no safe methods (all methods are unsafe/undeclared) - skipping mutable field check", class.name);
    } else {
        debug_println!("MUTABLE: Class '{}' is undeclared with no methods - skipping mutable field check", class.name);
    }
    false
}
