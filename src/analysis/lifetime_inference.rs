use std::collections::{HashMap, HashSet};
use crate::ir::{IrFunction, IrStatement, BorrowKind};
use crate::debug_println;

/// Check if a return type string represents a reference
fn returns_reference(return_type: &str) -> bool {
    // Check for reference types: &, const &, const Type&, Type&, etc.
    return_type.contains('&') && !return_type.contains("&&") // Exclude rvalue references
}

/// Represents an inferred lifetime for a variable
#[derive(Debug, Clone, PartialEq)]
pub struct InferredLifetime {
    pub name: String,
    /// The point where this lifetime begins (statement index)
    pub start: usize,
    /// The point where this lifetime ends (statement index)
    pub end: usize,
    /// Variables that this lifetime depends on
    pub dependencies: HashSet<String>,
}

/// Infers lifetimes for local variables based on their usage patterns
pub struct LifetimeInferencer {
    /// Maps variable names to their inferred lifetimes
    lifetimes: HashMap<String, InferredLifetime>,
    /// Tracks the last use of each variable
    last_use: HashMap<String, usize>,
    /// Tracks when variables are first defined
    first_def: HashMap<String, usize>,
    /// Counter for generating unique lifetime names
    lifetime_counter: usize,
}

impl LifetimeInferencer {
    pub fn new() -> Self {
        Self {
            lifetimes: HashMap::new(),
            last_use: HashMap::new(),
            first_def: HashMap::new(),
            lifetime_counter: 0,
        }
    }
    
    /// Generate a unique lifetime name
    fn gen_lifetime(&mut self) -> String {
        let name = format!("'inferred_{}", self.lifetime_counter);
        self.lifetime_counter += 1;
        name
    }
    
    /// Infer lifetimes for all variables in a function
    pub fn infer_function_lifetimes(&mut self, function: &IrFunction) -> HashMap<String, InferredLifetime> {
        // First pass: collect all variable definitions and uses
        let mut statement_index = 0;
        
        for node_idx in function.cfg.node_indices() {
            let block = &function.cfg[node_idx];
            
            for statement in &block.statements {
                self.process_statement(statement, statement_index);
                statement_index += 1;
            }
        }
        
        // Second pass: create lifetime ranges
        let first_def_clone = self.first_def.clone();
        for (var, &first_def_idx) in &first_def_clone {
            let last_use_idx = self.last_use.get(var).copied().unwrap_or(first_def_idx);
            
            let lifetime = InferredLifetime {
                name: self.gen_lifetime(),
                start: first_def_idx,
                end: last_use_idx,
                dependencies: HashSet::new(),
            };
            
            self.lifetimes.insert(var.clone(), lifetime);
        }
        
        // Third pass: infer dependencies between lifetimes
        statement_index = 0;
        for node_idx in function.cfg.node_indices() {
            let block = &function.cfg[node_idx];
            
            for statement in &block.statements {
                self.infer_dependencies(statement, statement_index);
                statement_index += 1;
            }
        }
        
        self.lifetimes.clone()
    }
    
    /// Process a statement to track variable definitions and uses
    fn process_statement(&mut self, statement: &IrStatement, index: usize) {
        match statement {
            IrStatement::Assign { lhs, .. } => {
                self.first_def.entry(lhs.clone()).or_insert(index);
                self.last_use.insert(lhs.clone(), index);
            }
            
            IrStatement::Move { from, to } => {
                self.last_use.insert(from.clone(), index);
                self.first_def.entry(to.clone()).or_insert(index);
                self.last_use.insert(to.clone(), index);
            }
            
            IrStatement::Borrow { from, to, .. } => {
                self.last_use.insert(from.clone(), index);
                self.first_def.entry(to.clone()).or_insert(index);
                self.last_use.insert(to.clone(), index);
            }
            
            IrStatement::CallExpr { args, result, .. } => {
                for arg in args {
                    self.last_use.insert(arg.clone(), index);
                }
                if let Some(res) = result {
                    self.first_def.entry(res.clone()).or_insert(index);
                    self.last_use.insert(res.clone(), index);
                }
            }
            
            IrStatement::Return { value } => {
                if let Some(val) = value {
                    self.last_use.insert(val.clone(), index);
                }
            }
            
            _ => {}
        }
    }
    
