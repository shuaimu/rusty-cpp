//! Array Bounds Safety Analysis (Phase 3 of Full Pointer Safety)
//!
//! This module tracks array sizes and prevents out-of-bounds access.
//!
//! Part A: Static Array Bounds
//! - Track array declarations and their sizes
//! - Track bounds through pointer arithmetic
//! - At subscript, verify index against bounds
//! - Analyze loop bounds for common patterns
//!
//! Part B: Dynamic Bounds with rusty::Span<T>
//! - Recognize Span type and track its size
//! - Verify bounds-checked access patterns
//!
//! Example:
//! ```cpp
//! int arr[10];
//! arr[9];   // OK: 9 < 10
//! arr[10];  // ERROR: Index 10 out of bounds [0, 10)
//! ```

use crate::parser::{Statement, Expression, Function};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Bounds information for an array or pointer
#[derive(Debug, Clone)]
pub struct BoundsInfo {
    /// The size of the array (number of elements)
    pub size: Option<usize>,
    /// Current offset from the start (for pointer arithmetic)
    pub offset: usize,
    /// Whether this is a known array or derived pointer
    pub is_array: bool,
    /// The original array variable (for derived pointers)
    pub source_array: Option<String>,
}

impl BoundsInfo {
    /// Create bounds info for an array with known size
    pub fn array(size: usize) -> Self {
        BoundsInfo {
            size: Some(size),
            offset: 0,
            is_array: true,
            source_array: None,
        }
    }

    /// Create bounds info for a pointer derived from an array
    pub fn derived(source: &str, size: Option<usize>, offset: usize) -> Self {
        BoundsInfo {
            size,
            offset,
            is_array: false,
            source_array: Some(source.to_string()),
        }
    }

    /// Create bounds info with unknown size
    pub fn unknown() -> Self {
        BoundsInfo {
            size: None,
            offset: 0,
            is_array: false,
            source_array: None,
        }
    }

    /// Check if an index is within bounds
    pub fn is_in_bounds(&self, index: usize) -> bool {
        match self.size {
            Some(size) => {
                let effective_index = self.offset + index;
                effective_index < size
            }
            None => true, // Unknown size - allow (we can't prove it's wrong)
        }
    }

    /// Get the remaining bounds after current offset
    pub fn remaining_bounds(&self) -> Option<usize> {
        self.size.map(|s| s.saturating_sub(self.offset))
    }

    /// Apply pointer arithmetic (addition)
    pub fn add_offset(&self, amount: usize) -> Self {
        BoundsInfo {
            size: self.size,
            offset: self.offset + amount,
            is_array: false,
            source_array: self.source_array.clone(),
        }
    }

    /// Apply pointer arithmetic (subtraction)
    pub fn sub_offset(&self, amount: usize) -> Self {
        BoundsInfo {
            size: self.size,
            offset: self.offset.saturating_sub(amount),
            is_array: false,
            source_array: self.source_array.clone(),
        }
    }
}

/// Tracks array bounds
#[derive(Debug)]
pub struct BoundsTracker {
    /// Maps variable name to its bounds info
    bounds: HashMap<String, BoundsInfo>,
    /// Scope stack for nested blocks
    scope_stack: Vec<HashMap<String, BoundsInfo>>,
}

