// Example implementation of scope-aware ownership tracking
// This would be a first step toward fixing control flow issues

use std::collections::{HashMap, HashSet};
use crate::ir::{IrFunction, IrStatement, OwnershipState, BorrowKind};

/// Enhanced ownership tracker that understands scopes
pub struct ScopeAwareOwnershipTracker {
    // Stack of scopes (innermost last)
    scope_stack: Vec<Scope>,
    // Current ownership states
    ownership: HashMap<String, OwnershipState>,
    // Active borrows with their scope
    borrows: HashMap<String, BorrowInfo>,
}

#[derive(Debug, Clone)]
struct Scope {
    id: usize,
    // Variables declared in this scope
    local_variables: HashSet<String>,
    // References created in this scope
    local_borrows: HashSet<String>,
}

#[derive(Debug, Clone)]
struct BorrowInfo {
    from: String,
    kind: BorrowKind,
    scope_id: usize,
}

impl ScopeAwareOwnershipTracker {
    pub fn new() -> Self {
        let mut tracker = Self {
            scope_stack: Vec::new(),
            ownership: HashMap::new(),
            borrows: HashMap::new(),
        };
        // Start with global/function scope
        tracker.enter_scope();
        tracker
    }
    
    /// Enter a new scope (e.g., entering a {} block)
    pub fn enter_scope(&mut self) {
        let scope = Scope {
            id: self.scope_stack.len(),
            local_variables: HashSet::new(),
            local_borrows: HashSet::new(),
        };
        self.scope_stack.push(scope);
    }
    
    /// Exit current scope, cleaning up local variables and borrows
    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            // Remove all borrows created in this scope
            for borrow_name in &scope.local_borrows {
                if let Some(borrow_info) = self.borrows.remove(borrow_name) {
                    // The borrow is gone, so the source is no longer borrowed
                    // In real implementation, would update borrow count
                    println!("Borrow {} of {} ends with scope", borrow_name, borrow_info.from);
                }
            }
            
