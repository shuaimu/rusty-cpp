use crate::parser::CppAst;
use crate::parser::ast_visitor::Class;
use crate::parser::safety_annotations::SafetyContext;
use crate::parser::external_annotations::ExternalAnnotations;

/// Check for mutable fields in safe functions and classes
///
/// In Rust, the `mutable` keyword is considered unsafe because it breaks
/// the const contract and allows interior mutability without proper guarantees.
/// Instead, users should use `UnsafeCell<T>` and explicitly unsafe code.
pub fn check_mutable_fields(
    ast: &CppAst,
    safety_context: &SafetyContext,
    external_annotations: Option<&ExternalAnnotations>,
) -> Result<Vec<String>, String> {
    use crate::debug_println;

    let mut errors = Vec::new();

    debug_println!("MUTABLE: Checking {} classes for mutable fields", ast.classes.len());

    // Check all classes for mutable fields
    for class in &ast.classes {
        debug_println!("MUTABLE: Checking class '{}' with {} members", class.name, class.members.len());

        // First, check if this class is an unsafe_type (e.g., STL container internal)
        // If so, skip analyzing its internal structure entirely
        if let Some(ext_annot) = external_annotations {
            if ext_annot.is_type_unsafe(&class.name) {
                debug_println!("MUTABLE: Class '{}' is an unsafe_type - skipping internal analysis", class.name);
                continue;
            }
        }

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
///
/// With the two-state model (Safe/Unsafe), mutable field checking is done at the CLASS level:
/// - @safe class → mutable fields are errors
/// - @unsafe class (default for unannotated) → mutable fields are allowed
///
/// Method-level @safe annotations do NOT affect mutable field checking.
/// If you have an @unsafe class with a @safe method, mutable fields are still allowed.
fn is_class_safe(class: &Class, safety_context: &SafetyContext) -> bool {
    use crate::parser::safety_annotations::SafetyMode;
    use crate::debug_println;

    // Get the class's source file location
    let class_file = &class.location.file;

    // Use file-aware safety checking to avoid namespace collisions
    // This ensures that file_default only applies to classes from the source file
    let class_safety = safety_context.get_class_safety_for_file(&class.name, class_file);
    debug_println!("MUTABLE: Class '{}' from '{}' has safety mode: {:?}", class.name, class_file, class_safety);

    // With the two-state model, only check mutable fields for @safe classes
    // Method-level @safe annotations do NOT trigger mutable field checking
    if class_safety == SafetyMode::Safe {
        debug_println!("MUTABLE: Class '{}' is marked @safe - checking for mutable fields", class.name);
        return true;
    }

    debug_println!("MUTABLE: Class '{}' is @unsafe - mutable fields allowed", class.name);
    false
}
