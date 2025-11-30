use crate::ir::{IrProgram, IrFunction, OwnershipState, BorrowKind};
use crate::parser::HeaderCache;
use std::collections::{HashMap, HashSet};
use crate::debug_println;

/// Check if a file path is from a system header (not user code)
/// System headers are from standard library or third-party installations
fn is_system_header(file_path: &str) -> bool {
    // Common system header paths (absolute)
    let system_paths = [
        "/usr/include",
        "/usr/local/include",
        "/opt/homebrew/include",
        "/Library/Developer",
        "C:\\Program Files",
        "/Applications/Xcode.app",
    ];

    for path in &system_paths {
        if file_path.starts_with(path) {
            return true;
        }
    }

    // STL and system library patterns (works for relative paths too)
    if file_path.contains("/include/c++/") ||
       file_path.contains("/bits/") ||
       file_path.contains("/ext/") ||
       file_path.contains("stl_") ||
       file_path.contains("/lib/gcc/") {
        return true;
    }

    // Also skip the project's include/ directory (third-party headers like rusty::Box)
    // These are library headers that shouldn't be analyzed internally
    if file_path.contains("/include/rusty/") || file_path.contains("/include/unified_") {
        return true;
    }

    false
}

/// Check if a type name represents a primitive type that can't contain references
fn is_primitive_type(type_name: &str) -> bool {
    // Strip template parameters and qualifiers
    let base_type = type_name
        .split('<').next().unwrap_or(type_name)
        .trim();

    matches!(base_type,
        "int" | "char" | "bool" | "float" | "double" |
        "long" | "short" | "unsigned" | "signed" |
        "int8_t" | "int16_t" | "int32_t" | "int64_t" |
        "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" |
        "size_t" | "ptrdiff_t" | "void"
    )
}

pub mod ownership;
pub mod borrows;
pub mod lifetimes;
pub mod lifetime_checker;
pub mod scope_lifetime;
pub mod lifetime_inference;
pub mod pointer_safety;
pub mod unsafe_propagation;
pub mod this_tracking;
pub mod liveness;
pub mod mutable_checker;
pub mod lambda_capture_safety;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BorrowCheckError {
    pub kind: ErrorKind,
    pub location: String,
    pub message: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ErrorKind {
    UseAfterMove,
    DoubleBorrow,
    MutableBorrowWhileImmutable,
    DanglingReference,
    LifetimeViolation,
}

#[allow(dead_code)]
pub fn check_borrows(program: IrProgram) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    
    for function in &program.functions {
        let function_errors = check_function(function)?;
        errors.extend(function_errors);
    }
    
    Ok(errors)
}

#[allow(dead_code)]
pub fn check_borrows_with_annotations_and_safety(
    program: IrProgram, 
    header_cache: HeaderCache,
    file_safe: bool
) -> Result<Vec<String>, String> {
    // If file is marked unsafe and no functions are marked safe, skip checking
    if !file_safe && !has_any_safe_functions(&program, &header_cache) {
        return Ok(Vec::new()); // No checking for unsafe code
    }
    
    check_borrows_with_annotations(program, header_cache)
}

pub fn check_borrows_with_safety_context(
    program: IrProgram,
    header_cache: HeaderCache,
    safety_context: crate::parser::safety_annotations::SafetyContext
) -> Result<Vec<String>, String> {
    use crate::parser::safety_annotations::SafetyMode;

    // If the file default is unsafe and no functions are marked safe, skip checking
    if safety_context.file_default != SafetyMode::Safe &&
       !safety_context.function_overrides.iter().any(|(_, mode)| *mode == SafetyMode::Safe) &&
       !has_any_safe_functions(&program, &header_cache) {
        return Ok(Vec::new()); // No checking for unsafe code
    }

    let mut errors = Vec::new();

    // PHASE 1: Check that safe functions returning references have lifetime annotations
    let annotation_errors = check_lifetime_annotation_requirements(&program, &header_cache, &safety_context)?;
    errors.extend(annotation_errors);

    // Check each function based on its safety mode
    for function in &program.functions {
        debug_println!("DEBUG: Checking function '{}'", function.name);

        // Skip borrow checking for system header functions
        // They are tracked for safety status but not analyzed internally
        if is_system_header(&function.source_file) {
            debug_println!("DEBUG: Skipping system header function '{}' from {}", function.name, function.source_file);
            continue;
        }

        // Check if this function should be checked
        if !safety_context.should_check_function(&function.name) {
            debug_println!("DEBUG: Skipping unsafe function '{}'", function.name);
            continue; // Skip unsafe functions
        }
        debug_println!("DEBUG: Function '{}' is safe, checking...", function.name);

        // Phase 2: Use version with header_cache for return value borrow detection
        let function_errors = check_function_with_header_cache(function, &header_cache)?;
        errors.extend(function_errors);
    }

    // Run lifetime inference and validation for safe functions
    for function in &program.functions {
        // Skip system headers
        if is_system_header(&function.source_file) {
            continue;
        }

        if safety_context.should_check_function(&function.name) {
            let inference_errors = lifetime_inference::infer_and_validate_lifetimes(function)?;
            errors.extend(inference_errors);
        }
    }
    
    // If we have header annotations, also check lifetime constraints
    if header_cache.has_signatures() {
        let lifetime_errors = lifetime_checker::check_lifetimes_with_annotations(&program, &header_cache)?;
        errors.extend(lifetime_errors);
        
        // Also run scope-based lifetime checking
        let scope_errors = scope_lifetime::check_scoped_lifetimes(&program, &header_cache)?;
        errors.extend(scope_errors);
    }
    
    Ok(errors)
}

fn has_any_safe_functions(program: &IrProgram, header_cache: &HeaderCache) -> bool {
    use crate::parser::annotations::SafetyAnnotation;

    for function in &program.functions {
        if let Some(sig) = header_cache.get_signature(&function.name) {
            if let Some(SafetyAnnotation::Safe) = sig.safety {
                return true;
            }
        }
    }
    false
}

/// Check if a return type string represents a reference
fn returns_reference(return_type: &str) -> bool {
    // Check for reference types: &, const &, const Type&, Type&, etc.
    // This is a simple heuristic based on the string representation
    return_type.contains('&') && !return_type.contains("&&") // Exclude rvalue references for now
}

/// Phase 1: Check that safe functions returning references have lifetime annotations
fn check_lifetime_annotation_requirements(
    program: &IrProgram,
    header_cache: &HeaderCache,
    safety_context: &crate::parser::safety_annotations::SafetyContext
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();

    for function in &program.functions {
        // Skip system header functions
        if is_system_header(&function.source_file) {
            continue;
        }

        // Only check safe functions
        if !safety_context.should_check_function(&function.name) {
            continue;
        }

        // Get the function's return type from the original AST
        // We need to access this through the parser/AST, but for now we can check
        // if the function has a Return statement with a reference

        // Check if function signature has lifetime annotation
        let has_lifetime_annotation = if let Some(sig) = header_cache.get_signature(&function.name) {
            sig.return_lifetime.is_some()
        } else {
            false
        };

        // Check if the function returns a reference by analyzing return statements
        let returns_ref = check_if_function_returns_reference(function);

        if returns_ref && !has_lifetime_annotation {
            errors.push(format!(
                "Safe function '{}' returns a reference but has no @lifetime annotation",
                function.name
            ));
        }
    }

    Ok(errors)
}

/// Check if a function returns a reference by analyzing its return type
fn check_if_function_returns_reference(function: &IrFunction) -> bool {
    // Check if the return type is a reference
    // References have & in the type (e.g., "const int&", "int&", "Type&")
    function.return_type.contains('&') && !function.return_type.contains("&&")
}

#[allow(dead_code)]
pub fn check_borrows_with_annotations(program: IrProgram, header_cache: HeaderCache) -> Result<Vec<String>, String> {
    use crate::parser::annotations::SafetyAnnotation;
    let mut errors = Vec::new();
    
    // Run regular borrow checking, but skip unsafe functions
    for function in &program.functions {
        // Check if this function is marked as unsafe
        let is_unsafe = if let Some(sig) = header_cache.get_signature(&function.name) {
            matches!(sig.safety, Some(SafetyAnnotation::Unsafe))
        } else {
            false
        };
        
        // Skip checking if function is marked unsafe
        if !is_unsafe {
            let function_errors = check_function(function)?;
            errors.extend(function_errors);
        }
    }
    
    // Run lifetime inference and validation
    for function in &program.functions {
        let inference_errors = lifetime_inference::infer_and_validate_lifetimes(function)?;
        errors.extend(inference_errors);
    }
    
    // If we have header annotations, also check lifetime constraints
    if header_cache.has_signatures() {
        let lifetime_errors = lifetime_checker::check_lifetimes_with_annotations(&program, &header_cache)?;
        errors.extend(lifetime_errors);
        
        // Also run scope-based lifetime checking
        let scope_errors = scope_lifetime::check_scoped_lifetimes(&program, &header_cache)?;
        errors.extend(scope_errors);
    }
    
    Ok(errors)
}

// Phase 2: Wrapper for backward compatibility
fn check_function(function: &IrFunction) -> Result<Vec<String>, String> {
    // Create an empty HeaderCache for functions that don't have annotations
    let empty_cache = HeaderCache::new();
    check_function_with_header_cache(function, &empty_cache)
}

