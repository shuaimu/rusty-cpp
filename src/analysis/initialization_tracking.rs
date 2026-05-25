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

use crate::parser::safety_annotations::SafetyMode;
use crate::parser::{Expression, Function, Statement};
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
        let state = if initialized {
            InitState::Initialized
        } else {
            InitState::Uninitialized
        };
        self.states.insert(var.to_string(), state);
    }

    /// Mark a variable as initialized
    pub fn initialize(&mut self, var: &str) {
        self.states.insert(var.to_string(), InitState::Initialized);
    }

    /// Get the initialization state of a variable
    pub fn get_state(&self, var: &str) -> InitState {
        self.states
            .get(var)
            .copied()
            .unwrap_or(InitState::Initialized)
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
pub fn check_initialization_safety(
    function: &Function,
    function_safety: SafetyMode,
) -> Vec<String> {
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
            // A variable declaration with no initializer leaves the
            // variable uninitialized for primitive types. Use the
            // `has_initializer` flag set by `extract_variable` from the
            // libclang VarDecl entity — this is authoritative regardless
            // of whether `extract_expression` can parse the RHS into our
            // `Expression` IR. This fixes false-positives for shapes
            // like `int mode = EnumScope::VALUE;`, `auto id = call();`,
            // `auto v = cond ? a : b;`, and brace-init `Foo f{...}`.
            //
            // Other cases that imply initialization:
            // - References (must bind at declaration in C++)
            // - Const variables (must be initialized — language rule)
            // - Class/struct types (default constructor runs even for
            //   `Container c;` so the object is reachable, just default-
            //   initialized; only primitive types are truly uninitialized)
            let is_initialized = var.has_initializer
                || var.is_reference
                || (!var.type_name.is_empty() && var.is_const)
                || is_class_or_struct_type(&var.type_name);

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

        Statement::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
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

/// Check if a type is a class or struct type (not a primitive)
/// Class/struct types are initialized by their constructor, so they should be
/// considered initialized even without an explicit initializer.
fn is_class_or_struct_type(type_name: &str) -> bool {
    // Strip qualifiers and modifiers
    let clean_type = type_name
        .replace("const ", "")
        .replace("volatile ", "")
        .replace("&", "")
        .replace("*", "")
        .trim()
        .to_string();

    // Primitive types that are NOT initialized by default
    let primitives = [
        "int",
        "unsigned int",
        "signed int",
        "short",
        "unsigned short",
        "signed short",
        "long",
        "unsigned long",
        "signed long",
        "long long",
        "unsigned long long",
        "signed long long",
        "char",
        "unsigned char",
        "signed char",
        "float",
        "double",
        "long double",
        "bool",
        "void",
        "size_t",
        "ssize_t",
        "int8_t",
        "int16_t",
        "int32_t",
        "int64_t",
        "uint8_t",
        "uint16_t",
        "uint32_t",
        "uint64_t",
        "intptr_t",
        "uintptr_t",
        "ptrdiff_t",
    ];

    // If it's a primitive, it's NOT a class/struct
    if primitives.iter().any(|p| clean_type == *p) {
        return false;
    }

    // If it's empty, assume not initialized
    if clean_type.is_empty() {
        return false;
    }

    // STL types that have constructors
    if clean_type.starts_with("std::") {
        return true;
    }

    // If it starts with an uppercase letter, likely a user-defined type
    // This is a heuristic - most C++ class names start with uppercase
    if clean_type
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
    {
        return true;
    }

    // Template types are typically classes
    if clean_type.contains('<') {
        return true;
    }

    // Default: assume it's a class type if it's not a recognized primitive
    // This errs on the side of not flagging false positives
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state_merge() {
        assert_eq!(
            InitState::Initialized.merge(InitState::Initialized),
            InitState::Initialized
        );
        assert_eq!(
            InitState::Uninitialized.merge(InitState::Uninitialized),
            InitState::Uninitialized
        );
        assert_eq!(
            InitState::Initialized.merge(InitState::Uninitialized),
            InitState::MaybeUninitialized
        );
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

    use crate::parser::ast_visitor::Variable;
    use crate::parser::{Function, SourceLocation, Statement};

    fn loc() -> SourceLocation {
        SourceLocation {
            file: "test.cpp".to_string(),
            line: 1,
            column: 1,
        }
    }

    /// Helper: build a Variable matching a primitive local declaration.
    /// `has_initializer` is the only knob that varies across the tests.
    fn primitive_var(name: &str, type_name: &str, has_initializer: bool) -> Variable {
        Variable {
            name: name.to_string(),
            type_name: type_name.to_string(),
            is_reference: false,
            is_pointer: false,
            is_const: false,
            is_unique_ptr: false,
            is_shared_ptr: false,
            is_static: false,
            is_mutable: false,
            location: loc(),
            is_pack: false,
            pack_element_type: None,
            has_initializer,
        }
    }

    fn safe_fn(name: &str, body: Vec<Statement>) -> Function {
        Function {
            name: name.to_string(),
            parameters: vec![],
            return_type: "void".to_string(),
            body,
            location: loc(),
            is_method: false,
            method_qualifier: None,
            template_parameters: vec![],
            safety_annotation: Some(SafetyMode::Safe),
            has_explicit_safety_annotation: true,
            is_deleted: false,
            member_initializers: vec![],
        }
    }

    #[test]
    fn test_primitive_without_initializer_used_in_return_is_flagged() {
        // `int x; return x;` — genuinely uninitialized.
        let body = vec![
            Statement::VariableDecl(primitive_var("x", "int", false)),
            Statement::Return(Some(crate::parser::Expression::Variable("x".to_string()))),
        ];
        let errors = check_initialization_safety(&safe_fn("f", body), SafetyMode::Safe);
        assert_eq!(errors.len(), 1, "expected 1 error, got: {:?}", errors);
        assert!(errors[0].contains("uninitialized variable 'x'"));
    }

    #[test]
    fn test_primitive_with_initializer_used_in_return_is_ok() {
        // `int x = 5; return x;` — has_initializer set by extract_variable.
        // This covers the `int mode = PollMode::READ;` shape from the rrr
        // tcp_channel.cpp false positive — the RHS may or may not be
        // parseable as `Expression`, but `has_initializer=true` is enough.
        let body = vec![
            Statement::VariableDecl(primitive_var("x", "int", true)),
            Statement::Return(Some(crate::parser::Expression::Variable("x".to_string()))),
        ];
        let errors = check_initialization_safety(&safe_fn("f", body), SafetyMode::Safe);
        assert!(
            errors.is_empty(),
            "expected no errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_typedef_primitive_with_initializer_is_ok() {
        // `uint64_t id = get_next_id(); return id;` — same shape as the
        // alock.cpp WaitDieALock::vlock false positive. The function-call
        // RHS may not parse into Expression, but has_initializer=true.
        let body = vec![
            Statement::VariableDecl(primitive_var("id", "uint64_t", true)),
            Statement::Return(Some(crate::parser::Expression::Variable("id".to_string()))),
        ];
        let errors = check_initialization_safety(&safe_fn("f", body), SafetyMode::Safe);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_double_with_ternary_initializer_is_ok() {
        // `double avg = (n > 0) ? sum/n : 0.0; log(avg);` — the reactor.cpp
        // stackless_profile_report_periodic shape that motivated the
        // original Bug 3 investigation. Ternary expressions exposed via
        // UnexposedExpr may not parse into Expression, but has_initializer
        // is enough.
        let body = vec![
            Statement::VariableDecl(primitive_var("avg", "double", true)),
            Statement::FunctionCall {
                name: "log".to_string(),
                args: vec![crate::parser::Expression::Variable("avg".to_string())],
                location: loc(),
            },
        ];
        let errors = check_initialization_safety(&safe_fn("f", body), SafetyMode::Safe);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }
}
