use crate::ir::Lifetime;
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub struct LifetimeAnalyzer {
    lifetimes: HashMap<String, Lifetime>,
    constraints: Vec<LifetimeConstraint>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LifetimeConstraint {
    Outlives(String, String), // 'a: 'b (a outlives b)
    Equal(String, String),    // 'a = 'b
}

impl LifetimeAnalyzer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            lifetimes: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add_lifetime(&mut self, name: String, scope_start: usize, scope_end: usize) {
        self.lifetimes.insert(
            name.clone(),
            Lifetime {
                name,
                scope_start,
                scope_end,
            },
        );
    }

    #[allow(dead_code)]
    pub fn add_constraint(&mut self, constraint: LifetimeConstraint) {
        self.constraints.push(constraint);
    }

    #[allow(dead_code)]
    pub fn check_constraints(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for constraint in &self.constraints {
            match constraint {
                LifetimeConstraint::Outlives(longer, shorter) => {
                    if let (Some(l1), Some(l2)) =
                        (self.lifetimes.get(longer), self.lifetimes.get(shorter))
                    {
                        if l1.scope_end < l2.scope_end {
                            errors.push(format!(
                                "Lifetime '{}' does not outlive '{}'",
                                longer, shorter
                            ));
                        }
                    }
                }
                LifetimeConstraint::Equal(a, b) => {
                    if let (Some(l1), Some(l2)) = (self.lifetimes.get(a), self.lifetimes.get(b)) {
                        if l1.scope_start != l2.scope_start || l1.scope_end != l2.scope_end {
                            errors.push(format!("Lifetimes '{}' and '{}' are not equal", a, b));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    #[allow(dead_code)]
    pub fn infer_lifetimes(&mut self) -> HashMap<String, Lifetime> {
        // Simple lifetime inference algorithm
        // In a real implementation, this would use a constraint solver
        self.lifetimes.clone()
    }
}