// Phase 2: Added header_cache parameter for return value borrow detection
fn check_function_with_header_cache(function: &IrFunction, header_cache: &HeaderCache) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();

    // NEW: Run liveness analysis first
    let mut liveness_analyzer = liveness::LivenessAnalyzer::new();
    let last_uses = liveness_analyzer.analyze(function);

    // Create ownership tracker with liveness information
    let mut ownership_tracker = OwnershipTracker::with_liveness(last_uses);

    // Create this pointer tracker if this is a method
    let mut this_tracker = if function.is_method {
        Some(this_tracking::ThisPointerTracker::new(function.method_qualifier.clone()))
    } else {
        None
    };

    // Initialize ownership for parameters and variables
    for (name, var_info) in &function.variables {
        ownership_tracker.set_ownership(name.clone(), var_info.ownership.clone());

        // Track reference types
        match &var_info.ty {
            crate::ir::VariableType::Reference(_) => {
                ownership_tracker.mark_as_reference(name.clone(), false);
            }
            crate::ir::VariableType::MutableReference(_) => {
                ownership_tracker.mark_as_reference(name.clone(), true);
            }
            _ => {}
        }
    }
    
    // Traverse CFG and check each block
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];
        
        // Process statements, handling loops specially
        let mut i = 0;
        while i < block.statements.len() {
            let statement = &block.statements[i];
            
            // Check if we're entering a loop
            if matches!(statement, crate::ir::IrStatement::EnterLoop) {
                // Find the matching ExitLoop
                let mut loop_end = i + 1;
                let mut loop_depth = 1;
                while loop_end < block.statements.len() && loop_depth > 0 {
                    match &block.statements[loop_end] {
                        crate::ir::IrStatement::EnterLoop => loop_depth += 1,
                        crate::ir::IrStatement::ExitLoop => loop_depth -= 1,
                        _ => {}
                    }
                    loop_end += 1;
                }
                
                // Process the loop body twice to simulate 2 iterations
                let loop_body = &block.statements[i+1..loop_end-1];
                
                // First iteration
                ownership_tracker.enter_loop();
                
                // Track variables declared in the loop
                let mut loop_local_vars = HashSet::new();
                
                for (loop_idx, loop_stmt) in loop_body.iter().enumerate() {
                    // Track variable declarations in the loop
                    if let crate::ir::IrStatement::Borrow { to, .. } = loop_stmt {
                        loop_local_vars.insert(to.clone());
                    }
                    process_statement(loop_stmt, &mut ownership_tracker, &mut this_tracker, &mut errors, header_cache, function);

                    // NEW: Check for last uses (after processing statement)
                    // Statement index is i+1+loop_idx (i is EnterLoop, +1 for first statement)
                    ownership_tracker.check_and_clear_last_uses(i + 1 + loop_idx);
                }

                // Save state after first iteration (but only for non-loop-local variables)
                let state_after_first = ownership_tracker.ownership.clone();

                // Clear loop-local borrows at end of first iteration
                ownership_tracker.clear_loop_locals(&loop_local_vars);

                // Second iteration - check for use-after-move
                for (loop_idx, loop_stmt) in loop_body.iter().enumerate() {
                    // Before processing each statement in second iteration,
                    // check if it would cause use-after-move (but only for non-loop-local vars)
                    check_statement_for_loop_errors(loop_stmt, &state_after_first, &mut errors);
                    process_statement(loop_stmt, &mut ownership_tracker, &mut this_tracker, &mut errors, header_cache, function);

                    // NEW: Check for last uses (after processing statement)
                    ownership_tracker.check_and_clear_last_uses(i + 1 + loop_idx);
                }
                
                // Clear loop-local borrows at end of second iteration
                ownership_tracker.clear_loop_locals(&loop_local_vars);
                
                ownership_tracker.exit_loop();
                
                // Skip past the loop
                i = loop_end;
            } else {
                // Normal statement processing
                process_statement(statement, &mut ownership_tracker, &mut this_tracker, &mut errors, header_cache, function);

                // NEW: Check for last uses (after processing statement)
                ownership_tracker.check_and_clear_last_uses(i);

                i += 1;
            }
        }
    }
    
    Ok(errors)
}

// Helper function to check for loop-specific errors in second iteration
fn check_statement_for_loop_errors(
    statement: &crate::ir::IrStatement,
    state_after_first: &HashMap<String, OwnershipState>,
    errors: &mut Vec<String>,
) {
    match statement {
        crate::ir::IrStatement::Move { from, .. } => {
            if let Some(state) = state_after_first.get(from) {
                if *state == OwnershipState::Moved {
                    errors.push(format!(
                        "Use after move in loop: variable '{}' was moved in first iteration and used again in second iteration",
                        from
                    ));
                }
            }
        }
        crate::ir::IrStatement::Assign { rhs, .. } => {
            if let crate::ir::IrExpression::Variable(var) = rhs {
                if let Some(state) = state_after_first.get(var) {
                    if *state == OwnershipState::Moved {
                        errors.push(format!(
                            "Use after move in loop: variable '{}' was moved in first iteration and used again in second iteration",
                            var
                        ));
                    }
                }
            }
        }
        _ => {}
    }
}

// Phase 3: Helper function to check for borrow conflicts
fn check_borrow_conflicts(
    from: &str,
    kind: &BorrowKind,
    ownership_tracker: &OwnershipTracker,
    errors: &mut Vec<String>,
) -> bool {
    let current_borrows = ownership_tracker.get_borrows(from);

    match kind {
        BorrowKind::Immutable => {
            // Can have multiple immutable borrows, but not if there's a mutable borrow
            if current_borrows.has_mutable {
                errors.push(format!(
                    "Cannot create immutable reference to '{}': already mutably borrowed",
                    from
                ));
                return false;
            }
        }
        BorrowKind::Mutable => {
            // Can only have one mutable borrow, and no immutable borrows
            if current_borrows.immutable_count > 0 {
                errors.push(format!(
                    "Cannot create mutable reference to '{}': already immutably borrowed",
                    from
                ));
                return false;
            } else if current_borrows.has_mutable {
                errors.push(format!(
                    "Cannot create mutable reference to '{}': already mutably borrowed",
                    from
                ));
                return false;
            }
        }
    }

    true
}

