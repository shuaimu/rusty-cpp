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
    // Check if the class itself or any of its methods are marked as safe

    // Check if any method in the class is marked safe
    for method in &class.methods {
        if safety_context.should_check_function(&method.name) {
            return true; // At least one safe method means the class should be checked
        }
    }

    false
}
