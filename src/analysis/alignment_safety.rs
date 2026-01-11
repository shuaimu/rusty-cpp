//! Alignment Safety Analysis (Phase 6 of Full Pointer Safety)
//!
//! This module tracks pointer alignment through arithmetic and casts to prevent
//! undefined behavior from accessing misaligned memory.
//!
//! Rules:
//! - Each allocation has an alignment based on its type or `alignas`
//! - Pointer arithmetic can change alignment offset
//! - Casts to stricter alignment require proper alignment
//! - Misaligned access is undefined behavior in C++
//!
//! Example:
//! ```cpp
//! alignas(8) char buffer[64];
//! int64_t* p1 = reinterpret_cast<int64_t*>(buffer);     // OK: 8-byte aligned
//! int64_t* p2 = reinterpret_cast<int64_t*>(buffer + 1); // ERROR: misaligned
//! ```

use crate::parser::{Statement, Expression, Function, CastKind};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Alignment information for a pointer
#[derive(Debug, Clone)]
pub struct AlignmentInfo {
    /// The base alignment of the allocation (in bytes)
    pub base_alignment: usize,
    /// Current offset from the aligned base (modulo base_alignment)
    pub offset: usize,
    /// The type this pointer points to
    pub pointee_type: String,
}

impl AlignmentInfo {
    /// Create alignment info with no offset
    pub fn aligned(alignment: usize, pointee_type: String) -> Self {
        AlignmentInfo {
            base_alignment: alignment,
            offset: 0,
            pointee_type,
        }
    }

    /// Create alignment info with an offset (used in tests)
    #[cfg(test)]
    pub fn with_offset(alignment: usize, offset: usize, pointee_type: String) -> Self {
        AlignmentInfo {
            base_alignment: alignment,
            offset: offset % alignment,
            pointee_type,
        }
    }

    /// Check if this pointer is properly aligned for a target alignment
    pub fn is_aligned_for(&self, target_alignment: usize) -> bool {
        if target_alignment == 0 || target_alignment == 1 {
            return true;
        }
        // Pointer is aligned if offset is 0 AND base alignment >= target
        self.offset % target_alignment == 0 && self.base_alignment >= target_alignment
    }

    /// Apply pointer arithmetic (addition of bytes) (used in tests)
    #[cfg(test)]
    pub fn add_offset(&self, bytes: usize, new_type: String) -> Self {
        AlignmentInfo {
            base_alignment: self.base_alignment,
            offset: (self.offset + bytes) % self.base_alignment,
            pointee_type: new_type,
        }
    }
}

/// Tracks pointer alignment
#[derive(Debug)]
pub struct AlignmentTracker {
    /// Maps pointer variable to its alignment info
    alignments: HashMap<String, AlignmentInfo>,
    /// Scope stack for nested blocks
    scope_stack: Vec<HashMap<String, AlignmentInfo>>,
}