            // Remove ownership info for local variables
            for var in &scope.local_variables {
                self.ownership.remove(var);
                println!("Variable {} goes out of scope", var);
            }
        }
    }
    
    /// Declare a variable in current scope
    pub fn declare_variable(&mut self, name: String, state: OwnershipState) {
        if let Some(current_scope) = self.scope_stack.last_mut() {
            current_scope.local_variables.insert(name.clone());
        }
        self.ownership.insert(name, state);
    }
    
    /// Create a borrow in current scope
    pub fn create_borrow(&mut self, from: String, to: String, kind: BorrowKind) -> Result<(), String> {
        // Check if source is available
        match self.ownership.get(&from) {
            Some(OwnershipState::Owned) => {
                // Check for existing borrows
                let existing_borrows: Vec<_> = self.borrows.values()
                    .filter(|b| b.from == from)
                    .collect();
                
                if !existing_borrows.is_empty() {
                    match kind {
                        BorrowKind::Mutable => {
                            return Err(format!("Cannot mutably borrow {}: already borrowed", from));
                        }
                        BorrowKind::Immutable => {
                            // Check if any existing borrow is mutable
                            if existing_borrows.iter().any(|b| matches!(b.kind, BorrowKind::Mutable)) {
                                return Err(format!("Cannot immutably borrow {}: already mutably borrowed", from));
                            }
                        }
                    }
                }
                
                // Create the borrow
                let scope_id = self.scope_stack.last().map(|s| s.id).unwrap_or(0);
                self.borrows.insert(to.clone(), BorrowInfo {
                    from: from.clone(),
                    kind,
                    scope_id,
                });
                
                if let Some(current_scope) = self.scope_stack.last_mut() {
                    current_scope.local_borrows.insert(to.clone());
                }
                
                Ok(())
            }
            Some(OwnershipState::Moved) => {
                Err(format!("Cannot borrow {}: value has been moved", from))
            }
            _ => Err(format!("Cannot borrow {}: not available", from))
        }
    }
    
    /// Process an if statement with scope tracking
    pub fn process_if_statement(
        &mut self,
        then_branch: Vec<IrStatement>,
        else_branch: Option<Vec<IrStatement>>,
    ) -> Result<Vec<String>, String> {
        let mut errors = Vec::new();
        
        // Process then branch in its own scope
        self.enter_scope();
        for stmt in then_branch {
            if let Err(e) = self.process_statement(stmt) {
                errors.push(e);
            }
        }
        self.exit_scope();  // Clean up then-branch borrows
        
        // Process else branch in its own scope
        if let Some(else_stmts) = else_branch {
            self.enter_scope();
            for stmt in else_stmts {
                if let Err(e) = self.process_statement(stmt) {
                    errors.push(e);
                }
            }
            self.exit_scope();  // Clean up else-branch borrows
        }
        
        Ok(errors)
    }
    
    /// Process a loop with scope tracking
    pub fn process_loop(
        &mut self,
        body: Vec<IrStatement>,
    ) -> Result<Vec<String>, String> {
        let mut errors = Vec::new();
        
        // Analyze loop body twice to catch iteration issues
        
        // First iteration
        self.enter_scope();
        let initial_state = self.ownership.clone();
        for stmt in &body {
            if let Err(e) = self.process_statement(stmt.clone()) {
                errors.push(format!("First iteration: {}", e));
            }
        }
        let after_first = self.ownership.clone();
        self.exit_scope();
        
        // Second iteration (simulated) - use state after first iteration
        self.ownership = after_first;
        self.enter_scope();
        for stmt in &body {
            if let Err(e) = self.process_statement(stmt.clone()) {
                errors.push(format!("Second iteration: {}", e));
            }
        }
        self.exit_scope();
        
        // Restore state (conservative: assume loop might not execute)
        for (var, state) in initial_state {
            if let Some(after_state) = self.ownership.get(&var) {
                if state != *after_state {
                    // Variable state changed in loop - mark as uncertain
                    self.ownership.insert(var, OwnershipState::Moved); // Conservative
                }
            }
        }
        
        Ok(errors)
    }
    
    fn process_statement(&mut self, stmt: IrStatement) -> Result<(), String> {
        match stmt {
            IrStatement::Borrow { from, to, kind, .. } => {
                self.create_borrow(from, to, kind)
            }
            IrStatement::Move { from, to } => {
                match self.ownership.get(&from) {
                    Some(OwnershipState::Owned) => {
                        self.ownership.insert(from, OwnershipState::Moved);
                        self.ownership.insert(to, OwnershipState::Owned);
                        Ok(())
                    }
                    Some(OwnershipState::Moved) => {
                        Err(format!("Use after move: {}", from))
                    }
                    _ => Err(format!("Cannot move {}: not owned", from))
                }
            }
            _ => Ok(())
        }
    }
}

/// Example of how to use this in analysis
pub fn analyze_with_scopes(function: &IrFunction) -> Result<Vec<String>, String> {
    let mut tracker = ScopeAwareOwnershipTracker::new();
    let mut errors = Vec::new();
    
    // In a real implementation, we'd traverse the CFG
    // For now, this is a simplified example
    
    // Example: handle a block scope
    tracker.enter_scope(); // Enter block
    // ... process statements in block
    tracker.exit_scope();  // Exit block - borrows cleaned up
    
    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_cleanup() {
        let mut tracker = ScopeAwareOwnershipTracker::new();
        
        // Declare a variable
        tracker.declare_variable("x".to_string(), OwnershipState::Owned);
        
        // Enter a new scope and create a borrow
        tracker.enter_scope();
        assert!(tracker.create_borrow(
            "x".to_string(),
            "ref_x".to_string(),
            BorrowKind::Immutable
        ).is_ok());
        
        // Exit scope - borrow should be cleaned up
        tracker.exit_scope();
        
        // Should be able to borrow again
        assert!(tracker.create_borrow(
            "x".to_string(),
            "ref_x2".to_string(),
            BorrowKind::Mutable
        ).is_ok());
    }
    
    #[test]
    fn test_loop_second_iteration() {
        let mut tracker = ScopeAwareOwnershipTracker::new();
        
        tracker.declare_variable("x".to_string(), OwnershipState::Owned);
        
        let loop_body = vec![
            IrStatement::Move {
                from: "x".to_string(),
                to: "y".to_string(),
            }
        ];
        
        let errors = tracker.process_loop(loop_body).unwrap();
        
        // Should detect use-after-move in second iteration
        assert!(errors.iter().any(|e| e.contains("Second iteration")));
    }
}