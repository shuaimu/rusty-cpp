//! Pointer Provenance Analysis (Phase 5 of Full Pointer Safety)
//!
//! This module tracks the allocation origin of pointers to prevent undefined
//! behavior from comparing or subtracting pointers to different allocations.
//!
//! Rules:
//! - Pointer subtraction (p - q) requires same allocation origin
//! - Relational comparison (<, >, <=, >=) requires same allocation origin
//! - Equality comparison (==, !=) is allowed between any pointers
//!
//! Each allocation (stack variable, array, new expression) gets a unique ID
//! that is tracked through pointer assignments and arithmetic.

use crate::parser::{Statement, Expression, Function};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Unique identifier for an allocation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AllocationId {
    /// Stack variable with the given name
    StackVar(String),
    /// Array with the given name
    Array(String),
    /// Heap allocation (from new expression)
    Heap(usize),
    /// Unknown provenance (e.g., from function return)
    Unknown,
}

/// Tracks pointer provenance
#[derive(Debug, Clone)]
pub struct ProvenanceTracker {
    /// Maps pointer variable to its allocation origin
    provenance: HashMap<String, AllocationId>,
    /// Counter for heap allocations
    heap_counter: usize,
    /// Scope stack
    scope_stack: Vec<HashMap<String, AllocationId>>,
}

impl ProvenanceTracker {
    pub fn new() -> Self {
        ProvenanceTracker {
            provenance: HashMap::new(),
            heap_counter: 0,
            scope_stack: Vec::new(),
        }
    }

    /// Record that a pointer points to a stack variable
    pub fn set_stack_provenance(&mut self, ptr: &str, var: &str) {
        self.provenance.insert(ptr.to_string(), AllocationId::StackVar(var.to_string()));
    }

    /// Record that a pointer points to an array
    pub fn set_array_provenance(&mut self, ptr: &str, array: &str) {
        self.provenance.insert(ptr.to_string(), AllocationId::Array(array.to_string()));
    }

    /// Record that a pointer comes from a new expression
    pub fn set_heap_provenance(&mut self, ptr: &str) -> usize {
        let id = self.heap_counter;
        self.heap_counter += 1;
        self.provenance.insert(ptr.to_string(), AllocationId::Heap(id));
        id
    }

    /// Copy provenance from one pointer to another
    pub fn copy_provenance(&mut self, from: &str, to: &str) {
        if let Some(prov) = self.provenance.get(from).cloned() {
            self.provenance.insert(to.to_string(), prov);
        }
    }

    /// Get the provenance of a pointer
    pub fn get_provenance(&self, ptr: &str) -> Option<&AllocationId> {
        self.provenance.get(ptr)
    }

    /// Check if two pointers have the same provenance
    pub fn same_provenance(&self, p1: &str, p2: &str) -> bool {
        match (self.provenance.get(p1), self.provenance.get(p2)) {
            (Some(a), Some(b)) => a == b,
            _ => false, // Unknown provenance - be conservative
        }
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.provenance.clone());
    }

    /// Exit scope
    pub fn exit_scope(&mut self) {
        if let Some(prev) = self.scope_stack.pop() {
            self.provenance = prev;
        }
    }
}

/// Check for pointer provenance violations in a parsed function
pub fn check_pointer_provenance(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    let mut tracker = ProvenanceTracker::new();
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

        // Skip checking in unsafe blocks
        if unsafe_depth > 0 {
            continue;
        }

        analyze_statement_provenance(stmt, &mut tracker, &function.name, &mut errors);
    }

    errors
}

/// Analyze a statement for provenance tracking
fn analyze_statement_provenance(
    stmt: &Statement,
    tracker: &mut ProvenanceTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Statement::VariableDecl(var) => {
            // Arrays get their own provenance
            if var.type_name.contains('[') || var.type_name.contains("[]") {
                tracker.set_array_provenance(&var.name, &var.name);
            }
            // Pointers start with unknown provenance unless assigned
        }

        Statement::Assignment { lhs, rhs, .. } => {
            // Track provenance through assignments
            if let Some(ptr_name) = extract_var_name(lhs) {
                update_provenance_from_expr(&ptr_name, rhs, tracker);
            }

            // Check for provenance violations in binary operations
            check_expr_provenance(rhs, tracker, func_name, errors);
        }

        Statement::ReferenceBinding { name, target, .. } => {
            // References inherit provenance from their target
            if let Some(target_name) = extract_var_name(target) {
                tracker.copy_provenance(&target_name, name);
            }
        }

        Statement::ExpressionStatement { expr, .. } => {
            check_expr_provenance(expr, tracker, func_name, errors);
        }

        Statement::Return(Some(expr)) => {
            check_expr_provenance(expr, tracker, func_name, errors);
        }

        Statement::If { condition, then_branch, else_branch, .. } => {
            check_expr_provenance(condition, tracker, func_name, errors);

            tracker.enter_scope();
            for stmt in then_branch {
                analyze_statement_provenance(stmt, tracker, func_name, errors);
            }
            tracker.exit_scope();

            if let Some(else_stmts) = else_branch {
                tracker.enter_scope();
                for stmt in else_stmts {
                    analyze_statement_provenance(stmt, tracker, func_name, errors);
                }
                tracker.exit_scope();
            }
        }

        Statement::Block(stmts) => {
            tracker.enter_scope();
            for stmt in stmts {
                analyze_statement_provenance(stmt, tracker, func_name, errors);
            }
            tracker.exit_scope();
        }

        Statement::EnterScope => tracker.enter_scope(),
        Statement::ExitScope => tracker.exit_scope(),

        _ => {}
    }
}