impl AlignmentTracker {
    pub fn new() -> Self {
        AlignmentTracker {
            alignments: HashMap::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Record alignment for a pointer
    pub fn set_alignment(&mut self, ptr: &str, info: AlignmentInfo) {
        self.alignments.insert(ptr.to_string(), info);
    }

    /// Get alignment info for a pointer
    pub fn get_alignment(&self, ptr: &str) -> Option<&AlignmentInfo> {
        self.alignments.get(ptr)
    }

    /// Copy alignment from one pointer to another
    pub fn copy_alignment(&mut self, from: &str, to: &str) {
        if let Some(info) = self.alignments.get(from).cloned() {
            self.alignments.insert(to.to_string(), info);
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.alignments.clone());
    }

    /// Exit scope
    pub fn exit_scope(&mut self) {
        if let Some(prev) = self.scope_stack.pop() {
            self.alignments = prev;
        }
    }
}

/// Get the alignment requirement for a C++ type
fn get_type_alignment(type_name: &str) -> usize {
    let type_name = type_name.trim();

    // Pointers have their own alignment (8 on 64-bit)
    if type_name.ends_with('*') {
        return 8;
    }

    // Remove reference suffixes for alignment calculation
    let base_type = type_name
        .trim_end_matches('&')
        .trim();

    // Check for alignas attribute
    if let Some(alignas_start) = base_type.find("alignas(") {
        if let Some(alignas_end) = base_type[alignas_start..].find(')') {
            let num_str = &base_type[alignas_start + 8..alignas_start + alignas_end];
            if let Ok(alignment) = num_str.trim().parse::<usize>() {
                return alignment;
            }
        }
    }

    // Standard type alignments (assuming x86-64 / LP64)
    match base_type {
        "char" | "signed char" | "unsigned char" | "int8_t" | "uint8_t" | "bool" => 1,
        "short" | "signed short" | "unsigned short" | "int16_t" | "uint16_t" => 2,
        "int" | "signed int" | "unsigned int" | "int32_t" | "uint32_t" | "float" => 4,
        "long" | "signed long" | "unsigned long" | "long long" | "signed long long"
        | "unsigned long long" | "int64_t" | "uint64_t" | "double" | "size_t"
        | "ptrdiff_t" | "intptr_t" | "uintptr_t" => 8,
        "long double" => 16, // Platform dependent, 16 on x86-64
        "__m128" | "__m128i" | "__m128d" => 16,
        "__m256" | "__m256i" | "__m256d" => 32,
        "__m512" | "__m512i" | "__m512d" => 64,
        _ => {
            // Default to 1 for unknown types (be permissive)
            1
        }
    }
}

/// Check for alignment safety violations in a parsed function
pub fn check_alignment_safety(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    let mut tracker = AlignmentTracker::new();
    let mut unsafe_depth = 0;

    // Analyze each statement
    for stmt in &function.body {
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

        // Skip checking in unsafe blocks, but still track alignment
        analyze_statement_alignment(stmt, &mut tracker, &function.name, &mut errors, unsafe_depth > 0);
    }

    errors
}

/// Analyze a statement for alignment tracking and violations
fn analyze_statement_alignment(
    stmt: &Statement,
    tracker: &mut AlignmentTracker,
    func_name: &str,
    errors: &mut Vec<String>,
    in_unsafe: bool,
) {
    match stmt {
        Statement::VariableDecl(var) => {
            // Track alignment of declared variables
            if var.type_name.contains('*') || var.type_name.contains("char[") {
                let alignment = get_type_alignment(&var.type_name);
                let pointee_type = var.type_name.trim_end_matches('*').trim().to_string();
                tracker.set_alignment(&var.name, AlignmentInfo::aligned(alignment, pointee_type));
            }

            // Track arrays with alignas
            if var.type_name.contains('[') {
                let alignment = get_type_alignment(&var.type_name);
                let pointee_type = extract_array_element_type(&var.type_name);
                tracker.set_alignment(&var.name, AlignmentInfo::aligned(alignment, pointee_type));
            }
        }

        Statement::Assignment { lhs, rhs, .. } => {
            // Track alignment through assignments
            if let Some(ptr_name) = extract_var_name(lhs) {
                update_alignment_from_expr(&ptr_name, rhs, tracker);
            }

            // Check for alignment violations in casts
            if !in_unsafe {
                check_expr_alignment(rhs, tracker, func_name, errors);
            }
        }

        Statement::ExpressionStatement { expr, .. } => {
            if !in_unsafe {
                check_expr_alignment(expr, tracker, func_name, errors);
            }
        }

        Statement::If { condition, then_branch, else_branch, .. } => {
            if !in_unsafe {
                check_expr_alignment(condition, tracker, func_name, errors);
            }

            tracker.enter_scope();
            for stmt in then_branch {
                analyze_statement_alignment(stmt, tracker, func_name, errors, in_unsafe);
            }
            tracker.exit_scope();

            if let Some(else_stmts) = else_branch {
                tracker.enter_scope();
                for stmt in else_stmts {
                    analyze_statement_alignment(stmt, tracker, func_name, errors, in_unsafe);
                }
                tracker.exit_scope();
            }
        }

        Statement::Block(stmts) => {
            tracker.enter_scope();
            for stmt in stmts {
                analyze_statement_alignment(stmt, tracker, func_name, errors, in_unsafe);
            }
            tracker.exit_scope();
        }

        Statement::EnterScope => tracker.enter_scope(),
        Statement::ExitScope => tracker.exit_scope(),

        _ => {}
    }
}

/// Update alignment info based on an expression
fn update_alignment_from_expr(ptr: &str, expr: &Expression, tracker: &mut AlignmentTracker) {
    match expr {
        Expression::AddressOf(inner) => {
            // &x - alignment is based on the type of x
            if let Some(var_name) = extract_var_name(inner) {
                if let Some(info) = tracker.get_alignment(&var_name).cloned() {
                    tracker.set_alignment(ptr, info);
                } else {
                    // Default: assume natural alignment for the type
                    tracker.set_alignment(ptr, AlignmentInfo::aligned(8, "unknown".to_string()));
                }
            }
        }

        Expression::Variable(source) => {
            tracker.copy_alignment(source, ptr);
        }

        Expression::New(_) => {
            // new returns properly aligned memory for the requested type
            // Since we don't have the type info easily, assume maximum alignment
            tracker.set_alignment(ptr, AlignmentInfo::aligned(8, "unknown".to_string()));
        }

        Expression::PointerArithmetic { pointer, op: _ } => {
            // Pointer arithmetic changes offset - but we don't know by how much without the offset
            // For now, mark as potentially misaligned (offset unknown) for char* arithmetic
            if let Some(source) = extract_var_name(pointer) {
                if let Some(info) = tracker.get_alignment(&source).cloned() {
                    // For operations like p++, p--, p += n, the offset is unknown at static analysis time
                    // unless we can derive it from context. For now, mark as potentially misaligned.
                    let new_info = if info.pointee_type == "char" || info.pointee_type == "unsigned char" {
                        // Char pointer arithmetic can change alignment by 1 byte
                        AlignmentInfo {
                            base_alignment: info.base_alignment,
                            offset: 1, // Mark as potentially misaligned (offset 1)
                            pointee_type: info.pointee_type.clone(),
                        }
                    } else {
                        // For typed pointers, arithmetic maintains alignment if offset is element-sized
                        info.clone()
                    };
                    tracker.set_alignment(ptr, new_info);
                }
            }
        }

        Expression::Cast { inner, target_type, .. } => {
            // Cast may change the perceived type but the underlying alignment stays
            if let Some(source) = extract_var_name(inner) {
                if let Some(info) = tracker.get_alignment(&source).cloned() {
                    let new_type = target_type.clone().unwrap_or_else(|| info.pointee_type.clone());
                    let new_info = AlignmentInfo {
                        base_alignment: info.base_alignment,
                        offset: info.offset,
                        pointee_type: new_type.trim_end_matches('*').trim().to_string(),
                    };
                    tracker.set_alignment(ptr, new_info);
                }
            } else {
                // If source is not a tracked variable, derive from the expression
                update_alignment_from_expr(ptr, inner, tracker);
            }
        }

        _ => {}
    }
}

/// Check an expression for alignment violations
fn check_expr_alignment(
    expr: &Expression,
    tracker: &AlignmentTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::Cast { inner, kind, target_type } => {
            // Check for potentially misaligned casts
            if matches!(kind, CastKind::ReinterpretCast | CastKind::CStyleCast) {
                if let Some(target) = target_type {
                    if target.contains('*') {
                        // This is a pointer cast - check alignment
                        let target_pointee = target.trim_end_matches('*').trim();
                        let target_alignment = get_type_alignment(target_pointee);

                        if let Some(source_name) = extract_var_name(inner) {
                            if let Some(info) = tracker.get_alignment(&source_name) {
                                if !info.is_aligned_for(target_alignment) {
                                    errors.push(format!(
                                        "In function '{}': Cast to '{}' may create misaligned pointer \
                                        (source has alignment {} with offset {}, target requires {})",
                                        func_name, target, info.base_alignment, info.offset, target_alignment
                                    ));
                                }
                            }
                        }

                        // Check for pointer arithmetic in the cast source
                        check_arithmetic_alignment_in_cast(inner, target_alignment, tracker, func_name, errors);
                    }
                }
            }

            // Recursively check inner expression
            check_expr_alignment(inner, tracker, func_name, errors);
        }

        Expression::Dereference(inner) => {
            // Dereferencing a misaligned pointer is UB
            if let Some(ptr_name) = extract_var_name(inner) {
                if let Some(info) = tracker.get_alignment(&ptr_name) {
                    let required_alignment = get_type_alignment(&info.pointee_type);
                    if !info.is_aligned_for(required_alignment) {
                        errors.push(format!(
                            "In function '{}': Dereferencing potentially misaligned pointer '{}' \
                            (alignment {}, offset {}, type '{}' requires alignment {})",
                            func_name, ptr_name, info.base_alignment, info.offset,
                            info.pointee_type, required_alignment
                        ));
                    }
                }
            }
            check_expr_alignment(inner, tracker, func_name, errors);
        }

        Expression::BinaryOp { left, right, .. } => {
            check_expr_alignment(left, tracker, func_name, errors);
            check_expr_alignment(right, tracker, func_name, errors);
        }

        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expr_alignment(arg, tracker, func_name, errors);
            }
        }

