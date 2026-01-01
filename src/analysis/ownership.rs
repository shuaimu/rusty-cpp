use crate::ir::{OwnershipState, IrStatement, IrExpression};
use std::collections::HashMap;

#[allow(dead_code)]
pub struct OwnershipAnalyzer {
    states: HashMap<String, OwnershipState>,
}

impl OwnershipAnalyzer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }
    
    #[allow(dead_code)]
    pub fn analyze_statement(&mut self, stmt: &IrStatement) -> Result<(), String> {
        match stmt {
            IrStatement::Assign { lhs, rhs, .. } => {
                match rhs {
                    IrExpression::Move(from) => {
                        // Check if source is available
                        match self.states.get(from) {
                            Some(OwnershipState::Owned) => {
                                self.states.insert(from.clone(), OwnershipState::Moved);
                                self.states.insert(lhs.clone(), OwnershipState::Owned);
                            }
                            Some(OwnershipState::Moved) => {
                                return Err(format!("Use after move: '{}'", from));
                            }
                            _ => {}
                        }
                    }
                    IrExpression::New(_) => {
                        self.states.insert(lhs.clone(), OwnershipState::Owned);
                    }
                    _ => {}
                }
            }
            IrStatement::Drop(var) => {
                self.states.insert(var.clone(), OwnershipState::Moved);
            }
            _ => {}
        }
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn get_state(&self, var: &str) -> Option<&OwnershipState> {
        self.states.get(var)
    }
}