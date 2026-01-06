//! Null Safety Analysis
//!
//! This module implements dataflow analysis to track the "possibly null" state
//! of pointer variables and flag dereferences of potentially null pointers.
//!
//! States:
//! - NonNull: Definitely not null (e.g., result of &x)
//! - Null: Definitely null (e.g., nullptr)
//! - MaybeNull: Could be either (e.g., function parameter, conditional assignment)
//!
//! Rules:
//! - Dereferencing a MaybeNull pointer is an error in @safe code
//! - Null checks (if (ptr != nullptr)) narrow the state to NonNull in the true branch

use crate::parser::{Statement, Expression, Function};
use crate::parser::safety_annotations::SafetyMode;
use std::collections::HashMap;

/// Represents the null state of a pointer variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullState {
    /// Definitely not null (e.g., &x, new T, known non-null source)
    NonNull,
    /// Definitely null (e.g., nullptr, NULL, 0)
    Null,
    /// Could be either (e.g., parameter, conditional, unknown source)
    MaybeNull,
}

impl NullState {
    /// Merge two states (for control flow join points)
    /// Conservative: if either branch could be null, result is MaybeNull
    pub fn merge(self, other: NullState) -> NullState {
        match (self, other) {
            (NullState::NonNull, NullState::NonNull) => NullState::NonNull,
            (NullState::Null, NullState::Null) => NullState::Null,
            _ => NullState::MaybeNull,
        }
    }
}

/// Tracks null state for all pointer variables in scope
#[derive(Debug, Clone)]
pub struct NullStateTracker {
    /// Maps variable name to its null state
    states: HashMap<String, NullState>,
    /// Stack of scopes for handling blocks
    scope_stack: Vec<HashMap<String, NullState>>,
}