        Expression::MemberAccess { object, .. } => {
            check_expr_alignment(object, tracker, func_name, errors);
        }

        _ => {}
    }
}

/// Check for pointer arithmetic that may cause misalignment in a cast
fn check_arithmetic_alignment_in_cast(
    expr: &Expression,
    target_alignment: usize,
    tracker: &AlignmentTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::PointerArithmetic { pointer, op: _ } => {
            // Check if the arithmetic could cause misalignment
            if let Some(source) = extract_var_name(pointer) {
                if let Some(info) = tracker.get_alignment(&source) {
                    // If we're doing arithmetic on a char* and casting to a stricter type
                    if info.pointee_type == "char" || info.pointee_type == "unsigned char"
                       || info.pointee_type == "signed char" || info.pointee_type == "void" {
                        // Pointer arithmetic on char* before cast to stricter alignment is suspicious
                        if target_alignment > 1 {
                            errors.push(format!(
                                "In function '{}': Pointer arithmetic on char/void pointer \
                                may cause misalignment when cast to type requiring {} byte alignment",
                                func_name, target_alignment
                            ));
                        }
                    }
                }
            }
        }
        Expression::BinaryOp { left, op, right } if op == "+" => {
            // Binary addition used for pointer arithmetic
            check_arithmetic_alignment_in_cast(left, target_alignment, tracker, func_name, errors);
            check_arithmetic_alignment_in_cast(right, target_alignment, tracker, func_name, errors);
        }
        _ => {}
    }
}