/// Update provenance based on an expression
fn update_provenance_from_expr(ptr: &str, expr: &Expression, tracker: &mut ProvenanceTracker) {
    match expr {
        Expression::AddressOf(inner) => {
            if let Some(var_name) = extract_var_name(inner) {
                tracker.set_stack_provenance(ptr, &var_name);
            }
        }

        Expression::Variable(source) => {
            tracker.copy_provenance(source, ptr);
        }

        Expression::New(_) => {
            tracker.set_heap_provenance(ptr);
        }

        Expression::PointerArithmetic { pointer, .. } => {
            if let Some(source) = extract_var_name(pointer) {
                tracker.copy_provenance(&source, ptr);
            }
        }

        Expression::Cast { inner, .. } => {
            update_provenance_from_expr(ptr, inner, tracker);
        }

        _ => {
            // Unknown source - could add more cases
        }
    }
}

/// Check an expression for provenance violations
fn check_expr_provenance(
    expr: &Expression,
    tracker: &ProvenanceTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::BinaryOp { left, op, right } => {
            // Check for pointer subtraction
            if op == "-" || op == "pointer difference" {
                if let (Some(p1), Some(p2)) = (extract_var_name(left), extract_var_name(right)) {
                    // Check if both are pointers with tracked provenance
                    if tracker.get_provenance(&p1).is_some() && tracker.get_provenance(&p2).is_some() {
                        if !tracker.same_provenance(&p1, &p2) {
                            errors.push(format!(
                                "In function '{}': Pointer subtraction between '{}' and '{}' with different allocations is undefined behavior",
                                func_name, p1, p2
                            ));
                        }
                    }
                }
            }

            // Check for relational comparisons
            if op == "<" || op == ">" || op == "<=" || op == ">=" {
                if let (Some(p1), Some(p2)) = (extract_var_name(left), extract_var_name(right)) {
                    if tracker.get_provenance(&p1).is_some() && tracker.get_provenance(&p2).is_some() {
                        if !tracker.same_provenance(&p1, &p2) {
                            errors.push(format!(
                                "In function '{}': Relational comparison between pointers '{}' and '{}' with different allocations is undefined behavior",
                                func_name, p1, p2
                            ));
                        }
                    }
                }
            }

            // Recursively check sub-expressions
            check_expr_provenance(left, tracker, func_name, errors);
            check_expr_provenance(right, tracker, func_name, errors);
        }

        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expr_provenance(arg, tracker, func_name, errors);
            }
        }

        Expression::MemberAccess { object, .. } => {
            check_expr_provenance(object, tracker, func_name, errors);
        }

        Expression::Dereference(inner) => {
            check_expr_provenance(inner, tracker, func_name, errors);
        }

        Expression::Cast { inner, .. } => {
            check_expr_provenance(inner, tracker, func_name, errors);
        }

        Expression::Move { inner, .. } => {
            check_expr_provenance(inner, tracker, func_name, errors);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_same_allocation() {
        let mut tracker = ProvenanceTracker::new();
        tracker.set_stack_provenance("p1", "arr");
        tracker.set_stack_provenance("p2", "arr");
        assert!(tracker.same_provenance("p1", "p2"));
    }

    #[test]
    fn test_provenance_different_allocation() {
        let mut tracker = ProvenanceTracker::new();
        tracker.set_stack_provenance("p1", "arr1");
        tracker.set_stack_provenance("p2", "arr2");
        assert!(!tracker.same_provenance("p1", "p2"));
    }

    #[test]
    fn test_provenance_copy() {
        let mut tracker = ProvenanceTracker::new();
        tracker.set_stack_provenance("p1", "arr");
        tracker.copy_provenance("p1", "p2");
        assert!(tracker.same_provenance("p1", "p2"));
    }

    #[test]
    fn test_heap_provenance() {
        let mut tracker = ProvenanceTracker::new();
        let id1 = tracker.set_heap_provenance("p1");
        let id2 = tracker.set_heap_provenance("p2");
        assert_ne!(id1, id2);
        assert!(!tracker.same_provenance("p1", "p2"));
    }
}
