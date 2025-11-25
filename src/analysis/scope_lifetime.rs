use std::collections::{HashMap, HashSet, VecDeque};
use crate::ir::{IrFunction, IrStatement, IrProgram, BasicBlock};
use crate::parser::HeaderCache;
use crate::parser::annotations::{FunctionSignature, LifetimeAnnotation};
use petgraph::graph::NodeIndex;
use petgraph::Direction;

/// Represents a scope in the program (function, block, loop, etc.)
#[derive(Debug, Clone)]
pub struct Scope {
    #[allow(dead_code)]
    pub id: usize,
    pub parent: Option<usize>,
    #[allow(dead_code)]
    pub kind: ScopeKind,
    /// Variables declared in this scope
    pub local_variables: HashSet<String>,
    /// References created in this scope
    pub local_references: HashSet<String>,
    /// Starting point in the CFG
    #[allow(dead_code)]
    pub entry_block: Option<NodeIndex>,
    /// Ending point(s) in the CFG
    #[allow(dead_code)]
    pub exit_blocks: Vec<NodeIndex>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ScopeKind {
    Function,
    Block,
    Loop,
    Conditional,
}

/// Tracks lifetimes with respect to program scopes
#[derive(Debug)]
pub struct ScopedLifetimeTracker {
    /// All scopes in the program
    scopes: HashMap<usize, Scope>,
    /// Maps variables to their declaring scope
    variable_scope: HashMap<String, usize>,
    /// Maps lifetimes to their scope bounds
    lifetime_scopes: HashMap<String, (usize, usize)>, // (start_scope, end_scope)
    /// Active lifetime constraints
    constraints: Vec<LifetimeConstraint>,
    /// Counter for generating scope IDs
    next_scope_id: usize,
}

#[derive(Debug, Clone)]
pub struct LifetimeConstraint {
    pub kind: ConstraintKind,
    pub location: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConstraintKind {
    Outlives { longer: String, shorter: String },
    Equal { a: String, b: String },
    BorrowedFrom { reference: String, source: String },
    MustLiveUntil { lifetime: String, scope_id: usize },
}

impl ScopedLifetimeTracker {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
            variable_scope: HashMap::new(),
            lifetime_scopes: HashMap::new(),
            constraints: Vec::new(),
            next_scope_id: 0,
        }
    }
    
    /// Create a new scope
    pub fn push_scope(&mut self, kind: ScopeKind, parent: Option<usize>) -> usize {
        let id = self.next_scope_id;
        self.next_scope_id += 1;
        
        self.scopes.insert(id, Scope {
            id,
            parent,
            kind,
            local_variables: HashSet::new(),
            local_references: HashSet::new(),
            entry_block: None,
            exit_blocks: Vec::new(),
        });
        
        id
    }
    