/// Extract variable name from an expression
fn extract_var_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::Dereference(inner) => extract_var_name(inner),
        Expression::Cast { inner, .. } => extract_var_name(inner),
        Expression::Move { inner, .. } => extract_var_name(inner),
        Expression::PointerArithmetic { pointer, .. } => extract_var_name(pointer),
        _ => None,
    }
}

/// Extract element type from array type
fn extract_array_element_type(type_name: &str) -> String {
    // "int[10]" -> "int", "alignas(8) char[64]" -> "char"
    if let Some(bracket_pos) = type_name.find('[') {
        type_name[..bracket_pos].trim().to_string()
    } else {
        type_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_alignment() {
        assert_eq!(get_type_alignment("char"), 1);
        assert_eq!(get_type_alignment("short"), 2);
        assert_eq!(get_type_alignment("int"), 4);
        assert_eq!(get_type_alignment("long"), 8);
        assert_eq!(get_type_alignment("double"), 8);
        assert_eq!(get_type_alignment("int*"), 8);
    }

    #[test]
    fn test_alignment_info_is_aligned() {
        let info = AlignmentInfo::aligned(8, "int64_t".to_string());
        assert!(info.is_aligned_for(1));
        assert!(info.is_aligned_for(2));
        assert!(info.is_aligned_for(4));
        assert!(info.is_aligned_for(8));

        let info2 = AlignmentInfo::with_offset(8, 4, "int".to_string());
        assert!(info2.is_aligned_for(1));
        assert!(info2.is_aligned_for(2));
        assert!(info2.is_aligned_for(4));
        assert!(!info2.is_aligned_for(8)); // offset 4 not aligned for 8
    }

    #[test]
    fn test_alignment_offset_arithmetic() {
        let info = AlignmentInfo::aligned(8, "char".to_string());
        let new_info = info.add_offset(1, "char".to_string());
        assert_eq!(new_info.offset, 1);

        let new_info2 = info.add_offset(8, "char".to_string());
        assert_eq!(new_info2.offset, 0); // Wraps around
    }

    #[test]
    fn test_tracker_basic() {
        let mut tracker = AlignmentTracker::new();
        tracker.set_alignment("p", AlignmentInfo::aligned(8, "int64_t".to_string()));

        assert!(tracker.get_alignment("p").is_some());
        assert!(tracker.get_alignment("q").is_none());
    }

    #[test]
    fn test_tracker_copy() {
        let mut tracker = AlignmentTracker::new();
        tracker.set_alignment("p", AlignmentInfo::aligned(8, "int64_t".to_string()));
        tracker.copy_alignment("p", "q");

        assert!(tracker.get_alignment("q").is_some());
        assert_eq!(tracker.get_alignment("q").unwrap().base_alignment, 8);
    }

    #[test]
    fn test_tracker_scope() {
        let mut tracker = AlignmentTracker::new();
        tracker.set_alignment("p", AlignmentInfo::aligned(8, "int64_t".to_string()));

        tracker.enter_scope();
        tracker.set_alignment("q", AlignmentInfo::aligned(4, "int".to_string()));
        assert!(tracker.get_alignment("q").is_some());

        tracker.exit_scope();
        // q should be gone after exiting scope
        // Note: Our implementation preserves outer scope variables
        assert!(tracker.get_alignment("p").is_some());
    }
}
