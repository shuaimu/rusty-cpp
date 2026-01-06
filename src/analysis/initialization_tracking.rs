//! Initialization Tracking Analysis (Phase 2 of Full Pointer Safety)
//!
//! This module implements definite assignment analysis to ensure pointers
//! only reference initialized memory in @safe code.
//!
//! States:
//! - Uninitialized: Declared but not assigned
//! - Initialized: Has been assigned a value
//! - MaybeUninitialized: Assigned in some paths but not all
//!
//! Rules:
//! - Taking address of uninitialized variable is flagged
//! - Dereferencing pointer to uninitialized memory is an error

use crate::parser::{Statement, Expression, Function};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Represents the initialization state of a variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitState {
    /// Declared but not yet assigned
    Uninitialized,
    /// Has been assigned a value
    Initialized,
    /// Assigned in some paths but not all (e.g., if without else)
    MaybeUninitialized,
}

impl InitState {
    /// Merge two states (for control flow join points)
    pub fn merge(self, other: InitState) -> InitState {
        match (self, other) {
            (InitState::Initialized, InitState::Initialized) => InitState::Initialized,
            (InitState::Uninitialized, InitState::Uninitialized) => InitState::Uninitialized,
            _ => InitState::MaybeUninitialized,
        }
    }
}

/// Tracks initialization state for all variables in scope
#[derive(Debug, Clone)]
pub struct InitTracker {
    /// Maps variable name to its initialization state
    states: HashMap<String, InitState>,
    /// Maps pointer variable to what it points to (for indirect access)
    points_to: HashMap<String, String>,
    /// Stack of scopes for handling blocks
    scope_stack: Vec<HashMap<String, InitState>>,
}

impl InitTracker {
    pub fn new() -> Self {
        InitTracker {
            states: HashMap::new(),
            points_to: HashMap::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Declare a variable (initially uninitialized unless has initializer)
    pub fn declare(&mut self, var: &str, initialized: bool) {
        let state = if initialized { InitState::Initialized } else { InitState::Uninitialized };
        self.states.insert(var.to_string(), state);
    }

    /// Mark a variable as initialized
    pub fn initialize(&mut self, var: &str) {
        self.states.insert(var.to_string(), InitState::Initialized);
    }

    /// Get the initialization state of a variable
    pub fn get_state(&self, var: &str) -> InitState {
        self.states.get(var).copied().unwrap_or(InitState::Initialized)
    }

    /// Record that a pointer points to a variable
    pub fn set_points_to(&mut self, ptr: &str, target: &str) {
        self.points_to.insert(ptr.to_string(), target.to_string());
    }

    /// Get what a pointer points to
    pub fn get_points_to(&self, ptr: &str) -> Option<&String> {
        self.points_to.get(ptr)
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.states.clone());
    }

    /// Exit scope
    pub fn exit_scope(&mut self) {
        if let Some(previous) = self.scope_stack.pop() {
            self.states = previous;
        }
    }

    /// Create a snapshot for branch analysis
    pub fn snapshot(&self) -> InitTracker {
        InitTracker {
            states: self.states.clone(),
            points_to: self.points_to.clone(),
            scope_stack: Vec::new(),
        }
    }

    /// Merge states from another tracker
    pub fn merge_branch(&mut self, other: &InitTracker) {
        for (var, state) in &other.states {
            let current = self.get_state(var);
            self.states.insert(var.clone(), current.merge(*state));
        }
    }
}

/// Check for initialization safety violations in a parsed function
pub fn check_initialization_safety(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    let mut tracker = InitTracker::new();
    let mut unsafe_depth = 0;

    // Parameters are always initialized (passed by caller)
    for param in &function.parameters {
        tracker.declare(&param.name, true);
    }

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

        analyze_statement_init(&stmt, &mut tracker, &function.name, &mut errors);
    }

    errors
}

