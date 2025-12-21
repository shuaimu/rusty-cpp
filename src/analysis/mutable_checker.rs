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
/// IMPORTANT: This function now uses file-aware safety checking to avoid the namespace
/// collision bug. The file_default (from @safe namespace) only applies to classes
/// from the source file being analyzed. Classes from other files (system headers,
/// external libraries) are treated as Undeclared unless explicitly annotated.
fn is_class_safe(class: &Class, safety_context: &SafetyContext) -> bool {
    use crate::parser::safety_annotations::SafetyMode;
    use crate::debug_println;

    // Get the class's source file location
    let class_file = &class.location.file;

    // Use file-aware safety checking to avoid namespace collisions
    // This ensures that file_default only applies to classes from the source file
    let class_safety = safety_context.get_class_safety_for_file(&class.name, class_file);
    debug_println!("MUTABLE: Class '{}' from '{}' has safety mode: {:?}", class.name, class_file, class_safety);

    if class_safety == SafetyMode::Unsafe {
        debug_println!("MUTABLE: Class '{}' is explicitly marked @unsafe - skipping mutable field check", class.name);
        return false;
    }

    if class_safety == SafetyMode::Safe {
        debug_println!("MUTABLE: Class '{}' is marked @safe - checking for mutable fields", class.name);
        return true;
    }

    // Class has no explicit annotation (Undeclared)
    // Check if any method in the class is marked safe AND is from the source file
    // IMPORTANT: Pre-annotated STL methods (like std::fpos::operator=) shouldn't
    // trigger mutable field checking on their containing classes. Only consider
    // methods that are actually from the source file being analyzed.
    let mut has_safe_methods = false;
    let mut has_any_methods = false;
    for method in &class.methods {
        has_any_methods = true;
        let method_file = &method.location.file;

        // Only consider methods from the source file for mutable field checking
        // This prevents pre-annotated STL methods from triggering checks on STL classes
        if !safety_context.is_from_source_file(method_file) {
            continue;
        }

        if safety_context.should_check_function_for_file(&method.name, method_file) {
            debug_println!("MUTABLE: Class '{}' has safe method '{}' from source file - will check for mutable fields",
                class.name, method.name);
            has_safe_methods = true;
            break;
        }
    }

    if has_safe_methods {
        debug_println!("MUTABLE: Class '{}' has safe methods - checking for mutable fields", class.name);
        return true;
    }

    if has_any_methods {
        debug_println!("MUTABLE: Class '{}' has no safe methods - skipping mutable field check", class.name);
    } else {
        debug_println!("MUTABLE: Class '{}' is undeclared with no methods - skipping mutable field check", class.name);
    }
    false
}