    /// Add a variable to the current scope
    pub fn declare_variable(&mut self, var: String, scope_id: usize) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.local_variables.insert(var.clone());
            self.variable_scope.insert(var, scope_id);
        }
    }
    
    /// Add a reference to the current scope
    pub fn declare_reference(&mut self, ref_name: String, scope_id: usize) {
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            scope.local_references.insert(ref_name.clone());
            self.variable_scope.insert(ref_name, scope_id);
        }
    }
    
    /// Check if a lifetime outlives another considering scopes
    pub fn check_outlives(&self, longer: &str, shorter: &str) -> bool {
        // Get the scope ranges for both lifetimes
        let longer_range = self.lifetime_scopes.get(longer);
        let shorter_range = self.lifetime_scopes.get(shorter);
        
        match (longer_range, shorter_range) {
            (Some((l_start, l_end)), Some((s_start, s_end))) => {
                // longer outlives shorter if it starts before or at the same time
                // and ends after or at the same time
                l_start <= s_start && l_end >= s_end
            }
            _ => {
                // If we don't know the scopes, check if they're equal
                longer == shorter
            }
        }
    }
    
    /// Check if a variable is still alive in a given scope
    pub fn is_alive_in_scope(&self, var: &str, scope_id: usize) -> bool {
        if let Some(var_scope) = self.variable_scope.get(var) {
            // Check if the variable's scope is an ancestor of the given scope
            self.is_ancestor_scope(*var_scope, scope_id)
        } else {
            false
        }
    }
    
    /// Check if scope_a is an ancestor of scope_b
    fn is_ancestor_scope(&self, scope_a: usize, scope_b: usize) -> bool {
        if scope_a == scope_b {
            return true;
        }
        
        let mut current = scope_b;
        while let Some(scope) = self.scopes.get(&current) {
            if let Some(parent) = scope.parent {
                if parent == scope_a {
                    return true;
                }
                current = parent;
            } else {
                break;
            }
        }
        
        false
    }
    
    /// Add a lifetime constraint
    pub fn add_constraint(&mut self, constraint: LifetimeConstraint) {
        self.constraints.push(constraint);
    }
    
    /// Validate all lifetime constraints
    pub fn validate_constraints(&self) -> Vec<String> {
        let mut errors = Vec::new();
        
        for constraint in &self.constraints {
            match &constraint.kind {
                ConstraintKind::Outlives { longer, shorter } => {
                    if !self.check_outlives(longer, shorter) {
                        errors.push(format!(
                            "Lifetime '{}' does not outlive '{}' at {}",
                            longer, shorter, constraint.location
                        ));
                    }
                }
                ConstraintKind::BorrowedFrom { reference, source } => {
                    // Skip alive check for function call results (temporaries)
                    // These include operator* (dereference), operator-> (member access), etc.
                    let is_function_call_result = source.starts_with("operator") ||
                        source.contains("::") ||  // Qualified function calls
                        source.starts_with("__") || // Compiler-generated temporaries
                        source.starts_with("temp_"); // Explicit temporaries

                    // Check that the source is alive where the reference is used (skip for temporaries)
                    if !is_function_call_result {
                        if let Some(ref_scope) = self.variable_scope.get(reference) {
                            if !self.is_alive_in_scope(source, *ref_scope) {
                                errors.push(format!(
                                    "Reference '{}' borrows from '{}' which is not alive at {}",
                                    reference, source, constraint.location
                                ));
                            }
                        }
                    }
                }
                ConstraintKind::MustLiveUntil { lifetime, scope_id } => {
                    if let Some((_, end_scope)) = self.lifetime_scopes.get(lifetime) {
                        if !self.is_ancestor_scope(*end_scope, *scope_id) && *end_scope != *scope_id {
                            errors.push(format!(
                                "Lifetime '{}' must live until scope {} but ends at scope {} at {}",
                                lifetime, scope_id, end_scope, constraint.location
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        
        errors
    }
}

/// Analyze a function with scope-based lifetime tracking
pub fn analyze_function_scopes(
    function: &IrFunction,
    header_cache: &HeaderCache,
) -> Result<Vec<String>, String> {
    let mut tracker = ScopedLifetimeTracker::new();
    let mut errors = Vec::new();
    
    // Create function scope
    let func_scope = tracker.push_scope(ScopeKind::Function, None);
    
    // Initialize function parameters
    for (name, var_info) in &function.variables {
        tracker.declare_variable(name.clone(), func_scope);
        
        // If it's a reference, track its lifetime
        match &var_info.ty {
            crate::ir::VariableType::Reference(_) |
            crate::ir::VariableType::MutableReference(_) => {
                tracker.declare_reference(name.clone(), func_scope);
                // Assign a lifetime based on the parameter position
                let lifetime = format!("'param_{}", name);
                tracker.lifetime_scopes.insert(lifetime, (func_scope, func_scope));
            }
            _ => {}
        }
    }
    
    // Analyze each block in the CFG
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    
    // Start with entry blocks
    for node in function.cfg.node_indices() {
        if function.cfg.edges_directed(node, Direction::Incoming).count() == 0 {
            queue.push_back((node, func_scope));
        }
    }
    
    while let Some((node_idx, current_scope)) = queue.pop_front() {
        if visited.contains(&node_idx) {
            continue;
        }
        visited.insert(node_idx);
        
        let block = &function.cfg[node_idx];
        let block_errors = analyze_block(
            block,
            current_scope,
            &mut tracker,
            function,
            header_cache,
        )?;
        errors.extend(block_errors);
        
        // Queue successor blocks
        for neighbor in function.cfg.neighbors_directed(node_idx, Direction::Outgoing) {
            queue.push_back((neighbor, current_scope));
        }
    }
    
    // Validate all constraints
    errors.extend(tracker.validate_constraints());
    
    Ok(errors)
}

fn analyze_block(
    block: &BasicBlock,
    scope_id: usize,
    tracker: &mut ScopedLifetimeTracker,
    function: &IrFunction,
    header_cache: &HeaderCache,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    
    for statement in &block.statements {
        match statement {
            IrStatement::Borrow { from, to, .. } => {
                // Skip alive check for function call results (temporaries)
                // These include operator* (dereference), operator-> (member access), etc.
                // Temporaries are alive for the duration of the full expression
                let is_function_call_result = from.starts_with("operator") ||
                    from.contains("::") ||  // Qualified function calls
                    from.starts_with("__") || // Compiler-generated temporaries
                    from.starts_with("temp_"); // Explicit temporaries

                // Check that 'from' is alive in this scope (skip for temporaries)
                if !is_function_call_result && !tracker.is_alive_in_scope(from, scope_id) {
                    errors.push(format!(
                        "Cannot borrow from '{}': variable is not alive in current scope",
                        from
                    ));
                } else {
                    // Create a borrow constraint
                    tracker.add_constraint(LifetimeConstraint {
                        kind: ConstraintKind::BorrowedFrom {
                            reference: to.clone(),
                            source: from.clone(),
                        },
                        location: format!("borrow of {} from {}", to, from),
                    });
                    
                    // Track the new reference
                    tracker.declare_reference(to.clone(), scope_id);
                }
            }
            
            IrStatement::Return { value } => {
                if let Some(val) = value {
                    // Check for dangling references
                    if let Some(var_scope) = tracker.variable_scope.get(val) {
                        // If returning a reference to a local variable, it's an error
                        // Static variables are OK to return (they have 'static lifetime)
                        if *var_scope != 0 && !is_parameter(val, function) {
                            // Check if this is a reference type and not a static variable
                            if let Some(var_info) = function.variables.get(val) {
                                if !var_info.is_static {  // Static variables are safe to return
                                    match var_info.ty {
                                        crate::ir::VariableType::Reference(_) |
                                        crate::ir::VariableType::MutableReference(_) => {
                                            errors.push(format!(
                                                "Returning reference to local variable '{}' - will create dangling reference",
                                                val
                                            ));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            IrStatement::CallExpr { func, args, result } => {
                // Check function signature if available
                if let Some(signature) = header_cache.get_signature(func) {
                    let call_errors = check_call_lifetimes(
                        func,
                        args,
                        result.as_ref(),
                        signature,
                        scope_id,
                        tracker,
                    );
                    errors.extend(call_errors);
                }
            }
            
            _ => {}
        }
    }
    
    Ok(errors)
}

fn check_call_lifetimes(
    func_name: &str,
    _args: &[String],
    result: Option<&String>,
    signature: &FunctionSignature,
    scope_id: usize,
    tracker: &mut ScopedLifetimeTracker,
) -> Vec<String> {
    let errors = Vec::new();
    
    // Check lifetime bounds from signature
    for bound in &signature.lifetime_bounds {
        // For each bound like 'a: 'b, ensure the constraint is satisfied
        // This would require mapping signature lifetimes to actual argument lifetimes
        tracker.add_constraint(LifetimeConstraint {
            kind: ConstraintKind::Outlives {
                longer: bound.longer.clone(),
                shorter: bound.shorter.clone(),
            },
            location: format!("call to {}", func_name),
        });
    }
    
    // Check return lifetime
    if let (Some(result_var), Some(return_lifetime)) = (result, &signature.return_lifetime) {
        match return_lifetime {
            LifetimeAnnotation::Ref(_) | LifetimeAnnotation::MutRef(_) => {
                // The result is a reference - track it
                tracker.declare_reference(result_var.clone(), scope_id);
            }
            LifetimeAnnotation::Owned => {
                // The result is owned
                tracker.declare_variable(result_var.clone(), scope_id);
            }
            _ => {}
        }
    }
    
    errors
}

fn is_parameter(var_name: &str, function: &IrFunction) -> bool {
    // Check if this variable was declared as a parameter
    // In a real implementation, we'd track this properly
    // For now, use a heuristic
    var_name.starts_with("param") || var_name.starts_with("arg") || 
    (function.variables.get(var_name).map_or(false, |info| {
        // If it's in the variables map but not initialized in the function body,
        // it's likely a parameter
        matches!(info.ownership, crate::ir::OwnershipState::Owned)
    }))
}

/// Check lifetimes for the entire program with scope tracking
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

pub fn check_scoped_lifetimes(
    program: &IrProgram,
    header_cache: &HeaderCache,
) -> Result<Vec<String>, String> {
    let mut all_errors = Vec::new();

    for function in &program.functions {
        // Skip system header functions
        if is_system_header(&function.source_file) {
            continue;
        }

        let errors = analyze_function_scopes(function, header_cache)?;
        all_errors.extend(errors);
    }

    Ok(all_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_tracking() {
        let mut tracker = ScopedLifetimeTracker::new();
        
        let func_scope = tracker.push_scope(ScopeKind::Function, None);
        let block_scope = tracker.push_scope(ScopeKind::Block, Some(func_scope));
        
        tracker.declare_variable("x".to_string(), func_scope);
        tracker.declare_variable("y".to_string(), block_scope);
        
        assert!(tracker.is_alive_in_scope("x", block_scope));
        assert!(tracker.is_alive_in_scope("y", block_scope));
        assert!(!tracker.is_alive_in_scope("y", func_scope));
    }
    
    #[test]
    fn test_outlives_checking() {
        let mut tracker = ScopedLifetimeTracker::new();
        
        tracker.lifetime_scopes.insert("'a".to_string(), (0, 2));
        tracker.lifetime_scopes.insert("'b".to_string(), (1, 2));
        
        assert!(tracker.check_outlives("'a", "'b"));
        assert!(!tracker.check_outlives("'b", "'a"));
    }
}