impl BoundsTracker {
    pub fn new() -> Self {
        BoundsTracker {
            bounds: HashMap::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Record bounds for a variable
    pub fn set_bounds(&mut self, var: &str, bounds: BoundsInfo) {
        self.bounds.insert(var.to_string(), bounds);
    }

    /// Get bounds for a variable
    pub fn get_bounds(&self, var: &str) -> Option<&BoundsInfo> {
        self.bounds.get(var)
    }

    /// Copy bounds from one variable to another
    pub fn copy_bounds(&mut self, from: &str, to: &str) {
        if let Some(bounds) = self.bounds.get(from).cloned() {
            let derived = BoundsInfo::derived(
                bounds.source_array.as_deref().unwrap_or(from),
                bounds.size,
                bounds.offset,
            );
            self.bounds.insert(to.to_string(), derived);
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.bounds.clone());
    }

    /// Exit scope
    pub fn exit_scope(&mut self) {
        if let Some(prev) = self.scope_stack.pop() {
            self.bounds = prev;
        }
    }
}

/// Check for array bounds violations in a parsed function
pub fn check_array_bounds(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    let mut tracker = BoundsTracker::new();
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

        // Skip checking in unsafe blocks, but still track bounds
        analyze_statement_bounds(stmt, &mut tracker, &function.name, &mut errors, unsafe_depth > 0);
    }

    errors
}

/// Analyze a statement for bounds tracking and violations
fn analyze_statement_bounds(
    stmt: &Statement,
    tracker: &mut BoundsTracker,
    func_name: &str,
    errors: &mut Vec<String>,
    in_unsafe: bool,
) {
    match stmt {
        Statement::VariableDecl(var) => {
            // Track array declarations
            if let Some(size) = extract_array_size(&var.type_name) {
                tracker.set_bounds(&var.name, BoundsInfo::array(size));
            }
            // Track rusty::Span declarations
            else if var.type_name.contains("Span<") || var.type_name.contains("span<") {
                // Span carries its own size - we'll track it when assigned
                tracker.set_bounds(&var.name, BoundsInfo::unknown());
            }
        }

        Statement::Assignment { lhs, rhs, .. } => {
            // Track bounds through assignments
            if let Some(ptr_name) = extract_var_name(lhs) {
                update_bounds_from_expr(&ptr_name, rhs, tracker);
            }

            // Check for bounds violations in array accesses
            if !in_unsafe {
                check_expr_bounds(rhs, tracker, func_name, errors);
            }
        }

        Statement::ExpressionStatement { expr, .. } => {
            if !in_unsafe {
                check_expr_bounds(expr, tracker, func_name, errors);
            }
        }

        Statement::Return(Some(expr)) => {
            if !in_unsafe {
                check_expr_bounds(expr, tracker, func_name, errors);
            }
        }

        Statement::If { condition, then_branch, else_branch, .. } => {
            if !in_unsafe {
                check_expr_bounds(condition, tracker, func_name, errors);
            }

            tracker.enter_scope();
            for stmt in then_branch {
                analyze_statement_bounds(stmt, tracker, func_name, errors, in_unsafe);
            }
            tracker.exit_scope();

            if let Some(else_stmts) = else_branch {
                tracker.enter_scope();
                for stmt in else_stmts {
                    analyze_statement_bounds(stmt, tracker, func_name, errors, in_unsafe);
                }
                tracker.exit_scope();
            }
        }

        // EnterLoop/ExitLoop markers are handled through scope tracking
        Statement::EnterLoop => {
            tracker.enter_scope();
        }

        Statement::ExitLoop => {
            tracker.exit_scope();
        }

        Statement::Block(stmts) => {
            tracker.enter_scope();
            for stmt in stmts {
                analyze_statement_bounds(stmt, tracker, func_name, errors, in_unsafe);
            }
            tracker.exit_scope();
        }

        Statement::EnterScope => tracker.enter_scope(),
        Statement::ExitScope => tracker.exit_scope(),

        _ => {}
    }
}

/// Update bounds based on an expression
fn update_bounds_from_expr(ptr: &str, expr: &Expression, tracker: &mut BoundsTracker) {
    match expr {
        Expression::Variable(source) => {
            tracker.copy_bounds(source, ptr);
        }

        Expression::AddressOf(inner) => {
            // &arr[i] - pointer to element, bounds from i onwards
            if let Expression::ArraySubscript { array, index } = inner.as_ref() {
                if let Some(arr_name) = extract_var_name(array) {
                    if let Some(bounds) = tracker.get_bounds(&arr_name).cloned() {
                        if let Some(idx) = extract_constant_value(index) {
                            let derived = bounds.add_offset(idx as usize);
                            tracker.set_bounds(ptr, derived);
                        }
                    }
                }
            }
        }

        Expression::PointerArithmetic { pointer, .. } => {
            // For now, just copy the source bounds
            // Full tracking would require knowing the offset amount
            if let Some(source) = extract_var_name(pointer) {
                tracker.copy_bounds(&source, ptr);
            }
        }

        Expression::New(_) => {
            // new T - unknown size (single element)
            tracker.set_bounds(ptr, BoundsInfo::array(1));
        }

        _ => {}
    }
}

/// Check an expression for bounds violations
fn check_expr_bounds(
    expr: &Expression,
    tracker: &BoundsTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::ArraySubscript { array, index } => {
            // Check if the index is within bounds
            if let Some(arr_name) = extract_var_name(array) {
                if let Some(bounds) = tracker.get_bounds(&arr_name) {
                    // Try to get a constant index
                    if let Some(idx) = extract_constant_value(index) {
                        if idx < 0 {
                            errors.push(format!(
                                "In function '{}': Array index {} is negative for array '{}'",
                                func_name, idx, arr_name
                            ));
                        } else if !bounds.is_in_bounds(idx as usize) {
                            if let Some(size) = bounds.size {
                                let remaining = bounds.remaining_bounds().unwrap_or(0);
                                errors.push(format!(
                                    "In function '{}': Array index {} out of bounds for array '{}' \
                                    (size {}, offset {}, remaining {})",
                                    func_name, idx, arr_name, size, bounds.offset, remaining
                                ));
                            }
                        }
                    }
                    // For non-constant indices, we can only detect at runtime
                    // Future: Add loop variable tracking for better static analysis
                }
            }

            // Recursively check sub-expressions
            check_expr_bounds(array, tracker, func_name, errors);
            check_expr_bounds(index, tracker, func_name, errors);
        }

        Expression::BinaryOp { left, right, .. } => {
            check_expr_bounds(left, tracker, func_name, errors);
            check_expr_bounds(right, tracker, func_name, errors);
        }

        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expr_bounds(arg, tracker, func_name, errors);
            }
        }

        Expression::MemberAccess { object, field, .. } => {
            check_expr_bounds(object, tracker, func_name, errors);

            // Check for Span.size() usage
            if field == "size" || field == "size()" {
                // This is getting the size - no bounds check needed
            }
            // Check for Span[] access - handled by ArraySubscript
        }

        Expression::Dereference(inner) => {
            check_expr_bounds(inner, tracker, func_name, errors);
        }

        Expression::Cast { inner, .. } => {
            check_expr_bounds(inner, tracker, func_name, errors);
        }

        _ => {}
    }
}

