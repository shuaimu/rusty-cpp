use crate::parser::annotations::{LifetimeAnnotation, FunctionSignature, LifetimeBound};
use crate::parser::HeaderCache;
use crate::parser::safety_annotations::SafetyContext;
use crate::ir::{IrProgram, IrStatement, IrFunction, VariableType};
use std::collections::{HashMap, HashSet};
use crate::debug_println;

/// Tracks lifetime information for variables in the current scope
#[derive(Debug, Clone)]
pub struct LifetimeScope {
    /// Maps variable names to their lifetimes
    variable_lifetimes: HashMap<String, String>,
    /// Active lifetime constraints
    constraints: Vec<LifetimeBound>,
    /// Variables that own their data (not references)
    owned_variables: HashSet<String>,
    /// Lifetimes that have expired (scope has ended)
    expired_lifetimes: HashSet<String>,
}

impl LifetimeScope {
    pub fn new() -> Self {
        Self {
            variable_lifetimes: HashMap::new(),
            constraints: Vec::new(),
            owned_variables: HashSet::new(),
            expired_lifetimes: HashSet::new(),
        }
    }
    
    /// Assign a lifetime to a variable
    pub fn set_lifetime(&mut self, var: String, lifetime: String) {
        self.variable_lifetimes.insert(var, lifetime);
    }
    
    /// Mark a variable as owned (not a reference)
    pub fn mark_owned(&mut self, var: String) {
        self.owned_variables.insert(var);
    }
    
    /// Get the lifetime of a variable
    pub fn get_lifetime(&self, var: &str) -> Option<&String> {
        self.variable_lifetimes.get(var)
    }
    
    /// Check if a variable is owned
    pub fn is_owned(&self, var: &str) -> bool {
        self.owned_variables.contains(var)
    }
    
    /// Add a lifetime constraint
    #[allow(dead_code)]
    pub fn add_constraint(&mut self, constraint: LifetimeBound) {
        self.constraints.push(constraint);
    }

    /// Mark a scope's lifetime as expired (when ExitScope is encountered)
    pub fn expire_scope(&mut self, scope_depth: usize) {
        let lifetime = format!("'scope_{}", scope_depth);
        self.expired_lifetimes.insert(lifetime);
    }

    /// Check if a variable's lifetime has expired
    pub fn is_lifetime_expired(&self, var: &str) -> bool {
        if let Some(lifetime) = self.variable_lifetimes.get(var) {
            self.expired_lifetimes.contains(lifetime)
        } else {
            false
        }
    }

    /// Get the lifetime string for a variable (for error messages)
    pub fn get_lifetime_for_error(&self, var: &str) -> Option<String> {
        self.variable_lifetimes.get(var).cloned()
    }

    /// Check if lifetime 'a outlives lifetime 'b
    pub fn check_outlives(&self, longer: &str, shorter: &str) -> bool {
        // If they're the same lifetime, it trivially outlives itself
        if longer == shorter {
            return true;
        }

        // Check scope-based lifetimes: 'scope_X outlives 'scope_Y if X < Y
        // Lower scope number = outer scope = longer lifetime
        if let (Some(longer_scope), Some(shorter_scope)) = (
            longer.strip_prefix("'scope_"),
            shorter.strip_prefix("'scope_")
        ) {
            if let (Ok(longer_depth), Ok(shorter_depth)) = (
                longer_scope.parse::<usize>(),
                shorter_scope.parse::<usize>()
            ) {
                return longer_depth <= shorter_depth;
            }
        }

        // Check explicit constraints
        for constraint in &self.constraints {
            if constraint.longer == longer && constraint.shorter == shorter {
                return true;
            }
        }

        // Implement transitive outlives checking
        // If 'a: 'b and 'b: 'c, then 'a: 'c
        self.check_outlives_transitive(longer, shorter, &mut HashSet::new())
    }
    
    /// Check outlives relationship with transitive closure
    fn check_outlives_transitive(&self, longer: &str, shorter: &str, visited: &mut HashSet<String>) -> bool {
        // Avoid infinite recursion
        if visited.contains(longer) {
            return false;
        }
        visited.insert(longer.to_string());
        
        // Find all lifetimes that 'longer' outlives directly
        for constraint in &self.constraints {
            if constraint.longer == longer {
                // Check if we found the target
                if constraint.shorter == shorter {
                    return true;
                }
                
                // Try transitively through this intermediate lifetime
                if self.check_outlives_transitive(&constraint.shorter, shorter, visited) {
                    return true;
                }
            }
        }
        
        false
    }
}