// Extract statement processing logic into a separate function
// Phase 2: Added header_cache and function parameters for return value borrow detection
fn process_statement(
    statement: &crate::ir::IrStatement,
    ownership_tracker: &mut OwnershipTracker,
    this_tracker: &mut Option<this_tracking::ThisPointerTracker>,
    errors: &mut Vec<String>,
    header_cache: &HeaderCache,  // Phase 2: For looking up function signatures
    function: &IrFunction,       // Phase 2: For checking variable types
) {
    match statement {
        crate::ir::IrStatement::Move { from, to } => {
            debug_println!("DEBUG ANALYSIS: Processing Move from '{}' to '{}'", from, to);
            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                // Still update ownership state for consistency
                ownership_tracker.set_ownership(from.clone(), OwnershipState::Moved);
                ownership_tracker.set_ownership(to.clone(), OwnershipState::Owned);
                return;
            }
            
            // Check if 'from' is owned and not moved
            let from_state = ownership_tracker.get_ownership(from);
            debug_println!("DEBUG ANALYSIS: '{}' state: {:?}", from, from_state);
            
            // Can't move from a reference
            if ownership_tracker.is_reference(from) {
                errors.push(format!(
                    "Cannot move out of '{}' because it is behind a reference",
                    from
                ));
                return;
            }

            // Phase 4: Can't move from a variable that is transitively borrowed
            // Check both direct borrows AND transitive borrows (borrow chains)
            if ownership_tracker.is_transitively_borrowed(from) {
                let borrowers = ownership_tracker.get_transitive_borrowers(from);
                errors.push(format!(
                    "Cannot move '{}' because it is borrowed by: {}",
                    from,
                    borrowers.join(", ")
                ));
                return;
            }

            // REASSIGNMENT TRACKING: Check if 'to' has active borrows
            // In Rust, assignment drops the old value first, so we can't assign if borrowed
            // Example: box1 = std::move(box2); drops old value of box1
            if let Some(borrows) = ownership_tracker.get_active_borrows(to) {
                if !borrows.is_empty() {
                    let borrower_names: Vec<String> = borrows.iter().map(|b| b.borrower.clone()).collect();
                    errors.push(format!(
                        "Cannot assign to '{}' because it is borrowed by: {} (assignment would drop the old value)",
                        to,
                        borrower_names.join(", ")
                    ));
                    return;
                }
            }

            if from_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Use after move: variable '{}' has already been moved",
                    from
                ));
            }

            // NEW: Check if the object has any moved fields (partial move)
            if ownership_tracker.has_moved_fields(from) {
                let moved_fields = ownership_tracker.get_moved_fields(from);
                errors.push(format!(
                    "Cannot move '{}' because it has been partially moved (moved fields: {})",
                    from,
                    moved_fields.join(", ")
                ));
                return;
            }

            // Handle temporary move markers (from std::move in function calls)
            if to.starts_with("_temp_move_") || to.starts_with("_moved_") {
                // Just mark the source as moved, don't create the temporary
                ownership_tracker.set_ownership(from.clone(), OwnershipState::Moved);
            } else {
                // Transfer ownership for regular moves
                ownership_tracker.set_ownership(from.clone(), OwnershipState::Moved);
                ownership_tracker.set_ownership(to.clone(), OwnershipState::Owned);
            }
        }
        
        // NEW: Handle field-level operations
        crate::ir::IrStatement::MoveField { object, field, to } => {
            debug_println!("DEBUG ANALYSIS: Processing MoveField from '{}.{}' to '{}'", object, field, to);

            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                ownership_tracker.mark_field_moved(object.clone(), field.clone());
                ownership_tracker.set_ownership(to.clone(), OwnershipState::Owned);
                return;
            }

            // Check if the object itself has been moved
            let object_state = ownership_tracker.get_ownership(object);
            if object_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Cannot move field '{}' from '{}' because '{}' has been moved",
                    field, object, object
                ));
                return;
            }

            // Check if the field has already been moved
            let field_state = ownership_tracker.get_field_ownership(object, field);
            if field_state == OwnershipState::Moved {
                errors.push(format!(
                    "Use after move: field '{}.{}' has already been moved",
                    object, field
                ));
                return;
            }

            // Phase 4: Check if the object is transitively borrowed
            if ownership_tracker.is_transitively_borrowed(object) {
                let borrowers = ownership_tracker.get_transitive_borrowers(object);
                errors.push(format!(
                    "Cannot move field '{}.{}' because '{}' is borrowed by: {}",
                    object, field, object, borrowers.join(", ")
                ));
                return;
            }

            // NEW: Check method qualifier restrictions on field moves
            // If object is "this" (or we're in a method context), check this pointer rules
            if let Some(tracker) = this_tracker {
                if object == "this" {
                    if let Err(err) = tracker.can_move_member(field) {
                        errors.push(err);
                        return;
                    }
                }
            }

            // Mark the field as moved
            ownership_tracker.mark_field_moved(object.clone(), field.clone());
            ownership_tracker.set_ownership(to.clone(), OwnershipState::Owned);

            // Update this tracker state if this is a field of 'this'
            if let Some(tracker) = this_tracker {
                if object == "this" {
                    tracker.mark_field_moved(field.clone());
                }
            }
        }

        crate::ir::IrStatement::UseField { object, field, operation } => {
            debug_println!("DEBUG ANALYSIS: UseField object='{}', field='{}', operation='{}'", object, field, operation);

            // Skip checking if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }

            // Check if the object has been moved
            let object_state = ownership_tracker.get_ownership(object);
            if object_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Cannot {} field '{}.{}' because '{}' has been moved",
                    operation, object, field, object
                ));
                return;
            }

            // Check if the field has been moved
            let field_state = ownership_tracker.get_field_ownership(object, field);
            if field_state == OwnershipState::Moved {
                errors.push(format!(
                    "Cannot {} field '{}.{}' because it has been moved",
                    operation, object, field
                ));
                return;
            }

            // NEW: Check method qualifier restrictions on field usage
            if let Some(tracker) = this_tracker {
                if object == "this" {
                    // For read operations, just check if we can read
                    if operation == "read" {
                        if let Err(err) = tracker.can_read_member(field) {
                            errors.push(err);
                            return;
                        }
                    }
                    // For write operations, check if we can modify
                    else if operation == "write" {
                        if let Err(err) = tracker.can_modify_member(field) {
                            errors.push(err);
                            return;
                        }
                    }
                }
            }
        }

        crate::ir::IrStatement::BorrowField { object, field, to, kind } => {
            debug_println!("DEBUG ANALYSIS: BorrowField from '{}.{}' to '{}'", object, field, to);

            // Skip checking if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }

            // Check if the object has been moved
            let object_state = ownership_tracker.get_ownership(object);
            if object_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Cannot borrow field '{}.{}' because '{}' has been moved",
                    field, object, object
                ));
                return;
            }

            // Check if the field has been moved
            let field_state = ownership_tracker.get_field_ownership(object, field);
            if field_state == OwnershipState::Moved {
                errors.push(format!(
                    "Cannot borrow field '{}.{}' because it has been moved",
                    object, field
                ));
                return;
            }

            // NEW: Check method qualifier restrictions on field borrows
            if let Some(tracker) = this_tracker {
                if object == "this" {
                    if let Err(err) = tracker.can_borrow_member(field, kind.clone()) {
                        errors.push(err);
                        return;
                    }
                }
            }

            // Record the borrow (for now, borrow the whole object)
            // In a complete implementation, we'd track field-level borrows separately
            ownership_tracker.add_borrow(object.clone(), to.clone(), kind.clone());
            ownership_tracker.mark_as_reference(to.clone(), *kind == BorrowKind::Mutable);

            // Update this tracker state if this is a field of 'this'
            if let Some(tracker) = this_tracker {
                if object == "this" {
                    tracker.mark_field_borrowed(field.clone(), kind.clone());
                }
            }
        }

        crate::ir::IrStatement::Borrow { from, to, kind } => {
            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                // Still record the borrow for consistency
                ownership_tracker.add_borrow(from.clone(), to.clone(), kind.clone());
                ownership_tracker.mark_as_reference(to.clone(), *kind == BorrowKind::Mutable);
                return;
            }
            
            // Check if the source is accessible
            let from_state = ownership_tracker.get_ownership(from);
            
            if from_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Cannot borrow '{}' because it has been moved",
                    from
                ));
                return;
            }
            
            // Phase 3: Check for borrow conflicts using helper function
            if !check_borrow_conflicts(from, kind, ownership_tracker, errors) {
                return;
            }
            
            // Record the borrow
            ownership_tracker.add_borrow(from.clone(), to.clone(), kind.clone());
            ownership_tracker.mark_as_reference(to.clone(), *kind == BorrowKind::Mutable);
        }
        
        crate::ir::IrStatement::Assign { lhs, rhs } => {
            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }
            
            // Check if we're trying to modify through a const reference
            if ownership_tracker.is_reference(lhs) && !ownership_tracker.is_mutable_reference(lhs) {
                errors.push(format!(
                    "Cannot assign to '{}' through const reference",
                    lhs
                ));
            }
            
            // Check if the rhs uses a moved variable
            if let crate::ir::IrExpression::Variable(rhs_var) = rhs {
                if ownership_tracker.get_ownership(rhs_var) == Some(&OwnershipState::Moved) {
                    errors.push(format!(
                        "Use after move: variable '{}' has been moved",
                        rhs_var
                    ));
                }
            }

            // REASSIGNMENT FIX: After assignment, lhs becomes Owned again
            // This handles the case where a moved variable is reassigned a new value
            // Example: x = std::move(y); x = 42;  // x is valid again after reassignment
            if !ownership_tracker.is_reference(lhs) {
                ownership_tracker.set_ownership(lhs.clone(), OwnershipState::Owned);
            }
        }

        crate::ir::IrStatement::Drop(var) => {
            debug_println!("DEBUG ANALYSIS: Processing explicit Drop for '{}'", var);
            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                // Don't mark as moved - the subsequent assignment will handle ownership
                return;
            }

            // Check if the variable has active borrows
            // Explicit Drop (e.g., from reassignment of RAII type) checks borrows
            if let Some(borrows) = ownership_tracker.get_active_borrows(var) {
                if !borrows.is_empty() {
                    let borrower_names: Vec<String> = borrows.iter().map(|b| b.borrower.clone()).collect();
                    errors.push(format!(
                        "Cannot assign to '{}' because it is borrowed by: {} (assignment would drop the old value)",
                        var,
                        borrower_names.join(", ")
                    ));
                    return;
                }
            }

            // Check if variable is already moved
            let state = ownership_tracker.get_ownership(var);
            if state == Some(&OwnershipState::Moved) {
                debug_println!("DEBUG ANALYSIS: Skipping drop check for '{}' - already moved", var);
                // Still allow the drop check, but subsequent assignment will fail
            }

            // For explicit Drop (reassignment), we only check borrows.
            // We do NOT mark the variable as moved here!
            // The subsequent assignment IR statement (CallExpr, Assign, Move) will
            // handle the actual ownership transfer.
            debug_println!("DEBUG ANALYSIS: Drop check passed for '{}' - subsequent assignment will transfer ownership", var);
        }

        crate::ir::IrStatement::ImplicitDrop { var, has_destructor, .. } => {
            debug_println!("DEBUG ANALYSIS: Processing ImplicitDrop for '{}' (has_destructor={})", var, has_destructor);
            // Skip checks if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                // Still update ownership state for consistency (only for RAII types)
                if *has_destructor {
                    ownership_tracker.set_ownership(var.clone(), OwnershipState::Moved);
                }
                // Always clear borrows from the variable (references and RAII types)
                ownership_tracker.clear_borrows_from(var);
                return;
            }

            // Only check active borrows for RAII types (which actually drop)
            // References just clear their borrows without error checking
            if *has_destructor {
                // Check if the variable has active borrows
                // In Rust, implicit drop (scope end) is a move/consume operation
                if let Some(borrows) = ownership_tracker.get_active_borrows(var) {
                    if !borrows.is_empty() {
                        let borrower_names: Vec<String> = borrows.iter().map(|b| b.borrower.clone()).collect();
                        errors.push(format!(
                            "Cannot drop '{}' because it is borrowed by: {} (implicit drop at scope end)",
                            var,
                            borrower_names.join(", ")
                        ));
                        return;
                    }
                }

                // Check if variable is already moved
                let state = ownership_tracker.get_ownership(var);
                if state == Some(&OwnershipState::Moved) {
                    debug_println!("DEBUG ANALYSIS: Skipping implicit drop for '{}' - already moved", var);
                    return;  // Don't drop if already moved
                }

                // Mark as dropped (moved/consumed) - only for RAII types
                debug_println!("DEBUG ANALYSIS: Marking '{}' as dropped (implicit drop)", var);
                ownership_tracker.set_ownership(var.clone(), OwnershipState::Moved);
            }

            // NEW: Always clear borrows FROM this variable (for both RAII and non-RAII)
            // When a variable goes out of scope, any references it made become invalid
            // In C++, variables drop in reverse declaration order, so clearing borrows
            // after each "drop" simulates this correctly
            debug_println!("DEBUG ANALYSIS: Clearing borrows from '{}'", var);
            ownership_tracker.clear_borrows_from(var);
        }

        crate::ir::IrStatement::EnterScope => {
            ownership_tracker.enter_scope();
        }
        
        crate::ir::IrStatement::ExitScope => {
            // Before exiting scope, check for dangling references
            // A dangling reference occurs when:
            // 1. A variable x is defined in the current scope (will die)
            // 2. A reference ref from an outer scope borrows from x
            // 3. ref will outlive x, becoming a dangling reference
            let current_scope = ownership_tracker.scope_stack.len();

            // Find all variables defined at the current scope level
            for (var_name, var_info) in &function.variables {
                if var_info.scope_level == current_scope {
                    // This variable is dying - check if any outer-scope references borrow from it
                    if let Some(active_borrows) = ownership_tracker.active_borrows.get(var_name) {
                        for borrow in active_borrows {
                            // Check if the borrower (reference) is from an outer scope
                            if borrow.scope < current_scope {
                                errors.push(format!(
                                    "Dangling reference: '{}' borrows from '{}' which goes out of scope",
                                    borrow.borrower, var_name
                                ));
                            }
                        }
                    }
                }
            }

            ownership_tracker.exit_scope();
        }
        
        crate::ir::IrStatement::EnterLoop => {
            // Handled at the higher level
        }
        
        crate::ir::IrStatement::ExitLoop => {
            // Handled at the higher level
        }
        
        crate::ir::IrStatement::EnterUnsafe => {
            ownership_tracker.unsafe_depth += 1;
        }
        
        crate::ir::IrStatement::ExitUnsafe => {
            if ownership_tracker.unsafe_depth > 0 {
                ownership_tracker.unsafe_depth -= 1;
            }
        }
        
        crate::ir::IrStatement::If { then_branch, else_branch } => {
            // Skip checking if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }
            // Handle conditional execution with path-sensitive analysis
            // Save current state before branching
            let state_before_if = ownership_tracker.clone_state();
            
            // Process then branch
            for stmt in then_branch {
                process_statement(stmt, ownership_tracker, this_tracker, errors, header_cache, function);
            }
            let state_after_then = ownership_tracker.clone_state();

            // Restore state and process else branch if it exists
            ownership_tracker.restore_state(&state_before_if);

            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    process_statement(stmt, ownership_tracker, this_tracker, errors, header_cache, function);
                }
                let state_after_else = ownership_tracker.clone_state();

                // Merge states: a variable is moved if moved in ANY branch (Rust's aggressive approach)
                ownership_tracker.merge_states(&state_after_then, &state_after_else);
            } else {
                // No else branch: merge with original state
                // Variable is moved if moved in then branch (aggressive approach)
                ownership_tracker.merge_states(&state_after_then, &state_before_if);
            }
        }

        crate::ir::IrStatement::UseVariable { var, operation } => {
            debug_println!("DEBUG ANALYSIS: UseVariable var='{}', operation='{}'", var, operation);

            // Skip checking if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                debug_println!("DEBUG ANALYSIS: Skipping check - in unsafe block");
                return;
            }

            // Check if the variable has been moved
            let var_state = ownership_tracker.get_ownership(var);
            debug_println!("DEBUG ANALYSIS: var_state for '{}' = {:?}", var, var_state);

            if var_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Use after move: cannot {} variable '{}' because it has been moved",
                    operation, var
                ));
            }
        }

        crate::ir::IrStatement::Return { value } => {
            // Skip if in unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }

            if let Some(val) = value {
                // Check if returning a moved value
                let var_state = ownership_tracker.get_ownership(val);

                if var_state == Some(&OwnershipState::Moved) {
                    errors.push(format!(
                        "Cannot return '{}' because it has been moved",
                        val
                    ));
                }
            }
        }

        crate::ir::IrStatement::PackExpansion { pack_name, operation } => {
            // Phase 4: Handle pack expansion semantics
            debug_println!("DEBUG ANALYSIS: PackExpansion pack='{}', operation='{}'", pack_name, operation);

            // Skip checking if we're in an unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                debug_println!("DEBUG ANALYSIS: Skipping pack check - in unsafe block");
                return;
            }

            // Check if the pack has been moved
            let pack_state = ownership_tracker.get_ownership(pack_name);
            debug_println!("DEBUG ANALYSIS: pack_state for '{}' = {:?}", pack_name, pack_state);

            if pack_state == Some(&OwnershipState::Moved) {
                errors.push(format!(
                    "Use after move: cannot use pack '{}' because it has been moved",
                    pack_name
                ));
                return;
            }

            // Apply operation-specific semantics
            match operation.as_str() {
                "move" | "forward" => {
                    // Move or forward consumes the pack
                    debug_println!("DEBUG ANALYSIS: Pack '{}' is being moved/forwarded", pack_name);
                    ownership_tracker.set_ownership(pack_name.clone(), OwnershipState::Moved);
                }
                "use" => {
                    // Regular use creates implicit immutable borrows
                    // (packs are pass-by-value, so this doesn't create lasting borrows)
                    debug_println!("DEBUG ANALYSIS: Pack '{}' is being used (immutable)", pack_name);
                    // No state change needed for use
                }
                _ => {
                    debug_println!("DEBUG ANALYSIS: Unknown pack operation '{}'", operation);
                }
            }
        }

        // Phase 2: Handle CallExpr - detect return value borrows
        crate::ir::IrStatement::CallExpr { func, args, result } => {
            debug_println!("DEBUG ANALYSIS PHASE2: CallExpr func='{}', args={:?}, result={:?}", func, args, result);

            // Skip if in unsafe block
            if ownership_tracker.is_in_unsafe_block() {
                return;
            }

            // Skip if no result variable (void return)
            let result_var = match result {
                Some(r) => r,
                None => return,
            };

            debug_println!("DEBUG ANALYSIS PHASE2: Processing call result '{}'", result_var);

            // Phase 2: Detect return value borrows from lifetime annotations
            // Try to get the function signature from HeaderCache
            if let Some(signature) = header_cache.get_signature(func) {
                debug_println!("DEBUG ANALYSIS PHASE2: Found signature for function '{}'", func);

                // Check if the function has lifetime annotations
                if !signature.param_lifetimes.is_empty() || signature.return_lifetime.is_some() {
                    debug_println!("DEBUG ANALYSIS PHASE2: Function '{}' has lifetime annotations", func);

                    // Check if return type has a lifetime annotation
                    if let Some(ret_lifetime) = &signature.return_lifetime {
                        debug_println!("DEBUG ANALYSIS PHASE2: Return lifetime annotation found");

                        // Find which parameter has a matching lifetime
                        for (param_idx, param_lifetime_opt) in signature.param_lifetimes.iter().enumerate() {
                            if let Some(param_lifetime) = param_lifetime_opt {
                                // Check if lifetimes match (compare lifetime names)
                                let ret_lifetime_name = match ret_lifetime {
                                    crate::parser::annotations::LifetimeAnnotation::Ref(name) |
                                    crate::parser::annotations::LifetimeAnnotation::MutRef(name) |
                                    crate::parser::annotations::LifetimeAnnotation::Lifetime(name) => Some(name),
                                    _ => None,
                                };

                                let param_lifetime_name = match param_lifetime {
                                    crate::parser::annotations::LifetimeAnnotation::Ref(name) |
                                    crate::parser::annotations::LifetimeAnnotation::MutRef(name) |
                                    crate::parser::annotations::LifetimeAnnotation::Lifetime(name) => Some(name),
                                    _ => None,
                                };

                                // If lifetimes match, the return value borrows from this parameter
                                if ret_lifetime_name.is_some() && ret_lifetime_name == param_lifetime_name {
                                    debug_println!("DEBUG ANALYSIS PHASE2: Found matching lifetime '{}' between return and param {}",
                                        ret_lifetime_name.unwrap(), param_idx);

                                    // Get the parameter variable name from args
                                    if param_idx < args.len() {
                                        let borrowed_var = &args[param_idx];
                                        debug_println!("DEBUG ANALYSIS PHASE2: Return value '{}' borrows from parameter '{}'",
                                            result_var, borrowed_var);

                                        // CROSS-FUNCTION LIFETIME CHECK: Detect temporaries
                                        // If the borrowed variable is a temporary (literal or expression),
                                        // the return value would be a dangling reference
                                        if borrowed_var.starts_with("_temp_literal_") || borrowed_var.starts_with("_temp_expr_") {
                                            debug_println!("DEBUG ANALYSIS: Detected dangling reference from temporary argument");
                                            errors.push(format!(
                                                "Dangling reference: function '{}' returns reference tied to temporary argument",
                                                func
                                            ));
                                            break;  // Don't process further
                                        }

                                        // Determine borrow kind from RETURN annotation (not param)
                                        // If return annotation is &'a mut -> mutable, otherwise immutable
                                        // Also check the actual C++ variable type as fallback
                                        let borrow_kind = match ret_lifetime {
                                            crate::parser::annotations::LifetimeAnnotation::MutRef(_) => BorrowKind::Mutable,
                                            _ => {
                                                // Fallback: check the actual C++ variable type
                                                if let Some(var_info) = function.variables.get(result_var) {
                                                    if matches!(var_info.ty, crate::ir::VariableType::MutableReference(_)) {
                                                        BorrowKind::Mutable
                                                    } else {
                                                        BorrowKind::Immutable
                                                    }
                                                } else {
                                                    BorrowKind::Immutable
                                                }
                                            }
                                        };

                                        // Record the borrow with MethodReturnValue source
                                        let borrow_source = BorrowSource::MethodReturnValue {
                                            method: func.clone(),
                                            receiver: borrowed_var.clone(),
                                        };

                                        // Phase 2 FIX: Only create borrow if return type isn't "owned"
                                        // The lifetime annotation tells us whether a borrow exists, not the C++ type
                                        // Exception: For primitive types (int, bool, etc.) that are value types,
                                        // only create borrow if result variable is actually a reference
                                        debug_println!("DEBUG ANALYSIS PHASE2: Checking if should create borrow for '{}'", result_var);
                                        debug_println!("DEBUG ANALYSIS PHASE2: ret_lifetime.is_owned() = {}", ret_lifetime.is_owned());

                                        let should_create_borrow = if !ret_lifetime.is_owned() {
                                            // Return type has a lifetime annotation - check if we should create borrow
                                            if let Some(var_info) = function.variables.get(result_var) {
                                                debug_println!("DEBUG ANALYSIS PHASE2: Found var_info for '{}', type = {:?}", result_var, var_info.ty);
                                                let is_ref = matches!(var_info.ty,
                                                    crate::ir::VariableType::Reference(_) |
                                                    crate::ir::VariableType::MutableReference(_));
                                                debug_println!("DEBUG ANALYSIS PHASE2: is_ref = {}", is_ref);

                                                let is_complex = matches!(&var_info.ty, crate::ir::VariableType::Owned(type_name)
                                                    if !is_primitive_type(type_name));
                                                debug_println!("DEBUG ANALYSIS PHASE2: is_complex = {}", is_complex);

                                                is_ref || is_complex
                                            } else {
                                                debug_println!("DEBUG ANALYSIS PHASE2: No var_info found for '{}', assuming complex", result_var);
                                                // Unknown variable - assume it could be a complex type
                                                true
                                            }
                                        } else {
                                            debug_println!("DEBUG ANALYSIS PHASE2: Return type is owned - no borrow");
                                            // Return type is "owned" - no borrow
                                            false
                                        };

                                        debug_println!("DEBUG ANALYSIS PHASE2: should_create_borrow = {}", should_create_borrow);

                                        if should_create_borrow {
                                            debug_println!("DEBUG ANALYSIS PHASE2: Adding {} borrow: '{}' -> '{}' (result is reference type)",
                                                if borrow_kind == BorrowKind::Mutable { "mutable" } else { "immutable" },
                                                borrowed_var, result_var);

                                            // Phase 3: Check for borrow conflicts before creating the borrow
                                            if !check_borrow_conflicts(borrowed_var, &borrow_kind, ownership_tracker, errors) {
                                                debug_println!("DEBUG ANALYSIS PHASE3: Borrow conflict detected for '{}'", borrowed_var);
                                                break;  // Don't create the borrow
                                            }

                                            let is_mutable = borrow_kind == BorrowKind::Mutable;

                                            ownership_tracker.add_borrow_with_source(
                                                borrowed_var.clone(),
                                                result_var.clone(),
                                                borrow_kind,
                                                borrow_source
                                            );

                                            // Mark result as a reference
                                            ownership_tracker.mark_as_reference(
                                                result_var.clone(),
                                                is_mutable
                                            );
                                        } else {
                                            debug_println!("DEBUG ANALYSIS PHASE2: Skipping borrow creation for '{}' - result is value type, not reference",
                                                result_var);
                                        }

                                        // Only process first matching lifetime
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                debug_println!("DEBUG ANALYSIS PHASE2: No signature found for function '{}'", func);
            }
        }

        _ => {}
    }
}

struct OwnershipTracker {
    ownership: HashMap<String, OwnershipState>,
    borrows: HashMap<String, BorrowInfo>,
    reference_info: HashMap<String, ReferenceInfo>,
    // Stack of scopes, each scope tracks borrows created in it
    scope_stack: Vec<ScopeInfo>,
    // Loop tracking
    loop_depth: usize,
    // Save state when entering a loop (for 2nd iteration checking)
    loop_entry_states: Vec<LoopEntryState>,
    // Track if we're in an unsafe block
    unsafe_depth: usize,
    // Track active borrows: which variables are currently borrowed from
    // Key: variable being borrowed from, Value: list of active borrows on it
    active_borrows: HashMap<String, Vec<ActiveBorrow>>,
    // NEW: Track field-level ownership state
    // Key: object name, Value: map of field name to ownership state
    field_ownership: HashMap<String, HashMap<String, OwnershipState>>,
    // NEW: Track method context (are we in a method? is 'this' owned or borrowed?)
    this_context: Option<ThisContext>,
    // NEW: Liveness analysis - track last use of variables
    // Key: variable name, Value: statement index of last use
    last_use_map: HashMap<String, usize>,
}

#[derive(Clone)]
struct TrackerState {
    ownership: HashMap<String, OwnershipState>,
    borrows: HashMap<String, BorrowInfo>,
    reference_info: HashMap<String, ReferenceInfo>,
    active_borrows: HashMap<String, Vec<ActiveBorrow>>,
    // NEW: Field-level ownership tracking
    field_ownership: HashMap<String, HashMap<String, OwnershipState>>,
}

#[derive(Clone)]
struct LoopEntryState {
    ownership: HashMap<String, OwnershipState>,
    #[allow(dead_code)]
    borrows: HashMap<String, BorrowInfo>,
}

#[derive(Default, Clone)]
struct ScopeInfo {
    // Borrows created in this scope (to be cleaned up on exit)
    local_borrows: HashSet<String>,
}

#[derive(Default, Clone)]
struct BorrowInfo {
    immutable_count: usize,
    has_mutable: bool,
    borrowers: HashSet<String>,
}

#[derive(Clone)]
struct ReferenceInfo {
    is_reference: bool,
    is_mutable: bool,
}

// Track active borrows: when a variable is borrowed by a reference,
// we need to prevent moving the borrowed variable
#[derive(Clone, Debug)]
struct ActiveBorrow {
    borrower: String,      // The reference variable that is borrowing (e.g., "ref")
    borrowed_from: String, // The variable being borrowed from (e.g., "ptr")
    kind: BorrowKind,
    scope: usize,          // Scope level where this borrow was created
    // Phase 2: Track HOW the borrow was created
    source: BorrowSource,
}

// Phase 2: Represents how a borrow was created
#[derive(Clone, Debug, PartialEq)]
enum BorrowSource {
    DirectReference,          // T& ref = value;
    MethodReturnValue {       // auto x = obj.method();
        method: String,       // Method name (e.g., "as_ref", "as_mut")
        receiver: String,     // Object the method was called on
    },
    FieldAccess {             // auto& x = obj.field;
        object: String,
        field: String,
    },
}

// Track method context: how is 'this' accessed in methods?
#[derive(Clone, Debug, PartialEq)]
enum ThisContext {
    Borrowed,       // Regular method - implicit &self
    MutBorrowed,    // Mutable method - implicit &mut self
    ConstBorrowed,  // Const method - const &self
    Consumed,       // Rvalue ref method - implicit &&self (owned)
}

impl OwnershipTracker {
    fn new() -> Self {
        Self::with_liveness(HashMap::new())
    }

    fn with_liveness(last_use_map: HashMap<String, usize>) -> Self {
        let mut tracker = Self {
            ownership: HashMap::new(),
            borrows: HashMap::new(),
            reference_info: HashMap::new(),
            scope_stack: Vec::new(),
            loop_depth: 0,
            loop_entry_states: Vec::new(),
            unsafe_depth: 0,
            active_borrows: HashMap::new(),
            field_ownership: HashMap::new(),  // NEW
            this_context: None,                // NEW
            last_use_map,                      // NEW: Liveness analysis
        };
        // Start with a root scope
        tracker.scope_stack.push(ScopeInfo::default());
        tracker
    }
    
    fn is_in_unsafe_block(&self) -> bool {
        self.unsafe_depth > 0
    }
    
    fn set_ownership(&mut self, var: String, state: OwnershipState) {
        self.ownership.insert(var, state);
    }
    
    fn get_ownership(&self, var: &str) -> Option<&OwnershipState> {
        self.ownership.get(var)
    }
    
    fn get_borrows(&self, var: &str) -> BorrowInfo {
        self.borrows.get(var).cloned().unwrap_or_default()
    }
    
    // Phase 2: Enhanced add_borrow with source tracking
    fn add_borrow_with_source(&mut self, from: String, to: String, kind: BorrowKind, source: BorrowSource) {
        let borrow_info = self.borrows.entry(from.clone()).or_default();
        borrow_info.borrowers.insert(to.clone());

        // Track this borrow in the current scope
        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.local_borrows.insert(to.clone());
        }

        match kind {
            BorrowKind::Immutable => borrow_info.immutable_count += 1,
            BorrowKind::Mutable => borrow_info.has_mutable = true,
        }

        // NEW: Record active borrow - track that 'from' is currently borrowed by 'to'
        let current_scope_level = self.scope_stack.len();
        let active_borrow = ActiveBorrow {
            borrower: to,
            borrowed_from: from.clone(),
            kind,
            scope: current_scope_level,
            source,  // Phase 2: Track borrow source
        };
        self.active_borrows.entry(from).or_default().push(active_borrow);
    }

    // Convenience function for direct reference borrows (most common case)
    fn add_borrow(&mut self, from: String, to: String, kind: BorrowKind) {
        self.add_borrow_with_source(from, to, kind, BorrowSource::DirectReference);
    }

    // NEW: Get active borrows for a variable
    fn get_active_borrows(&self, var: &str) -> Option<&Vec<ActiveBorrow>> {
        self.active_borrows.get(var)
    }

    /// Phase 4: Check if a variable is transitively borrowed
    /// Returns true if the variable is directly borrowed OR if any of its borrowers are themselves borrowed
    /// This detects borrow chains like: s -> ref_opt -> opt
    fn is_transitively_borrowed(&self, var: &str) -> bool {
        // Check for direct borrows
        if let Some(borrows) = self.active_borrows.get(var) {
            if !borrows.is_empty() {
                // Variable is directly borrowed - check if any borrowers are themselves borrowed
                for borrow in borrows {
                    // If this borrower is also borrowed (creating a chain), we can't move
                    if self.is_transitively_borrowed(&borrow.borrower) {
                        return true;
                    }
                }
                // Has direct borrows but none of the borrowers are borrowed
                return true;
            }
        }
        // Not borrowed at all
        false
    }

    /// Phase 4: Get all variables in the transitive borrow chain
    /// Returns a list of all borrowers in the chain, useful for error messages
    fn get_transitive_borrowers(&self, var: &str) -> Vec<String> {
        let mut result = Vec::new();

        if let Some(borrows) = self.active_borrows.get(var) {
            for borrow in borrows {
                result.push(borrow.borrower.clone());
                // Recursively get borrowers of this borrower
                let nested = self.get_transitive_borrowers(&borrow.borrower);
                result.extend(nested);
            }
        }

        result
    }

    // NEW: Clear all borrows FROM a variable (for liveness analysis)
    // This clears borrows where 'var' is the borrower (e.g., a reference that's now dead)
    fn clear_borrows_from(&mut self, var: &str) {
        debug_println!("LIVENESS: Clearing borrows from '{}'", var);

        // Remove all borrows where this variable is the borrower
        for (_borrowed_var, borrows) in &mut self.active_borrows {
            borrows.retain(|b| {
                if b.borrower == var {
                    debug_println!("LIVENESS: Removing borrow: '{}' → '{}'", var, _borrowed_var);
                    false
                } else {
                    true
                }
            });
        }

        // Clean up empty borrow lists
        self.active_borrows.retain(|_, borrows| !borrows.is_empty());
    }

    // NEW: Check if any variable reached its last use at this statement index
    // If so, clear its borrows (the variable is now dead)
    fn check_and_clear_last_uses(&mut self, statement_idx: usize) {
        let vars_to_clear: Vec<String> = self.last_use_map.iter()
            .filter(|(_, &last_use_idx)| last_use_idx == statement_idx)
            .map(|(var, _)| var.clone())
            .collect();

        for var in vars_to_clear {
            debug_println!("LIVENESS: Variable '{}' reached its last use at statement {}", var, statement_idx);
            self.clear_borrows_from(&var);
        }
    }

    // NEW: Helper methods for field-level ownership tracking

    /// Get ownership state of a specific field
    fn get_field_ownership(&self, object: &str, field: &str) -> OwnershipState {
        self.field_ownership
            .get(object)
            .and_then(|fields| fields.get(field))
            .cloned()
            .unwrap_or(OwnershipState::Owned)
    }

    /// Mark field as moved
    fn mark_field_moved(&mut self, object: String, field: String) {
        self.field_ownership
            .entry(object)
            .or_default()
            .insert(field, OwnershipState::Moved);
    }

    /// Check if object has any moved fields
    fn has_moved_fields(&self, object: &str) -> bool {
        self.field_ownership
            .get(object)
            .map(|fields| fields.values().any(|s| *s == OwnershipState::Moved))
            .unwrap_or(false)
    }

    /// Get list of moved fields
    fn get_moved_fields(&self, object: &str) -> Vec<String> {
        self.field_ownership
            .get(object)
            .map(|fields| {
                fields.iter()
                    .filter(|(_, state)| **state == OwnershipState::Moved)
                    .map(|(field, _)| field.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if can move from 'this' in current context
    fn can_move_from_this(&self) -> bool {
        match &self.this_context {
            Some(ThisContext::Consumed) => true,  // && method - can move
            Some(_) => false,  // All other methods - cannot move
            None => true,  // Not in method - OK (though this shouldn't happen)
        }
    }

    /// Set method context
    fn set_this_context(&mut self, context: Option<ThisContext>) {
        self.this_context = context;
    }

    fn enter_scope(&mut self) {
        self.scope_stack.push(ScopeInfo::default());
    }
    
    fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            let current_scope_level = self.scope_stack.len() + 1; // +1 because we just popped

            // Clean up all borrows created in this scope
            for borrow_name in &scope.local_borrows {
                // Remove from reference info
                self.reference_info.remove(borrow_name);

                // Remove from all borrow tracking
                for borrow_info in self.borrows.values_mut() {
                    borrow_info.borrowers.remove(borrow_name);
                    // Note: In a more complete implementation, we'd also
                    // decrement counts based on the borrow kind
                }

                // NEW: Remove from active borrows
                // Remove any active borrow where this variable is the borrower
                for active_borrows in self.active_borrows.values_mut() {
                    active_borrows.retain(|b| &b.borrower != borrow_name);
                }
            }

            // Clean up empty borrow entries
            self.borrows.retain(|_, info| !info.borrowers.is_empty());

            // NEW: Clean up empty active borrow entries
            self.active_borrows.retain(|_, borrows| !borrows.is_empty());
        }
    }
    
    fn mark_as_reference(&mut self, var: String, is_mutable: bool) {
        self.reference_info.insert(var, ReferenceInfo {
            is_reference: true,
            is_mutable,
        });
    }
    
    fn is_reference(&self, var: &str) -> bool {
        self.reference_info
            .get(var)
            .map(|info| info.is_reference)
            .unwrap_or(false)
    }
    
    fn is_mutable_reference(&self, var: &str) -> bool {
        self.reference_info
            .get(var)
            .map(|info| info.is_reference && info.is_mutable)
            .unwrap_or(false)
    }
    
    fn enter_loop(&mut self) {
        // Save current state when entering a loop
        // This state represents the state at the END of the first iteration
        // which is what we'll use to check the BEGINNING of the second iteration
        self.loop_entry_states.push(LoopEntryState {
            ownership: self.ownership.clone(),
            borrows: self.borrows.clone(),
        });
        self.loop_depth += 1;
    }
    
    fn exit_loop(&mut self) {
        if self.loop_depth > 0 {
            self.loop_depth -= 1;
            
            // When exiting a loop, we simulate having run it twice
            // The current state is after one iteration
            // We saved the state at loop entry, now apply the second iteration effects
            if let Some(entry_state) = self.loop_entry_states.pop() {
                // The key insight: variables that were moved in the loop body
                // will be moved at the START of the second iteration
                // So check if any variables that are currently Moved
                // were NOT moved at loop entry
                for (var, current_state) in &self.ownership {
                    if *current_state == OwnershipState::Moved {
                        // If this variable was Owned at loop entry,
                        // it means it was moved during the loop body
                        // On second iteration, it would already be Moved
                        if let Some(entry_ownership) = entry_state.ownership.get(var) {
                            if *entry_ownership == OwnershipState::Owned {
                                // Keep it as Moved - this correctly represents
                                // the state after 2 iterations
                                // The error will be caught if the variable is used
                                // in the loop body (which we already processed)
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn clone_state(&self) -> TrackerState {
        TrackerState {
            ownership: self.ownership.clone(),
            borrows: self.borrows.clone(),
            reference_info: self.reference_info.clone(),
            active_borrows: self.active_borrows.clone(),
            field_ownership: self.field_ownership.clone(),  // NEW
        }
    }

    fn restore_state(&mut self, state: &TrackerState) {
        self.ownership = state.ownership.clone();
        self.borrows = state.borrows.clone();
        self.reference_info = state.reference_info.clone();
        self.active_borrows = state.active_borrows.clone();
        self.field_ownership = state.field_ownership.clone();  // NEW
    }
    
    fn merge_states(&mut self, then_state: &TrackerState, else_state: &TrackerState) {
        // Merge ownership states aggressively (matching Rust's behavior)
        // A variable is considered moved if moved in ANY branch
        for (var, then_ownership) in &then_state.ownership {
            if let Some(else_ownership) = else_state.ownership.get(var) {
                if *then_ownership == OwnershipState::Moved || *else_ownership == OwnershipState::Moved {
                    // Moved in at least one branch - mark as moved (Rust's aggressive approach)
                    // This is sound: if any path moves the variable, it's unsafe to use after
                    self.ownership.insert(var.clone(), OwnershipState::Moved);
                } else {
                    // Not moved in either branch - use the common state
                    self.ownership.insert(var.clone(), then_ownership.clone());
                }
            }
        }
        
        // Merge borrows - a borrow exists only if it exists in BOTH branches
        // This is conservative: if a borrow doesn't exist in one branch, it's not guaranteed after the if
        self.borrows.clear();
        for (var, then_borrow) in &then_state.borrows {
            if let Some(else_borrow) = else_state.borrows.get(var) {
                // Borrow exists in both branches - keep it
                let mut merged_borrow = then_borrow.clone();
                // Keep only common borrowers
                merged_borrow.borrowers.retain(|b| else_borrow.borrowers.contains(b));
                // Use minimum counts (conservative)
                merged_borrow.immutable_count = merged_borrow.immutable_count.min(else_borrow.immutable_count);
                merged_borrow.has_mutable = merged_borrow.has_mutable && else_borrow.has_mutable;
                
                if !merged_borrow.borrowers.is_empty() {
                    self.borrows.insert(var.clone(), merged_borrow);
                }
            }
            // If borrow doesn't exist in else branch, don't include it
        }
        
        // Also clear reference info for references that don't exist in both branches
        let mut refs_to_keep = HashSet::new();
        for (var, _) in &then_state.reference_info {
            if else_state.reference_info.contains_key(var) {
                refs_to_keep.insert(var.clone());
            }
        }
        self.reference_info.retain(|var, _| refs_to_keep.contains(var));

        // Merge active borrows - keep only borrows that exist in BOTH branches
        // This is conservative: if a borrow doesn't exist in one branch, it's not guaranteed after the if
        self.active_borrows.clear();
        for (var, then_borrows) in &then_state.active_borrows {
            if let Some(else_borrows) = else_state.active_borrows.get(var) {
                // Borrow exists in both branches - keep common borrows
                let then_borrowers: HashSet<String> = then_borrows.iter().map(|b| b.borrower.clone()).collect();
                let else_borrowers: HashSet<String> = else_borrows.iter().map(|b| b.borrower.clone()).collect();

                let common_borrowers: Vec<ActiveBorrow> = then_borrows.iter()
                    .filter(|b| else_borrowers.contains(&b.borrower))
                    .cloned()
                    .collect();

                if !common_borrowers.is_empty() {
                    self.active_borrows.insert(var.clone(), common_borrowers);
                }
            }
        }

        // NEW: Merge field ownership - field moved in EITHER branch is marked as moved
        self.field_ownership.clear();
        // Collect all objects that have field ownership in either branch
        let mut all_objects: HashSet<String> = HashSet::new();
        all_objects.extend(then_state.field_ownership.keys().cloned());
        all_objects.extend(else_state.field_ownership.keys().cloned());

        for object in all_objects {
            let then_fields = then_state.field_ownership.get(&object);
            let else_fields = else_state.field_ownership.get(&object);

            match (then_fields, else_fields) {
                (Some(then_f), Some(else_f)) => {
                    // Object has fields in both branches
                    let mut merged_fields = HashMap::new();

                    // Collect all field names
                    let mut all_field_names: HashSet<String> = HashSet::new();
                    all_field_names.extend(then_f.keys().cloned());
                    all_field_names.extend(else_f.keys().cloned());

                    for field in all_field_names {
                        let then_state = then_f.get(&field);
                        let else_state = else_f.get(&field);

                        match (then_state, else_state) {
                            (Some(t), Some(e)) => {
                                // Field exists in both - moved if moved in either
                                if *t == OwnershipState::Moved || *e == OwnershipState::Moved {
                                    merged_fields.insert(field, OwnershipState::Moved);
                                } else {
                                    merged_fields.insert(field, t.clone());
                                }
                            }
                            (Some(t), None) | (None, Some(t)) => {
                                // Field only in one branch - use that state
                                merged_fields.insert(field, t.clone());
                            }
                            (None, None) => unreachable!(),
                        }
                    }

                    if !merged_fields.is_empty() {
                        self.field_ownership.insert(object, merged_fields);
                    }
                }
                (Some(fields), None) | (None, Some(fields)) => {
                    // Object only has fields in one branch - keep those fields
                    self.field_ownership.insert(object, fields.clone());
                }
                (None, None) => unreachable!(),
            }
        }
    }
    
    fn clear_loop_locals(&mut self, loop_locals: &HashSet<String>) {
        // Clear borrows for loop-local variables
        for local_var in loop_locals {
            // Remove from reference info
            self.reference_info.remove(local_var);
            
            // Remove from all borrow tracking
            for borrow_info in self.borrows.values_mut() {
                borrow_info.borrowers.remove(local_var);
                // We should also decrement counts, but need to track the kind
                // For simplicity, we'll rebuild the counts
            }
            
            // Remove the ownership entry for loop-local variables
            self.ownership.remove(local_var);
        }
        
        // Clean up empty borrow entries and recalculate counts
        for (_, borrow_info) in self.borrows.iter_mut() {
            // Reset counts based on remaining borrowers
            // This is a simplification - in a real implementation we'd track
            // the kind of each borrow
            if borrow_info.borrowers.is_empty() {
                borrow_info.immutable_count = 0;
                borrow_info.has_mutable = false;
            }
        }
        
        // Remove empty entries
        self.borrows.retain(|_, info| !info.borrowers.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrProgram, IrFunction, BasicBlock, IrStatement};
    use petgraph::graph::DiGraph;

    fn create_test_program() -> IrProgram {
        IrProgram {
            functions: vec![],
            ownership_graph: DiGraph::new(),
        }
    }

    fn create_test_function(name: &str) -> IrFunction {
        let mut cfg = DiGraph::new();
        let block = BasicBlock {
            id: 0,
            statements: vec![],
            terminator: None,
        };
        cfg.add_node(block);
        
        IrFunction {
            name: name.to_string(),
            cfg,
            variables: HashMap::new(),
            return_type: "void".to_string(),
            source_file: "test.cpp".to_string(),
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            lifetime_params: HashMap::new(),
            param_lifetimes: Vec::new(),
            return_lifetime: None,
            lifetime_constraints: Vec::new(),
        }
    }

    #[test]
    fn test_empty_program_passes() {
        let program = create_test_program();
        let result = check_borrows(program);
        
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_ownership_tracker_initialization() {
        let mut tracker = OwnershipTracker::new();
        tracker.set_ownership("x".to_string(), OwnershipState::Owned);
        
        assert_eq!(tracker.get_ownership("x"), Some(&OwnershipState::Owned));
        assert_eq!(tracker.get_ownership("y"), None);
    }

    #[test]
    fn test_ownership_state_transitions() {
        let mut tracker = OwnershipTracker::new();
        
        // Start with owned
        tracker.set_ownership("x".to_string(), OwnershipState::Owned);
        assert_eq!(tracker.get_ownership("x"), Some(&OwnershipState::Owned));
        
        // Move to another variable
        tracker.set_ownership("x".to_string(), OwnershipState::Moved);
        tracker.set_ownership("y".to_string(), OwnershipState::Owned);
        
        assert_eq!(tracker.get_ownership("x"), Some(&OwnershipState::Moved));
        assert_eq!(tracker.get_ownership("y"), Some(&OwnershipState::Owned));
    }

    #[test]
    fn test_borrow_tracking() {
        let mut tracker = OwnershipTracker::new();
        tracker.set_ownership("x".to_string(), OwnershipState::Owned);
        
        // Add immutable borrow
        tracker.add_borrow("x".to_string(), "ref1".to_string(), BorrowKind::Immutable);
        let borrows = tracker.get_borrows("x");
        assert_eq!(borrows.immutable_count, 1);
        assert!(!borrows.has_mutable);
        
        // Add another immutable borrow
        tracker.add_borrow("x".to_string(), "ref2".to_string(), BorrowKind::Immutable);
        let borrows = tracker.get_borrows("x");
        assert_eq!(borrows.immutable_count, 2);
        assert!(!borrows.has_mutable);
    }

    #[test]
    fn test_mutable_borrow_tracking() {
        let mut tracker = OwnershipTracker::new();
        tracker.set_ownership("x".to_string(), OwnershipState::Owned);
        
        // Add mutable borrow
        tracker.add_borrow("x".to_string(), "mut_ref".to_string(), BorrowKind::Mutable);
        let borrows = tracker.get_borrows("x");
        assert_eq!(borrows.immutable_count, 0);
        assert!(borrows.has_mutable);
    }

    #[test]
    fn test_use_after_move_detection() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        // Add variables
        func.variables.insert(
            "x".to_string(),
            crate::ir::VariableInfo {
                name: "x".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        func.variables.insert(
            "y".to_string(),
            crate::ir::VariableInfo {
                name: "y".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        // Add statements: move x to y, then try to use x
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        block.statements.push(IrStatement::Move {
            from: "x".to_string(),
            to: "y".to_string(),
        });
        
        // Try to move x again (should fail)
        block.statements.push(IrStatement::Move {
            from: "x".to_string(),
            to: "z".to_string(),
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Use after move"));
    }

    #[test]
    fn test_multiple_immutable_borrows_allowed() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        func.variables.insert(
            "x".to_string(),
            crate::ir::VariableInfo {
                name: "x".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        block.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref1".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        block.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref2".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0); // Multiple immutable borrows are OK
    }

    #[test]
    fn test_mutable_borrow_while_immutable_fails() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        func.variables.insert(
            "x".to_string(),
            crate::ir::VariableInfo {
                name: "x".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // First, immutable borrow
        block.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref1".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        // Then try mutable borrow (should fail)
        block.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "mut_ref".to_string(),
            kind: BorrowKind::Mutable,
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Cannot"));
        assert!(errors[0].contains("mutable"));
    }

    #[test]
    fn test_const_reference_cannot_modify() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        // Add a value and a const reference to it
        func.variables.insert(
            "value".to_string(),
            crate::ir::VariableInfo {
                name: "value".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        func.variables.insert(
            "const_ref".to_string(),
            crate::ir::VariableInfo {
                name: "const_ref".to_string(),
                ty: crate::ir::VariableType::Reference("int".to_string()),
                ownership: OwnershipState::Borrowed(BorrowKind::Immutable),
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Create const reference
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "const_ref".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        // Try to modify through const reference (should fail)
        block.statements.push(IrStatement::Assign {
            lhs: "const_ref".to_string(),
            rhs: crate::ir::IrExpression::Variable("other".to_string()),
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Cannot assign"));
        assert!(errors[0].contains("const reference"));
    }

    #[test]
    fn test_mutable_reference_can_modify() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        // Add a value and a mutable reference to it
        func.variables.insert(
            "value".to_string(),
            crate::ir::VariableInfo {
                name: "value".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        func.variables.insert(
            "mut_ref".to_string(),
            crate::ir::VariableInfo {
                name: "mut_ref".to_string(),
                ty: crate::ir::VariableType::MutableReference("int".to_string()),
                ownership: OwnershipState::Borrowed(BorrowKind::Mutable),
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Create mutable reference
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "mut_ref".to_string(),
            kind: BorrowKind::Mutable,
        });
        
        // Modify through mutable reference (should succeed)
        block.statements.push(IrStatement::Assign {
            lhs: "mut_ref".to_string(),
            rhs: crate::ir::IrExpression::Variable("other".to_string()),
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0); // Should succeed
    }

    #[test]
    fn test_cannot_move_from_reference() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        // Add a reference variable
        func.variables.insert(
            "ref_var".to_string(),
            crate::ir::VariableInfo {
                name: "ref_var".to_string(),
                ty: crate::ir::VariableType::Reference("int".to_string()),
                ownership: OwnershipState::Borrowed(BorrowKind::Immutable),
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        func.variables.insert(
            "dest".to_string(),
            crate::ir::VariableInfo {
                name: "dest".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Try to move from reference (should fail)
        block.statements.push(IrStatement::Move {
            from: "ref_var".to_string(),
            to: "dest".to_string(),
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Cannot move"));
        assert!(errors[0].contains("reference"));
    }

    #[test]
    fn test_multiple_const_references_allowed() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        func.variables.insert(
            "value".to_string(),
            crate::ir::VariableInfo {
                name: "value".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Create multiple const references (should succeed)
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "const_ref1".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "const_ref2".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "const_ref3".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0); // Multiple const references are allowed
    }

    #[test]
    fn test_cannot_borrow_moved_value() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        func.variables.insert(
            "value".to_string(),
            crate::ir::VariableInfo {
                name: "value".to_string(),
                ty: crate::ir::VariableType::UniquePtr("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Move the value
        block.statements.push(IrStatement::Move {
            from: "value".to_string(),
            to: "other".to_string(),
        });
        
        // Try to create reference to moved value (should fail)
        block.statements.push(IrStatement::Borrow {
            from: "value".to_string(),
            to: "ref".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert!(errors.len() > 0);
        assert!(errors.iter().any(|e| e.contains("moved")));
    }

    #[test]
    fn test_multiple_functions_with_references() {
        let mut program = create_test_program();
        
        // First function with valid const refs
        let mut func1 = create_test_function("func1");
        func1.variables.insert(
            "x".to_string(),
            crate::ir::VariableInfo {
                name: "x".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block1 = &mut func1.cfg[petgraph::graph::NodeIndex::new(0)];
        block1.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref1".to_string(),
            kind: BorrowKind::Immutable,
        });
        block1.statements.push(IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref2".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        // Second function with invalid refs
        let mut func2 = create_test_function("func2");
        func2.variables.insert(
            "y".to_string(),
            crate::ir::VariableInfo {
                name: "y".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block2 = &mut func2.cfg[petgraph::graph::NodeIndex::new(0)];
        block2.statements.push(IrStatement::Borrow {
            from: "y".to_string(),
            to: "mut1".to_string(),
            kind: BorrowKind::Mutable,
        });
        block2.statements.push(IrStatement::Borrow {
            from: "y".to_string(),
            to: "mut2".to_string(),
            kind: BorrowKind::Mutable,
        });
        
        program.functions.push(func1);
        program.functions.push(func2);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1); // Only func2 should have errors
        assert!(errors[0].contains("already mutably borrowed"));
    }

    #[test]
    fn test_complex_borrow_chain() {
        let mut program = create_test_program();
        let mut func = create_test_function("test");
        
        // Create variables
        func.variables.insert(
            "a".to_string(),
            crate::ir::VariableInfo {
                name: "a".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        func.variables.insert(
            "b".to_string(),
            crate::ir::VariableInfo {
                name: "b".to_string(),
                ty: crate::ir::VariableType::Owned("int".to_string()),
                ownership: OwnershipState::Owned,
                lifetime: None,
                is_parameter: false,
                is_static: false,
                scope_level: 0,
                has_destructor: false,
                declaration_index: 0,
            },
        );
        
        let block = &mut func.cfg[petgraph::graph::NodeIndex::new(0)];
        
        // Create multiple immutable refs to 'a'
        block.statements.push(IrStatement::Borrow {
            from: "a".to_string(),
            to: "ref_a1".to_string(),
            kind: BorrowKind::Immutable,
        });
        block.statements.push(IrStatement::Borrow {
            from: "a".to_string(),
            to: "ref_a2".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        // Create mutable ref to 'b'
        block.statements.push(IrStatement::Borrow {
            from: "b".to_string(),
            to: "mut_b".to_string(),
            kind: BorrowKind::Mutable,
        });
        
        // Try to create another ref to 'b' (should fail)
        block.statements.push(IrStatement::Borrow {
            from: "b".to_string(),
            to: "ref_b".to_string(),
            kind: BorrowKind::Immutable,
        });
        
        program.functions.push(func);
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("'b'"));
        assert!(errors[0].contains("already mutably borrowed"));
    }
}
#[cfg(test)]
mod scope_tests {
    use super::*;
    use crate::ir::{BasicBlock, IrFunction, IrProgram, IrStatement, BorrowKind};
    use petgraph::graph::Graph;
    use std::collections::HashMap;

    fn create_test_function_with_statements(statements: Vec<IrStatement>) -> IrFunction {
        let mut cfg = Graph::new();
        let block = BasicBlock {
            id: 0,
            statements,
            terminator: None,
        };
        cfg.add_node(block);

        IrFunction {
            name: "test".to_string(),
            cfg,
            variables: HashMap::new(),
            return_type: "void".to_string(),
            source_file: "test.cpp".to_string(),
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            lifetime_params: HashMap::new(),
            param_lifetimes: Vec::new(),
            return_lifetime: None,
            lifetime_constraints: Vec::new(),
        }
    }

    #[test]
    fn test_scope_cleanup_simple() {
        let statements = vec![
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref1".to_string(),
                kind: BorrowKind::Mutable,
            },
            IrStatement::ExitScope,
            // After scope exit, should be able to borrow again
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref2".to_string(),
                kind: BorrowKind::Mutable,
            },
        ];
        
        let func = create_test_function_with_statements(statements);
        let mut program = IrProgram {
            functions: vec![func],
            ownership_graph: petgraph::graph::DiGraph::new(),
        };
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0, "Should not report errors for borrows in different scopes");
    }

    #[test]
    fn test_nested_scopes() {
        let statements = vec![
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref1".to_string(),
                kind: BorrowKind::Immutable,
            },
            IrStatement::EnterScope,
            // Nested scope - should be able to have another immutable borrow
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref2".to_string(),
                kind: BorrowKind::Immutable,
            },
            IrStatement::ExitScope,
            // ref2 is gone, but ref1 still exists
            IrStatement::ExitScope,
            // Now both are gone
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref3".to_string(),
                kind: BorrowKind::Mutable,
            },
        ];
        
        let func = create_test_function_with_statements(statements);
        let mut program = IrProgram {
            functions: vec![func],
            ownership_graph: petgraph::graph::DiGraph::new(),
        };
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0, "Nested scopes should work correctly");
    }

    #[test]
    fn test_scope_doesnt_affect_moves() {
        let statements = vec![
            IrStatement::EnterScope,
            IrStatement::Move {
                from: "x".to_string(),
                to: "y".to_string(),
            },
            IrStatement::ExitScope,
            // x is still moved even after scope exit
            IrStatement::Move {
                from: "x".to_string(),
                to: "z".to_string(),
            },
        ];
        
        let func = create_test_function_with_statements(statements);
        let mut program = IrProgram {
            functions: vec![func],
            ownership_graph: petgraph::graph::DiGraph::new(),
        };
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert!(errors.len() > 0, "Should still detect use-after-move across scopes");
        assert!(errors[0].contains("already been moved") || errors[0].contains("Use after move"));
    }

    #[test]
    fn test_multiple_sequential_scopes() {
        let statements = vec![
            // First scope
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref1".to_string(),
                kind: BorrowKind::Mutable,
            },
            IrStatement::ExitScope,
            
            // Second scope
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref2".to_string(),
                kind: BorrowKind::Mutable,
            },
            IrStatement::ExitScope,
            
            // Third scope
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref3".to_string(),
                kind: BorrowKind::Mutable,
            },
            IrStatement::ExitScope,
        ];
        
        let func = create_test_function_with_statements(statements);
        let mut program = IrProgram {
            functions: vec![func],
            ownership_graph: petgraph::graph::DiGraph::new(),
        };
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 0, "Sequential scopes should not conflict");
    }

    #[test]
    fn test_error_still_caught_in_same_scope() {
        let statements = vec![
            IrStatement::EnterScope,
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref1".to_string(),
                kind: BorrowKind::Mutable,
            },
            // This should error - same scope
            IrStatement::Borrow {
                from: "value".to_string(),
                to: "ref2".to_string(),
                kind: BorrowKind::Mutable,
            },
            IrStatement::ExitScope,
        ];
        
        let func = create_test_function_with_statements(statements);
        let mut program = IrProgram {
            functions: vec![func],
            ownership_graph: petgraph::graph::DiGraph::new(),
        };
        
        let result = check_borrows(program);
        assert!(result.is_ok());
        let errors = result.unwrap();
        assert!(errors.len() > 0, "Should still catch errors within the same scope");
        assert!(errors[0].contains("already mutably borrowed"));
    }
}