/// Extract array size from type string
fn extract_array_size(type_name: &str) -> Option<usize> {
    // Match patterns like "int[10]", "char[256]", "int arr[10]"
    if let Some(bracket_start) = type_name.find('[') {
        if let Some(bracket_end) = type_name[bracket_start..].find(']') {
            let size_str = &type_name[bracket_start + 1..bracket_start + bracket_end];
            if let Ok(size) = size_str.trim().parse::<usize>() {
                return Some(size);
            }
        }
    }
    None
}

/// Extract constant value from an expression
fn extract_constant_value(expr: &Expression) -> Option<i64> {
    match expr {
        Expression::Literal(lit) => {
            lit.parse::<i64>().ok()
        }
        _ => None,
    }
}

/// Extract variable name from an expression
fn extract_var_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::Dereference(inner) => extract_var_name(inner),
        Expression::Cast { inner, .. } => extract_var_name(inner),
        Expression::Move { inner, .. } => extract_var_name(inner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_info_in_bounds() {
        let bounds = BoundsInfo::array(10);
        assert!(bounds.is_in_bounds(0));
        assert!(bounds.is_in_bounds(9));
        assert!(!bounds.is_in_bounds(10));
        assert!(!bounds.is_in_bounds(11));
    }

    #[test]
    fn test_bounds_info_with_offset() {
        let bounds = BoundsInfo::array(10);
        let offset_bounds = bounds.add_offset(5);
        assert!(offset_bounds.is_in_bounds(0)); // Actually index 5
        assert!(offset_bounds.is_in_bounds(4)); // Actually index 9
        assert!(!offset_bounds.is_in_bounds(5)); // Actually index 10 - out of bounds
    }

    #[test]
    fn test_extract_array_size() {
        assert_eq!(extract_array_size("int[10]"), Some(10));
        assert_eq!(extract_array_size("char[256]"), Some(256));
        assert_eq!(extract_array_size("int arr[5]"), Some(5));
        assert_eq!(extract_array_size("int*"), None);
        assert_eq!(extract_array_size("int"), None);
    }

    #[test]
    fn test_bounds_tracker_basic() {
        let mut tracker = BoundsTracker::new();
        tracker.set_bounds("arr", BoundsInfo::array(10));

        assert!(tracker.get_bounds("arr").is_some());
        assert_eq!(tracker.get_bounds("arr").unwrap().size, Some(10));
    }

    #[test]
    fn test_bounds_tracker_copy() {
        let mut tracker = BoundsTracker::new();
        tracker.set_bounds("arr", BoundsInfo::array(10));
        tracker.copy_bounds("arr", "p");

        assert!(tracker.get_bounds("p").is_some());
        assert_eq!(tracker.get_bounds("p").unwrap().size, Some(10));
    }

    #[test]
    fn test_bounds_tracker_scope() {
        let mut tracker = BoundsTracker::new();
        tracker.set_bounds("arr", BoundsInfo::array(10));

        tracker.enter_scope();
        tracker.set_bounds("arr2", BoundsInfo::array(20));
        assert!(tracker.get_bounds("arr2").is_some());

        tracker.exit_scope();
        assert!(tracker.get_bounds("arr").is_some());
    }

    #[test]
    fn test_remaining_bounds() {
        let bounds = BoundsInfo::array(10);
        assert_eq!(bounds.remaining_bounds(), Some(10));

        let offset_bounds = bounds.add_offset(3);
        assert_eq!(offset_bounds.remaining_bounds(), Some(7));
    }
}