/// Analyze a statement for initialization violations
fn analyze_statement_init(
    stmt: &Statement,
    tracker: &mut InitTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Statement::VariableDecl(var) => {
            // Variable declarations
            // Check if there's an initializer by looking at the type context
            // For now, assume variables with types that suggest initialization are initialized
            // (e.g., "int x = 5" vs "int x;")

            // Heuristic: if the variable is a reference or has a complex type, assume initialized
            // Otherwise, mark as uninitialized
            let is_initialized = var.is_reference ||
                                 var.type_name.contains('=') ||
                                 !var.type_name.is_empty() && var.is_const;

            tracker.declare(&var.name, is_initialized);
        }

        Statement::Assignment { lhs, rhs, .. } => {
            // Check RHS for uninitialized variable access
            check_expr_init(rhs, tracker, func_name, errors);

            // Mark LHS as initialized
            if let Some(var_name) = extract_var_name(lhs) {
                tracker.initialize(&var_name);
            }

            // Track pointer assignments
            if let Expression::Variable(ptr_name) = lhs {
                if let Expression::AddressOf(target) = rhs {
                    if let Some(target_name) = extract_var_name(target) {
                        tracker.set_points_to(ptr_name, &target_name);
                    }
                }
            }
        }

        Statement::ReferenceBinding { name, target, .. } => {
            // Reference bindings
            check_expr_init(target, tracker, func_name, errors);

            // Track what the reference points to
            if let Some(target_name) = extract_var_name(target) {
                tracker.set_points_to(name, &target_name);
                // Also mark the binding itself as initialized
                tracker.declare(name, true);
            }
        }

        Statement::FunctionCall { args, .. } => {
            for arg in args {
                check_expr_init(arg, tracker, func_name, errors);
            }
        }

        Statement::Return(Some(expr)) => {
            check_expr_init(expr, tracker, func_name, errors);
        }

        Statement::ExpressionStatement { expr, .. } => {
            check_expr_init(expr, tracker, func_name, errors);
        }

        Statement::If { condition, then_branch, else_branch, .. } => {
            check_expr_init(condition, tracker, func_name, errors);

            // Analyze branches
            let mut then_tracker = tracker.snapshot();
            for stmt in then_branch {
                analyze_statement_init(stmt, &mut then_tracker, func_name, errors);
            }

            let mut else_tracker = tracker.snapshot();
            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    analyze_statement_init(stmt, &mut else_tracker, func_name, errors);
                }
            }

            // Merge branch states
            tracker.merge_branch(&then_tracker);
            if else_branch.is_some() {
                tracker.merge_branch(&else_tracker);
            }
        }

        Statement::Block(stmts) => {
            tracker.enter_scope();
            for stmt in stmts {
                analyze_statement_init(stmt, tracker, func_name, errors);
            }
            tracker.exit_scope();
        }

        Statement::EnterScope => {
            tracker.enter_scope();
        }

        Statement::ExitScope => {
            tracker.exit_scope();
        }

        _ => {}
    }
}

/// Check an expression for use of uninitialized variables
fn check_expr_init(
    expr: &Expression,
    tracker: &InitTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::Variable(name) => {
            let state = tracker.get_state(name);
            match state {
                InitState::Uninitialized => {
                    errors.push(format!(
                        "In function '{}': Use of uninitialized variable '{}'",
                        func_name, name
                    ));
                }
                InitState::MaybeUninitialized => {
                    errors.push(format!(
                        "In function '{}': Use of potentially uninitialized variable '{}' - assign in all branches",
                        func_name, name
                    ));
                }
                InitState::Initialized => {}
            }
        }

        Expression::AddressOf(inner) => {
            // Taking address of uninitialized variable
            if let Some(var_name) = extract_var_name(inner) {
                let state = tracker.get_state(&var_name);
                if state == InitState::Uninitialized {
                    errors.push(format!(
                        "In function '{}': Taking address of uninitialized variable '{}'",
                        func_name, var_name
                    ));
                }
            }
            check_expr_init(inner, tracker, func_name, errors);
        }

        Expression::Dereference(inner) => {
            // Check the pointer itself
            check_expr_init(inner, tracker, func_name, errors);

            // If dereferencing a known pointer, check its target
            if let Some(ptr_name) = extract_var_name(inner) {
                if let Some(target_name) = tracker.get_points_to(&ptr_name) {
                    let state = tracker.get_state(target_name);
                    if state == InitState::Uninitialized {
                        errors.push(format!(
                            "In function '{}': Dereferencing pointer to uninitialized variable '{}'",
                            func_name, target_name
                        ));
                    }
                }
            }
        }

        Expression::BinaryOp { left, right, .. } => {
            check_expr_init(left, tracker, func_name, errors);
            check_expr_init(right, tracker, func_name, errors);
        }

        Expression::FunctionCall { args, .. } => {
            for arg in args {
                check_expr_init(arg, tracker, func_name, errors);
            }
        }

        Expression::MemberAccess { object, .. } => {
            check_expr_init(object, tracker, func_name, errors);
        }

        Expression::Move { inner, .. } => {
            check_expr_init(inner, tracker, func_name, errors);
        }

        Expression::Cast { inner, .. } => {
            check_expr_init(inner, tracker, func_name, errors);
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
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state_merge() {
        assert_eq!(InitState::Initialized.merge(InitState::Initialized), InitState::Initialized);
        assert_eq!(InitState::Uninitialized.merge(InitState::Uninitialized), InitState::Uninitialized);
        assert_eq!(InitState::Initialized.merge(InitState::Uninitialized), InitState::MaybeUninitialized);
    }

    #[test]
    fn test_tracker_basic() {
        let mut tracker = InitTracker::new();
        tracker.declare("x", false);
        assert_eq!(tracker.get_state("x"), InitState::Uninitialized);

        tracker.initialize("x");
        assert_eq!(tracker.get_state("x"), InitState::Initialized);
    }

    #[test]
    fn test_tracker_scope() {
        let mut tracker = InitTracker::new();
        tracker.declare("x", true);
        assert_eq!(tracker.get_state("x"), InitState::Initialized);

        tracker.enter_scope();
        tracker.declare("y", false);
        assert_eq!(tracker.get_state("y"), InitState::Uninitialized);

        tracker.exit_scope();
        // y should be gone after scope exit
        assert_eq!(tracker.get_state("y"), InitState::Initialized); // defaults to Initialized
    }
}
