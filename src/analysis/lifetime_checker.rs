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
}

impl LifetimeScope {
    pub fn new() -> Self {
        Self {
            variable_lifetimes: HashMap::new(),
            constraints: Vec::new(),
            owned_variables: HashSet::new(),
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
        let function_errors = check_function_lifetimes(function, &mut scope, header_cache)?;
        errors.extend(function_errors);
    }

    Ok(errors)
}

fn check_function_lifetimes(
    function: &IrFunction,
    scope: &mut LifetimeScope,
    header_cache: &HeaderCache
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
    
    // Track scope depth for lifetime inference
    let mut scope_depth = 0;
    let mut variable_scopes: HashMap<String, usize> = HashMap::new();

    // First pass: assign scope depths to variables based on where they're declared/used
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];

        for statement in &block.statements {
            match statement {
                IrStatement::EnterScope => {
                    scope_depth += 1;
                }
                IrStatement::ExitScope => {
                    if scope_depth > 0 {
                        scope_depth -= 1;
                    }
                }
                IrStatement::Assign { lhs, .. } |
                IrStatement::Borrow { to: lhs, .. } => {
                    // Record the scope depth where this variable is first assigned
                    if !variable_scopes.contains_key(lhs) {
                        variable_scopes.insert(lhs.clone(), scope_depth);
                    }
                }
                IrStatement::CallExpr { args, result, .. } => {
                    // Record result variable if present
                    if let Some(lhs) = result {
                        if !variable_scopes.contains_key(lhs) {
                            variable_scopes.insert(lhs.clone(), scope_depth);
                        }
                    }
                    // For arguments that don't have lifetimes yet, record their scope
                    for arg in args {
                        if !variable_scopes.contains_key(arg) && scope.is_owned(arg) {
                            variable_scopes.insert(arg.clone(), scope_depth);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Assign scope-based lifetimes to owned variables
    // Variables not in variable_scopes must have been declared before any tracked statements (scope 0)
    // This includes function parameters and variables declared at the start of the function
    for (var_name, var_info) in &function.variables {
        if scope.is_owned(var_name) {
            // Check if this variable was assigned/declared in a tracked statement
            if let Some(&depth) = variable_scopes.get(var_name) {
                // Variable was assigned/declared, use that scope
                let lifetime = format!("'scope_{}", depth);
                scope.set_lifetime(var_name.clone(), lifetime.clone());
            } else {
                // Variable exists but was never assigned/declared in statements
                // It must be a parameter or declared at function start (scope 0)
                let lifetime = "'scope_0".to_string();
                scope.set_lifetime(var_name.clone(), lifetime.clone());
            }
        }
    }

    // Reset scope depth for statement processing
    scope_depth = 0;

    // Check each statement in the function
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];

        for (idx, statement) in block.statements.iter().enumerate() {
            match statement {
                IrStatement::EnterScope => {
                    scope_depth += 1;
                }
                IrStatement::ExitScope => {
                    if scope_depth > 0 {
                        scope_depth -= 1;
                    }
                }
                IrStatement::CallExpr { func, args, result } => {
                    // Check if we have annotations for this function
                    if let Some(signature) = header_cache.get_signature(func) {
                        let call_errors = check_function_call(
                            func,
                            args,
                            result.as_ref(),
                            signature,
                            scope
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
                
                IrStatement::Return { value } => {
                    // Check that returned references have appropriate lifetimes
                    if let Some(value) = value {
                        let return_errors = check_return_lifetime(value, function, scope);
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
    scope: &LifetimeScope
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
    
    // Check return lifetime
    if let (Some(_result_var), Some(return_lifetime)) = (result, &signature.return_lifetime) {
        match return_lifetime {
            LifetimeAnnotation::Ref(ret_lifetime) | LifetimeAnnotation::MutRef(ret_lifetime) => {
                // The return value is a reference that borrows from one of the parameters
                // Map the return lifetime to the actual argument lifetime
                let actual_lifetime = map_lifetime_to_actual(ret_lifetime, &arg_lifetimes);
                if let Some(_lifetime) = actual_lifetime {
                    // The result variable gets this lifetime
                    // Note: We're not modifying scope here as it's borrowed
                    // In a real implementation, we'd need mutable access
                }
            }
            LifetimeAnnotation::Owned => {
                // The return value is owned, no lifetime constraints
            }
            _ => {}
        }
    }
    
    errors
}

fn check_return_lifetime(
    value: &str,
    function: &IrFunction,
    scope: &LifetimeScope
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
        let errors = check_return_lifetime("p", &function, &scope);
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
        let errors = check_return_lifetime("ref", &function, &scope);
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
        let errors = check_return_lifetime("ptr", &function, &scope);
        assert!(errors.is_empty(), "Returning owned value should be safe, got: {:?}", errors);
    }
}