    /// Infer dependencies between lifetimes based on borrows and moves
    fn infer_dependencies(&mut self, statement: &IrStatement, _index: usize) {
        match statement {
            IrStatement::Borrow { from, to, kind } => {
                // The lifetime of 'to' depends on 'from'
                if let Some(to_lifetime) = self.lifetimes.get_mut(to) {
                    to_lifetime.dependencies.insert(from.clone());
                    
                    // For mutable borrows, the lifetime must be exclusive
                    if matches!(kind, BorrowKind::Mutable) {
                        // Mark that this lifetime requires exclusive access
                        // In a full implementation, we'd track this for conflict detection
                    }
                }
            }
            
            IrStatement::Move { from, to } => {
                // After a move, 'from' lifetime ends and 'to' lifetime begins
                if let Some(from_lifetime) = self.lifetimes.get(from).cloned() {
                    if let Some(to_lifetime) = self.lifetimes.get_mut(to) {
                        // 'to' inherits dependencies from 'from'
                        to_lifetime.dependencies.extend(from_lifetime.dependencies);
                    }
                }
            }
            
            _ => {}
        }
    }
    
    /// Check if two lifetimes overlap
    pub fn lifetimes_overlap(&self, a: &str, b: &str) -> bool {
        if let (Some(lifetime_a), Some(lifetime_b)) = (self.lifetimes.get(a), self.lifetimes.get(b)) {
            // Check if the ranges [start_a, end_a] and [start_b, end_b] overlap
            !(lifetime_a.end < lifetime_b.start || lifetime_b.end < lifetime_a.start)
        } else {
            false
        }
    }
    
    /// Get the inferred lifetime for a variable
    #[allow(dead_code)]
    pub fn get_lifetime(&self, var: &str) -> Option<&InferredLifetime> {
        self.lifetimes.get(var)
    }
    
    /// Check if a variable is alive at a given point
    pub fn is_alive_at(&self, var: &str, point: usize) -> bool {
        if let Some(lifetime) = self.lifetimes.get(var) {
            point >= lifetime.start && point <= lifetime.end
        } else {
            false
        }
    }
}