impl NullStateTracker {
    pub fn new() -> Self {
        NullStateTracker {
            states: HashMap::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Set the null state for a variable
    pub fn set_state(&mut self, var: &str, state: NullState) {
        self.states.insert(var.to_string(), state);
    }

    /// Get the null state for a variable
    pub fn get_state(&self, var: &str) -> NullState {
        self.states.get(var).copied().unwrap_or(NullState::MaybeNull)
    }

    /// Enter a new scope (save current state)
    pub fn enter_scope(&mut self) {
        self.scope_stack.push(self.states.clone());
    }

    /// Exit scope (restore previous state)
    pub fn exit_scope(&mut self) {
        if let Some(previous) = self.scope_stack.pop() {
            self.states = previous;
        }
    }

    /// Merge states from another tracker (for if/else join)
    pub fn merge_branch(&mut self, other: &NullStateTracker) {
        for (var, state) in &other.states {
            let current = self.get_state(var);
            self.set_state(var, current.merge(*state));
        }
    }

    /// Create a snapshot for branch analysis
    pub fn snapshot(&self) -> NullStateTracker {
        NullStateTracker {
            states: self.states.clone(),
            scope_stack: Vec::new(),
        }
    }
}

/// Check for null safety violations in a parsed function
pub fn check_null_safety(function: &Function, function_safety: SafetyMode) -> Vec<String> {
    let mut errors = Vec::new();

    // Only check @safe functions
    if function_safety != SafetyMode::Safe {
        return errors;
    }

    let mut tracker = NullStateTracker::new();
    let mut unsafe_depth = 0;

    // Initialize parameters as MaybeNull (we don't know what caller passes)
    // Exception: parameters with NonNull annotation would be NonNull
    for param in &function.parameters {
        if is_pointer_type(&param.type_name) {
            // Check for _Nonnull or similar annotation
            if param.type_name.contains("_Nonnull") || param.type_name.contains("nonnull") {
                tracker.set_state(&param.name, NullState::NonNull);
            } else {
                tracker.set_state(&param.name, NullState::MaybeNull);
            }
        }
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

        // Analyze statement for null safety
        analyze_statement_null_safety(stmt, &mut tracker, &function.name, &mut errors);
    }

    errors
}

/// Analyze a single statement for null safety
fn analyze_statement_null_safety(
    stmt: &Statement,
    tracker: &mut NullStateTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match stmt {
        Statement::VariableDecl(var) => {
            // Track pointer variables
            if is_pointer_type(&var.type_name) {
                // Variable declarations don't have initializers in this AST,
                // so we'll track based on subsequent assignments
                // Default to MaybeNull unless we can prove otherwise
                tracker.set_state(&var.name, NullState::MaybeNull);
            }
        }

        Statement::Assignment { lhs, rhs, .. } => {
            // First check for null safety violations in rhs
            check_expr_null_safety(rhs, tracker, func_name, errors);

            // Update null state on assignment
            if let Some(var_name) = extract_var_name(lhs) {
                let state = determine_null_state_from_expr(rhs, tracker);
                tracker.set_state(&var_name, state);
            }
        }

        Statement::ReferenceBinding { target, .. } => {
            // Check the target expression
            check_expr_null_safety(target, tracker, func_name, errors);
        }

        Statement::FunctionCall { args, .. } => {
            // Check all arguments
            for arg in args {
                check_expr_null_safety(arg, tracker, func_name, errors);
            }
        }

        Statement::Return(Some(expr)) => {
            check_expr_null_safety(expr, tracker, func_name, errors);
        }

        Statement::ExpressionStatement { expr, .. } => {
            check_expr_null_safety(expr, tracker, func_name, errors);
        }

        Statement::If { condition, then_branch, else_branch, .. } => {
            // Check condition for null checks that narrow state
            let (narrowed_var, narrowed_to_nonnull) = check_null_narrowing(condition);

            // Check condition itself
            check_expr_null_safety(condition, tracker, func_name, errors);

            // Analyze then branch with potentially narrowed state
            let mut then_tracker = tracker.snapshot();
            if let Some(ref var) = narrowed_var {
                if narrowed_to_nonnull {
                    then_tracker.set_state(var, NullState::NonNull);
                }
            }
            for stmt in then_branch {
                analyze_statement_null_safety(stmt, &mut then_tracker, func_name, errors);
            }

            // Analyze else branch
            let mut else_tracker = tracker.snapshot();
            if let Some(ref var) = narrowed_var {
                if !narrowed_to_nonnull {
                    // In else branch of "if (ptr != nullptr)", ptr could be null
                    else_tracker.set_state(var, NullState::MaybeNull);
                }
            }
            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    analyze_statement_null_safety(stmt, &mut else_tracker, func_name, errors);
                }
            }

            // Merge states from both branches
            tracker.merge_branch(&then_tracker);
            if else_branch.is_some() {
                tracker.merge_branch(&else_tracker);
            }
        }

        Statement::Block(stmts) => {
            tracker.enter_scope();
            for stmt in stmts {
                analyze_statement_null_safety(stmt, tracker, func_name, errors);
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

/// Check an expression for null safety violations (dereference of MaybeNull)
fn check_expr_null_safety(
    expr: &Expression,
    tracker: &NullStateTracker,
    func_name: &str,
    errors: &mut Vec<String>,
) {
    match expr {
        Expression::Dereference(inner) => {
            // Check what we're dereferencing
            if let Some(var_name) = extract_var_name_from_expr(inner) {
                // Special case: 'this' is always non-null in C++ (guaranteed by the language)
                // Calling a method on a null pointer is undefined behavior, so we can assume
                // that if we're in a method, 'this' is valid.
                if var_name == "this" {
                    // OK - 'this' is always non-null
                } else {
                    let state = tracker.get_state(&var_name);
                    match state {
                        NullState::MaybeNull => {
                            errors.push(format!(
                                "In function '{}': Dereferencing potentially null pointer '{}' - add null check first",
                                func_name, var_name
                            ));
                        }
                        NullState::Null => {
                            errors.push(format!(
                                "In function '{}': Dereferencing null pointer '{}'",
                                func_name, var_name
                            ));
                        }
                        NullState::NonNull => {
                            // OK
                        }
                    }
                }
            }
            // Also check the inner expression
            check_expr_null_safety(inner, tracker, func_name, errors);
        }

        Expression::PointerArithmetic { pointer, .. } => {
            // Array subscript/pointer arithmetic on pointer is like dereference
            if let Some(var_name) = extract_var_name_from_expr(pointer) {
                let state = tracker.get_state(&var_name);
                if state == NullState::MaybeNull {
                    errors.push(format!(
                        "In function '{}': Pointer arithmetic on potentially null pointer '{}' - add null check first",
                        func_name, var_name
                    ));
                } else if state == NullState::Null {
                    errors.push(format!(
                        "In function '{}': Pointer arithmetic on null pointer '{}'",
                        func_name, var_name
                    ));
                }
            }
            check_expr_null_safety(pointer, tracker, func_name, errors);
        }

        Expression::MemberAccess { object, .. } => {
            // Member access could be via arrow operator
            // Check if the object is a pointer being dereferenced
            if let Some(var_name) = extract_var_name_from_expr(object) {
                // Note: MemberAccess doesn't have is_arrow flag, so we rely on
                // the type system. For now, check all member accesses on pointers.
                let state = tracker.get_state(&var_name);
                if state == NullState::MaybeNull {
                    // Only warn if we know it's a pointer type access
                    // This is conservative - we might miss some cases
                }
            }
            check_expr_null_safety(object, tracker, func_name, errors);
        }

        Expression::FunctionCall { args, .. } => {
            // Check all arguments
            for arg in args {
                check_expr_null_safety(arg, tracker, func_name, errors);
            }
        }

        Expression::BinaryOp { left, right, .. } => {
            check_expr_null_safety(left, tracker, func_name, errors);
            check_expr_null_safety(right, tracker, func_name, errors);
        }

        Expression::Move { inner, .. } => {
            check_expr_null_safety(inner, tracker, func_name, errors);
        }

        Expression::Cast { inner, .. } => {
            check_expr_null_safety(inner, tracker, func_name, errors);
        }

        Expression::AddressOf(inner) => {
            check_expr_null_safety(inner, tracker, func_name, errors);
        }

        _ => {}
    }
}

/// Determine the null state from an expression
fn determine_null_state_from_expr(expr: &Expression, tracker: &NullStateTracker) -> NullState {
    match expr {
        // Address-of is always non-null
        Expression::AddressOf(_) => NullState::NonNull,

        // nullptr literal is definitely null
        Expression::Nullptr => NullState::Null,

        // Literal "0" or "NULL" could be null in pointer context
        Expression::Literal(lit) if is_null_literal(lit) => NullState::Null,

        // new expressions are non-null (or throw)
        Expression::New(_) => NullState::NonNull,

        // Variable reference - inherit its state
        Expression::Variable(name) => tracker.get_state(name),

        // Function call - assume MaybeNull unless we know better
        Expression::FunctionCall { name, .. } => {
            // Some functions are known to return non-null
            if is_known_nonnull_function(name) {
                NullState::NonNull
            } else {
                NullState::MaybeNull
            }
        }

        // Cast preserves null state of inner expression
        Expression::Cast { inner, .. } => determine_null_state_from_expr(inner, tracker),

        // Move preserves null state
        Expression::Move { inner, .. } => determine_null_state_from_expr(inner, tracker),

        // Default: unknown = MaybeNull
        _ => NullState::MaybeNull,
    }
}

/// Check if a condition performs null checking and return the variable being checked
fn check_null_narrowing(condition: &Expression) -> (Option<String>, bool) {
    match condition {
        // ptr != nullptr or ptr != NULL or ptr != 0
        Expression::BinaryOp { left, right, op } if op == "!=" => {
            if is_null_expr(right) {
                if let Some(var) = extract_var_name_from_expr(left) {
                    return (Some(var), true); // narrowed to NonNull in true branch
                }
            }
            if is_null_expr(left) {
                if let Some(var) = extract_var_name_from_expr(right) {
                    return (Some(var), true);
                }
            }
            (None, false)
        }

        // ptr == nullptr (in true branch, it's definitely null)
        Expression::BinaryOp { left, right, op } if op == "==" => {
            if is_null_expr(right) {
                if let Some(var) = extract_var_name_from_expr(left) {
                    return (Some(var), false); // narrowed to Null in true branch
                }
            }
            if is_null_expr(left) {
                if let Some(var) = extract_var_name_from_expr(right) {
                    return (Some(var), false);
                }
            }
            (None, false)
        }

        // Just a variable as condition: if (ptr) { ... }
        // This is equivalent to ptr != nullptr
        Expression::Variable(name) => {
            (Some(name.clone()), true)
        }

        _ => (None, false),
    }
}

/// Check if an expression is a null expression
fn is_null_expr(expr: &Expression) -> bool {
    match expr {
        Expression::Nullptr => true,
        Expression::Literal(lit) => is_null_literal(lit),
        Expression::Variable(name) => name == "nullptr" || name == "NULL",
        _ => false,
    }
}

/// Check if a literal represents null
fn is_null_literal(lit: &str) -> bool {
    lit == "nullptr" || lit == "NULL" || lit == "0" || lit == "0L" || lit == "0UL"
}

/// Check if a type is a pointer type
fn is_pointer_type(type_name: &str) -> bool {
    type_name.contains('*') && !type_name.contains("&")
}

/// Extract variable name from a target expression (for assignments)
fn extract_var_name(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::Dereference(inner) => extract_var_name(inner),
        _ => None,
    }
}

/// Extract variable name from any expression
fn extract_var_name_from_expr(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Variable(name) => Some(name.clone()),
        Expression::Cast { inner, .. } => extract_var_name_from_expr(inner),
        Expression::Move { inner, .. } => extract_var_name_from_expr(inner),
        _ => None,
    }
}

/// Check if a function is known to return non-null
fn is_known_nonnull_function(name: &str) -> bool {
    // Functions that are known to return non-null (or throw on failure)
    name.contains("make_unique") ||
    name.contains("make_shared") ||
    name.contains("make_box") ||
    name.contains("make_arc") ||
    name == "operator new" ||
    name == "operator new[]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_state_merge() {
        assert_eq!(NullState::NonNull.merge(NullState::NonNull), NullState::NonNull);
        assert_eq!(NullState::Null.merge(NullState::Null), NullState::Null);
        assert_eq!(NullState::NonNull.merge(NullState::Null), NullState::MaybeNull);
        assert_eq!(NullState::NonNull.merge(NullState::MaybeNull), NullState::MaybeNull);
        assert_eq!(NullState::Null.merge(NullState::MaybeNull), NullState::MaybeNull);
    }

    #[test]
    fn test_tracker_basic() {
        let mut tracker = NullStateTracker::new();
        tracker.set_state("ptr", NullState::NonNull);
        assert_eq!(tracker.get_state("ptr"), NullState::NonNull);
        assert_eq!(tracker.get_state("unknown"), NullState::MaybeNull);
    }

    #[test]
    fn test_tracker_scope() {
        let mut tracker = NullStateTracker::new();
        tracker.set_state("ptr", NullState::NonNull);

        tracker.enter_scope();
        tracker.set_state("ptr", NullState::Null);
        assert_eq!(tracker.get_state("ptr"), NullState::Null);

        tracker.exit_scope();
        assert_eq!(tracker.get_state("ptr"), NullState::NonNull);
    }

    #[test]
    fn test_is_null_literal() {
        assert!(is_null_literal("nullptr"));
        assert!(is_null_literal("NULL"));
        assert!(is_null_literal("0"));
        assert!(!is_null_literal("42"));
        assert!(!is_null_literal("ptr"));
    }

    #[test]
    fn test_is_pointer_type() {
        assert!(is_pointer_type("int*"));
        assert!(is_pointer_type("const char*"));
        assert!(is_pointer_type("void *"));
        assert!(!is_pointer_type("int&"));
        assert!(!is_pointer_type("int"));
    }
}