/// Check lifetime constraints in a program using header annotations
/// Check if a file path is from a system header (not user code)
fn is_system_header(file_path: &str) -> bool {
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

    // Also skip project include directory
    if file_path.contains("/include/rusty/") || file_path.contains("/include/unified_") {
        return true;
    }

    false
}

pub fn check_lifetimes_with_annotations(
    program: &IrProgram,
    header_cache: &HeaderCache,
    safety_context: &SafetyContext
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();

    for function in &program.functions {
        // Skip system header functions
        if is_system_header(&function.source_file) {
            continue;
        }

        // Only check functions that should be analyzed (i.e., @safe functions)
        // Bug #9 fix: undeclared functions should NOT be analyzed
        if !safety_context.should_check_function(&function.name) {
            continue;
        }
        let mut scope = LifetimeScope::new();
        let function_errors = check_function_lifetimes(function, &mut scope, header_cache, &program.types_with_ref_members)?;
        errors.extend(function_errors);
    }

    Ok(errors)
}

fn check_function_lifetimes(
    function: &IrFunction,
    scope: &mut LifetimeScope,
    header_cache: &HeaderCache,
    types_with_ref_members: &std::collections::HashSet<String>
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    
    // Initialize lifetimes for function parameters and variables
    // For now, give each variable a unique lifetime based on its name
    for (name, var_info) in &function.variables {
        match &var_info.ty {
            crate::ir::VariableType::Reference(_) |
            crate::ir::VariableType::MutableReference(_) => {
                // References get a lifetime based on their name
                scope.set_lifetime(name.clone(), format!("'{}", name));
            }
            _ => {
                // Owned types don't have lifetimes
                scope.mark_owned(name.clone());
            }
        }
    }
    
    // Track scope using a unique scope ID (counter) instead of just depth
    // This ensures sequential scopes at the same depth have different IDs
    let mut scope_counter: usize = 0;  // Monotonically increasing scope ID
    let mut scope_stack: Vec<usize> = vec![0];  // Stack of active scope IDs (starts with scope 0)
    let mut variable_scopes: HashMap<String, usize> = HashMap::new();

    // First pass: assign unique scope IDs to variables based on where they're declared/used
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];

        for statement in &block.statements {
            match statement {
                IrStatement::EnterScope => {
                    scope_counter += 1;
                    scope_stack.push(scope_counter);
                }
                IrStatement::ExitScope => {
                    if scope_stack.len() > 1 {
                        scope_stack.pop();
                    }
                }
                IrStatement::Assign { lhs, .. } |
                IrStatement::Borrow { to: lhs, .. } => {
                    // Record the current scope ID where this variable is first assigned
                    if !variable_scopes.contains_key(lhs) {
                        let current_scope = *scope_stack.last().unwrap_or(&0);
                        variable_scopes.insert(lhs.clone(), current_scope);
                    }
                }
                IrStatement::CallExpr { args, result, .. } => {
                    // Record result variable if present
                    if let Some(lhs) = result {
                        if !variable_scopes.contains_key(lhs) {
                            let current_scope = *scope_stack.last().unwrap_or(&0);
                            variable_scopes.insert(lhs.clone(), current_scope);
                        }
                    }
                    // For arguments that don't have lifetimes yet, record their scope
                    for arg in args {
                        if !variable_scopes.contains_key(arg) && scope.is_owned(arg) {
                            let current_scope = *scope_stack.last().unwrap_or(&0);
                            variable_scopes.insert(arg.clone(), current_scope);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Assign scope-based lifetimes to owned variables using unique scope IDs
    // Variables not in variable_scopes must have been declared before any tracked statements (scope 0)
    // This includes function parameters and variables declared at the start of the function
    for (var_name, _var_info) in &function.variables {
        if scope.is_owned(var_name) {
            // Check if this variable was assigned/declared in a tracked statement
            if let Some(&scope_id) = variable_scopes.get(var_name) {
                // Variable was assigned/declared, use that unique scope ID
                let lifetime = format!("'scope_{}", scope_id);
                scope.set_lifetime(var_name.clone(), lifetime.clone());
            } else {
                // Variable exists but was never assigned/declared in statements
                // It must be a parameter or declared at function start (scope 0)
                let lifetime = "'scope_0".to_string();
                scope.set_lifetime(var_name.clone(), lifetime.clone());
            }
        }
    }

    // Reset for statement processing - use a new stack with scope ID tracking
    scope_counter = 0;
    scope_stack = vec![0];

    // Check each statement in the function
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];

        for (_idx, statement) in block.statements.iter().enumerate() {
            match statement {
                IrStatement::EnterScope => {
                    scope_counter += 1;
                    scope_stack.push(scope_counter);
                }
                IrStatement::ExitScope => {
                    // Mark the current scope's lifetime as expired BEFORE popping
                    if let Some(&current_scope_id) = scope_stack.last() {
                        scope.expire_scope(current_scope_id);
                    }
                    if scope_stack.len() > 1 {
                        scope_stack.pop();
                    }
                }
                IrStatement::CallExpr { func, args, result, receiver_is_temporary } => {
                    // Check if we have annotations for this function
                    if let Some(signature) = header_cache.get_signature(func) {
                        let call_errors = check_function_call(
                            func,
                            args,
                            result.as_ref(),
                            signature,
                            scope,  // Pass mutable scope to set result lifetime
                            *receiver_is_temporary,
                        );
                        errors.extend(call_errors);
                    }
                }

                IrStatement::Borrow { from, to, .. } => {
                    // When creating a reference, the new reference has the same lifetime
                    // as the source or a shorter one
                    if let Some(from_lifetime) = scope.get_lifetime(from) {
                        scope.set_lifetime(to.clone(), from_lifetime.clone());
                    } else if scope.is_owned(from) {
                        // Borrowing from owned data creates a new lifetime
                        scope.set_lifetime(to.clone(), format!("'{}", to));
                    }
                }

                IrStatement::UseVariable { var, operation } => {
                    // Check if variable's lifetime has expired
                    if scope.is_lifetime_expired(var) {
                        if let Some(lifetime) = scope.get_lifetime_for_error(var) {
                            errors.push(format!(
                                "Use of '{}' after its lifetime has expired (lifetime {} is no longer valid) - {} operation",
                                var, lifetime, operation
                            ));
                        }
                    }
                }

                IrStatement::Assign { lhs, rhs, .. } => {
                    // Check if RHS variable's lifetime has expired
                    if let crate::ir::IrExpression::Variable(rhs_var) = rhs {
                        if scope.is_lifetime_expired(rhs_var) {
                            if let Some(lifetime) = scope.get_lifetime_for_error(rhs_var) {
                                errors.push(format!(
                                    "Use of '{}' in assignment to '{}' after its lifetime has expired (lifetime {})",
                                    rhs_var, lhs, lifetime
                                ));
                            }
                        }
                    }
                }

                IrStatement::Return { value, .. } => {
                    // Check that returned references have appropriate lifetimes
                    if let Some(value) = value {
                        let return_errors = check_return_lifetime(value, function, scope, types_with_ref_members);
                        errors.extend(return_errors);
                    }
                }

                _ => {}
            }
        }
    }
    
    Ok(errors)
}

fn check_function_call(
    func_name: &str,
    args: &[String],
    result: Option<&String>,
    signature: &FunctionSignature,
    scope: &mut LifetimeScope,
    receiver_is_temporary: bool,
) -> Vec<String> {
    let mut errors = Vec::new();

    // Check that we have the right number of arguments
    if args.len() != signature.param_lifetimes.len() {
        // Signature doesn't match, skip lifetime checking
        return errors;
    }

    // Collect the actual lifetimes of arguments
    let mut arg_lifetimes = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if let Some(lifetime) = scope.get_lifetime(arg) {
            arg_lifetimes.push(Some(lifetime.clone()));
        } else if scope.is_owned(arg) {
            arg_lifetimes.push(None); // Owned value
        } else {
            arg_lifetimes.push(Some(format!("'arg{}", i)));
        }
    }

    // Check for temporary receiver with 'self lifetime in return
    // If the method returns &'self and receiver is temporary, result is dangling
    if receiver_is_temporary {
        if let Some(return_lifetime) = &signature.return_lifetime {
            let is_self_lifetime = match return_lifetime {
                LifetimeAnnotation::Ref(lt) | LifetimeAnnotation::MutRef(lt) => lt == "self",
                LifetimeAnnotation::Ptr(lt) | LifetimeAnnotation::ConstPtr(lt) => lt == "self",
                _ => false,
            };
            if is_self_lifetime {
                if let Some(result_var) = result {
                    errors.push(format!(
                        "Reference '{}' is bound to a temporary object that will be destroyed at the end of the statement",
                        result_var
                    ));
                }
            }
        }
    }
    
    // Check parameter lifetime requirements
    // Note: In C++, passing owned values to const reference parameters is legal (creates temporary)
    // So we only check for ownership transfer violations
    for (i, (arg, expected)) in args.iter().zip(&signature.param_lifetimes).enumerate() {
        if let Some(expected_lifetime) = expected {
            match expected_lifetime {
                LifetimeAnnotation::Ref(_expected) | LifetimeAnnotation::MutRef(_expected) => {
                    // In C++, you can pass owned values to reference parameters
                    // The compiler creates a temporary reference
                    // So we don't error here - just note that owned values will create temporaries
                }
                LifetimeAnnotation::Ptr(_expected) | LifetimeAnnotation::ConstPtr(_expected) => {
                    // Pointer parameters work similarly to reference parameters
                    // The pointer value is passed, lifetime is tracked
                }
                LifetimeAnnotation::Owned => {
                    // The argument should transfer ownership
                    if !scope.is_owned(arg) {
                        errors.push(format!(
                            "Function '{}' expects ownership of parameter {}, but '{}' is a reference",
                            func_name, i + 1, arg
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    
    // Check lifetime bounds
    for bound in &signature.lifetime_bounds {
        // Map lifetime names from signature to actual argument lifetimes
        let longer_lifetime = map_lifetime_to_actual(&bound.longer, &arg_lifetimes);
        let shorter_lifetime = map_lifetime_to_actual(&bound.shorter, &arg_lifetimes);

        if let (Some(longer), Some(shorter)) = (longer_lifetime, shorter_lifetime) {
            let outlives = scope.check_outlives(&longer, &shorter);
            if !outlives {
                errors.push(format!(
                    "Lifetime constraint violated in call to '{}': '{}' must outlive '{}'",
                    func_name, longer, shorter
                ));
            }
        }
    }
    
    // Check return lifetime and set result variable's lifetime
    if let (Some(result_var), Some(return_lifetime)) = (result, &signature.return_lifetime) {
        match return_lifetime {
            LifetimeAnnotation::Ref(ret_lifetime) | LifetimeAnnotation::MutRef(ret_lifetime) => {
                // The return value is a reference that borrows from one of the parameters
                // Map the return lifetime to the actual argument lifetime
                let actual_lifetime = map_lifetime_to_actual(ret_lifetime, &arg_lifetimes);
                if let Some(lifetime) = actual_lifetime {
                    // Set the result variable's lifetime to match the argument's lifetime
                    scope.set_lifetime(result_var.clone(), lifetime);
                }
            }
            LifetimeAnnotation::Ptr(ret_lifetime) | LifetimeAnnotation::ConstPtr(ret_lifetime) => {
                // The return value is a pointer that borrows from one of the parameters
                // Same lifetime tracking as references
                let actual_lifetime = map_lifetime_to_actual(ret_lifetime, &arg_lifetimes);
                if let Some(lifetime) = actual_lifetime {
                    // Set the result pointer's lifetime to match the argument's lifetime
                    scope.set_lifetime(result_var.clone(), lifetime);
                }
            }
            LifetimeAnnotation::Owned => {
                // The return value is owned, mark it as owned in scope
                scope.mark_owned(result_var.clone());
            }
            _ => {}
        }
    }

    errors
}

fn check_return_lifetime(
    value: &str,
    function: &IrFunction,
    scope: &LifetimeScope,
    types_with_ref_members: &std::collections::HashSet<String>
) -> Vec<String> {
    let mut errors = Vec::new();

    // First, check if the returned value is actually a reference type
    // Returning pointer values (Raw, Owned) is safe - only references are dangerous
    if let Some(var_info) = function.variables.get(value) {
        match &var_info.ty {
            VariableType::Reference(_) | VariableType::MutableReference(_) => {
                // Variable is a REFERENCE type (alias) - it's not a local object itself
                // Check if its lifetime is tied to a local OWNED variable
                if let Some(lifetime) = scope.get_lifetime(value) {
                    // Check if this lifetime is tied to a local owned variable
                    for (var_name, other_var_info) in &function.variables {
                        // Skip self-referential check (variable tracking its own name as lifetime)
                        if var_name == value {
                            continue;
                        }
                        // Only flag if the dependency is an OWNED local variable
                        // Reference aliases that depend on other references are OK
                        // (they inherit the lifetime of whatever they're bound to)
                        let is_owned_type = matches!(
                            other_var_info.ty,
                            VariableType::Owned(_)
                        );
                        if is_owned_type && lifetime.contains(var_name) && !is_parameter(var_name, function) {
                            errors.push(format!(
                                "Returning reference to local variable '{}' - this will create a dangling reference",
                                var_name
                            ));
                        }
                    }
                }
            }
            VariableType::Owned(_) => {
                // Variable is an OWNED local object - returning a reference to it is dangerous
                // (This case is handled elsewhere - the function returns a reference but
                // the variable itself is not a reference type, so we're taking &local)
            }
            // Pointer types (Raw, UniquePtr, SharedPtr) are safe to return
            // The pointer value is copied, heap memory persists after function return
            _ => {}
        }
    }

    // Check if the function returns a struct with reference members
    // If the return type is a struct with ref members, and the value (constructor arg) is a local,
    // then the struct's reference members will dangle after return
    let return_type = &function.return_type;

    // Extract base type name (strip qualifiers, namespaces, etc.)
    let base_return_type = return_type
        .trim()
        .trim_start_matches("const ")
        .trim_start_matches("struct ")
        .split('<').next().unwrap_or(return_type)  // Handle templates
        .split("::").last().unwrap_or(return_type)  // Handle namespaces
        .trim();

    if types_with_ref_members.contains(base_return_type) {
        // The return type is a struct with reference members
        // The 'value' passed to this function is the source of the constructor (e.g., "x" for Holder{x})
        // If this source is a local owned variable, the struct's reference will dangle
        if let Some(var_info) = function.variables.get(value) {
            let is_local_owned = matches!(var_info.ty, VariableType::Owned(_)) && !var_info.is_parameter;
            if is_local_owned {
                errors.push(format!(
                    "Returning struct '{}' with reference member initialized from local variable '{}' - \
                    the struct's reference member will be dangling after function return",
                    base_return_type, value
                ));
            }
        }
    }

    errors
}

fn is_parameter(var_name: &str, function: &IrFunction) -> bool {
    // Check if variable is marked as a parameter in the IR
    function.variables.get(var_name)
        .map(|var_info| var_info.is_parameter)
        .unwrap_or(false)
}

fn map_lifetime_to_actual(lifetime_name: &str, arg_lifetimes: &[Option<String>]) -> Option<String> {
    // Map lifetime parameter names like 'a, 'b to actual argument lifetimes
    match lifetime_name {
        "a" => arg_lifetimes.get(0).and_then(|l| l.clone()),
        "b" => arg_lifetimes.get(1).and_then(|l| l.clone()),
        "c" => arg_lifetimes.get(2).and_then(|l| l.clone()),
        _ => Some(format!("'{}", lifetime_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lifetime_scope() {
        let mut scope = LifetimeScope::new();
        
        scope.set_lifetime("ref1".to_string(), "'a".to_string());
        scope.mark_owned("value".to_string());
        
        assert_eq!(scope.get_lifetime("ref1"), Some(&"'a".to_string()));
        assert!(scope.is_owned("value"));
        assert!(!scope.is_owned("ref1"));
    }
    
    #[test]
    fn test_outlives_checking() {
        let mut scope = LifetimeScope::new();

        scope.add_constraint(LifetimeBound {
            longer: "a".to_string(),
            shorter: "b".to_string(),
        });

        assert!(scope.check_outlives("a", "b"));
        assert!(scope.check_outlives("a", "a")); // Self outlives
        assert!(!scope.check_outlives("b", "a")); // Not declared
    }

    #[test]
    fn test_check_return_lifetime_pointer_is_safe() {
        use crate::ir::{VariableInfo, OwnershipState, ControlFlowGraph};
        use std::collections::HashMap;

        // Create a function with a pointer variable (Raw type)
        let mut variables = HashMap::new();
        variables.insert("p".to_string(), VariableInfo {
            name: "p".to_string(),
            ty: VariableType::Raw("void*".to_string()),
            ownership: OwnershipState::Owned,
            lifetime: None,
            is_parameter: false,
            is_static: false,
            scope_level: 1,
            has_destructor: false,
            declaration_index: 0,
        });

        let function = IrFunction {
            name: "allocate".to_string(),
            cfg: ControlFlowGraph::new(),
            variables,
            return_type: "void*".to_string(),
            source_file: "test.cpp".to_string(),
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            lifetime_params: HashMap::new(),
            param_lifetimes: vec![],
            return_lifetime: None,
            lifetime_constraints: vec![],
        };

        let mut scope = LifetimeScope::new();
        scope.set_lifetime("p".to_string(), "'p".to_string());

        // Returning a pointer should NOT produce any errors
        let empty_types = std::collections::HashSet::new();
        let errors = check_return_lifetime("p", &function, &scope, &empty_types);
        assert!(errors.is_empty(), "Returning pointer value should be safe, got: {:?}", errors);
    }

    #[test]
    fn test_check_return_lifetime_reference_is_unsafe() {
        use crate::ir::{VariableInfo, OwnershipState, ControlFlowGraph};
        use std::collections::HashMap;

        // Create a function with a reference variable
        let mut variables = HashMap::new();
        variables.insert("local".to_string(), VariableInfo {
            name: "local".to_string(),
            ty: VariableType::Owned("int".to_string()),
            ownership: OwnershipState::Owned,
            lifetime: None,
            is_parameter: false,
            is_static: false,
            scope_level: 1,
            has_destructor: false,
            declaration_index: 0,
        });
        variables.insert("ref".to_string(), VariableInfo {
            name: "ref".to_string(),
            ty: VariableType::Reference("int".to_string()),
            ownership: OwnershipState::Owned,
            lifetime: None,
            is_parameter: false,
            is_static: false,
            scope_level: 1,
            has_destructor: false,
            declaration_index: 1,
        });

        let function = IrFunction {
            name: "bad_return".to_string(),
            cfg: ControlFlowGraph::new(),
            variables,
            return_type: "int&".to_string(),
            source_file: "test.cpp".to_string(),
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            lifetime_params: HashMap::new(),
            param_lifetimes: vec![],
            return_lifetime: None,
            lifetime_constraints: vec![],
        };

        let mut scope = LifetimeScope::new();
        // The reference's lifetime is tied to the local variable
        scope.set_lifetime("ref".to_string(), "'local".to_string());

        // Returning a reference to local should produce an error
        let empty_types = std::collections::HashSet::new();
        let errors = check_return_lifetime("ref", &function, &scope, &empty_types);
        assert!(!errors.is_empty(), "Returning reference to local should be flagged as unsafe");
        assert!(errors[0].contains("local"), "Error should mention the local variable");
    }

    #[test]
    fn test_check_return_lifetime_owned_is_safe() {
        use crate::ir::{VariableInfo, OwnershipState, ControlFlowGraph};
        use std::collections::HashMap;

        // Create a function with an owned variable (like unique_ptr)
        let mut variables = HashMap::new();
        variables.insert("ptr".to_string(), VariableInfo {
            name: "ptr".to_string(),
            ty: VariableType::UniquePtr("int".to_string()),
            ownership: OwnershipState::Owned,
            lifetime: None,
            is_parameter: false,
            is_static: false,
            scope_level: 1,
            has_destructor: true,
            declaration_index: 0,
        });

        let function = IrFunction {
            name: "create".to_string(),
            cfg: ControlFlowGraph::new(),
            variables,
            return_type: "std::unique_ptr<int>".to_string(),
            source_file: "test.cpp".to_string(),
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            lifetime_params: HashMap::new(),
            param_lifetimes: vec![],
            return_lifetime: None,
            lifetime_constraints: vec![],
        };

        let mut scope = LifetimeScope::new();
        scope.set_lifetime("ptr".to_string(), "'ptr".to_string());

        // Returning owned value should NOT produce any errors
        let empty_types = std::collections::HashSet::new();
        let errors = check_return_lifetime("ptr", &function, &scope, &empty_types);
        assert!(errors.is_empty(), "Returning owned value should be safe, got: {:?}", errors);
    }
}