/// Perform lifetime inference and validation
pub fn infer_and_validate_lifetimes(function: &IrFunction) -> Result<Vec<String>, String> {
    let mut inferencer = LifetimeInferencer::new();
    let lifetimes = inferencer.infer_function_lifetimes(function);
    let mut errors = Vec::new();

    // Check for conflicts between inferred lifetimes
    let mut statement_index = 0;
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];

        for statement in &block.statements {
            match statement {
                IrStatement::Borrow { from, to, kind } => {
                    // Check that 'from' is alive when borrowed
                    // Note: We should only check this if we have actually tracked the variable
                    if inferencer.lifetimes.contains_key(from) && !inferencer.is_alive_at(from, statement_index) {
                        errors.push(format!(
                            "Cannot borrow '{}': variable is not alive at this point",
                            from
                        ));
                    }
                    
                    // For mutable borrows, check for conflicts
                    if matches!(kind, BorrowKind::Mutable) {
                        // Check if there are other borrows of 'from' that overlap with 'to'
                        for (other_var, other_lifetime) in &lifetimes {
                            if other_var != to && other_lifetime.dependencies.contains(from) {
                                if inferencer.lifetimes_overlap(to, other_var) {
                                    // Only error if the other borrow is also mutable or we have a mutable borrow
                                    // Multiple immutable borrows are allowed
                                    errors.push(format!(
                                        "Cannot create mutable borrow '{}': '{}' is already borrowed by '{}'",
                                        to, from, other_var
                                    ));
                                }
                            }
                        }
                    }
                }
                
                IrStatement::Move { from, .. } => {
                    // Check that 'from' is alive when moved
                    if !inferencer.is_alive_at(from, statement_index) {
                        errors.push(format!(
                            "Cannot move '{}': variable is not alive at this point",
                            from
                        ));
                    }
                }
                
                IrStatement::Return { value } => {
                    // PHASE 2: If function returns a reference but return value is None,
                    // it's returning a temporary (e.g., return 42;)
                    if value.is_none() && returns_reference(&function.return_type) {
                        errors.push(format!(
                            "Cannot return reference to temporary value in function '{}'",
                            function.name
                        ));
                    }

                    if let Some(val) = value {
                        // PHASE 2: If function returns a reference, check what we're returning
                        if returns_reference(&function.return_type) {
                            if let Some(var_info) = function.variables.get(val) {
                                // Check if we're returning a local variable
                                // This applies whether the variable is owned or already a reference
                                let is_param = is_parameter(val, function);

                                if !is_param {
                                    // Returning a local variable when function returns a reference
                                    errors.push(format!(
                                        "Cannot return reference to local variable '{}'",
                                        val
                                    ));
                                }

                                // If the returned variable is itself a reference, check dependencies
                                match var_info.ty {
                                    crate::ir::VariableType::Reference(_) |
                                    crate::ir::VariableType::MutableReference(_) => {
                                        if let Some(lifetime) = lifetimes.get(val) {
                                            // Check if it depends on local variables
                                            for dep in &lifetime.dependencies {
                                                if !is_parameter(dep, function) {
                                                    errors.push(format!(
                                                        "Potential dangling reference: returning '{}' which depends on local variable '{}'",
                                                        val, dep
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    _ => {} // Owned variable - already checked above
                                }
                            }
                        }
                    }
                }
                
                _ => {}
            }
            
            statement_index += 1;
        }
    }
    
    Ok(errors)
}

fn is_parameter(var_name: &str, function: &IrFunction) -> bool {
    // Check if variable is marked as a parameter in the IR
    function.variables.get(var_name)
        .map(|var_info| var_info.is_parameter)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::BasicBlock;
    use petgraph::graph::Graph;
    
    #[test]
    fn test_lifetime_inference() {
        let mut inferencer = LifetimeInferencer::new();
        
        // Simulate processing statements
        inferencer.process_statement(&IrStatement::Assign {
            lhs: "x".to_string(),
            rhs: crate::ir::IrExpression::Variable("temp".to_string()),
        }, 0);
        
        inferencer.process_statement(&IrStatement::Borrow {
            from: "x".to_string(),
            to: "ref_x".to_string(),
            kind: BorrowKind::Immutable,
        }, 1);
        
        inferencer.process_statement(&IrStatement::Return {
            value: Some("ref_x".to_string()),
        }, 2);
        
        // Create a dummy function for testing
        let mut cfg = Graph::new();
        let block = BasicBlock {
            id: 0,
            statements: vec![],
            terminator: None,
        };
        cfg.add_node(block);
        
        let function = IrFunction {
            name: "test".to_string(),
            cfg,
            variables: HashMap::new(),
            return_type: "void".to_string(),
        };
        
        let lifetimes = inferencer.infer_function_lifetimes(&function);
        
        // Check that lifetimes were inferred
        assert!(inferencer.first_def.contains_key("x"));
        assert!(inferencer.first_def.contains_key("ref_x"));
        assert_eq!(inferencer.last_use.get("x"), Some(&1));
        assert_eq!(inferencer.last_use.get("ref_x"), Some(&2));
    }
    
    #[test]
    fn test_lifetime_overlap() {
        let mut inferencer = LifetimeInferencer::new();
        
        inferencer.lifetimes.insert("a".to_string(), InferredLifetime {
            name: "'a".to_string(),
            start: 0,
            end: 5,
            dependencies: HashSet::new(),
        });
        
        inferencer.lifetimes.insert("b".to_string(), InferredLifetime {
            name: "'b".to_string(),
            start: 3,
            end: 7,
            dependencies: HashSet::new(),
        });
        
        inferencer.lifetimes.insert("c".to_string(), InferredLifetime {
            name: "'c".to_string(),
            start: 6,
            end: 10,
            dependencies: HashSet::new(),
        });
        
        assert!(inferencer.lifetimes_overlap("a", "b")); // [0,5] and [3,7] overlap
        assert!(!inferencer.lifetimes_overlap("a", "c")); // [0,5] and [6,10] don't overlap
        assert!(inferencer.lifetimes_overlap("b", "c")); // [3,7] and [6,10] overlap
    }
}