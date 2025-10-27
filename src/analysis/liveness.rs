// Liveness Analysis for Conservative Borrow Clearing
//
// This module implements conservative liveness analysis to determine when
// variables have their "last use" and can be safely considered dead.
//
// Conservative Rules:
// - Variables used in loops → live until loop exit
// - Variables passed to functions → live (might be stored)
// - Variables that escape scope → live (returned, stored, etc.)
// - When uncertain → assume live (safe default)

use crate::ir::{IrStatement, IrFunction};
use std::collections::{HashMap, HashSet};
use crate::debug_println;

#[derive(Debug, Clone, PartialEq)]
pub enum UseType {
    Read,           // Simple read: int x = r
    Write,          // Assignment: r = ...
    Escape,         // Returned, might escape scope
    FunctionArg,    // Passed to function (might be stored)
    InLoop,         // Used inside loop body
    InCondition,    // Used in if/else condition
}

#[derive(Debug, Clone)]
pub struct UseInfo {
    pub statement_idx: usize,
    pub use_type: UseType,
}

pub struct LivenessAnalyzer {
    // For each variable, track all its uses
    uses: HashMap<String, Vec<UseInfo>>,
    // Track current statement index
    current_idx: usize,
    // Track if we're in a loop
    in_loop_depth: usize,
    // Track if we're in a conditional
    in_conditional_depth: usize,
}

impl LivenessAnalyzer {
    pub fn new() -> Self {
        Self {
            uses: HashMap::new(),
            current_idx: 0,
            in_loop_depth: 0,
            in_conditional_depth: 0,
        }
    }

    /// Analyze a function and return map of variable -> last_use_statement_index
    /// Conservative: only returns last use if we're CERTAIN the variable won't be used again
    pub fn analyze(&mut self, function: &IrFunction) -> HashMap<String, usize> {
        // Get the first basic block (main function body)
        // For now, we only analyze the first block for simplicity
        let first_block = function.cfg.node_weights().next();

        if let Some(block) = first_block {
            debug_println!("LIVENESS: Analyzing function with {} statements", block.statements.len());

            // First pass: collect all uses
            self.collect_uses(&block.statements);
        } else {
            debug_println!("LIVENESS: No basic blocks found in function");
        }

        // Second pass: determine last uses (conservatively)
        let last_uses = self.compute_last_uses();

        debug_println!("LIVENESS: Found {} variables with determinable last use", last_uses.len());
        for (var, idx) in &last_uses {
            debug_println!("LIVENESS:   '{}' last used at statement {}", var, idx);
        }

        last_uses
    }

    fn collect_uses(&mut self, statements: &[IrStatement]) {
        for (idx, stmt) in statements.iter().enumerate() {
            self.current_idx = idx;
            self.collect_uses_from_statement(stmt);
        }
    }

    fn collect_uses_from_statement(&mut self, stmt: &IrStatement) {
        match stmt {
            IrStatement::Borrow { from, to, .. } => {
                // 'from' is being borrowed (read)
                self.record_use(from, UseType::Read);
                // 'to' is the new reference (written to)
                self.record_use(to, UseType::Write);
            }

            IrStatement::Move { from, to } => {
                // 'from' is being moved (read + consumed)
                self.record_use(from, UseType::Read);
                // 'to' is receiving the value (written to)
                self.record_use(to, UseType::Write);
            }

            IrStatement::Assign { lhs, rhs } => {
                // RHS is being read
                match rhs {
                    crate::ir::IrExpression::Variable(var) => {
                        self.record_use(var, UseType::Read);
                    }
                    crate::ir::IrExpression::Move(var) => {
                        self.record_use(var, UseType::Read);
                    }
                    crate::ir::IrExpression::Borrow(var, _) => {
                        self.record_use(var, UseType::Read);
                    }
                    crate::ir::IrExpression::New(_) => {
                        // Allocation, no variable read
                    }
                }
                // LHS is being assigned to
                self.record_use(lhs, UseType::Write);
            }

            IrStatement::Return { value } => {
                // Returned variable escapes scope - VERY conservative
                if let Some(var) = value {
                    self.record_use(var, UseType::Escape);
                }
            }

            IrStatement::CallExpr { args, result, .. } => {
                // All arguments might be stored by the function (conservative)
                for arg in args {
                    self.record_use(arg, UseType::FunctionArg);
                }
                // Result is written to
                if let Some(res) = result {
                    self.record_use(res, UseType::Write);
                }
            }

            IrStatement::UseVariable { var, .. } => {
                // Variable is being used (read)
                let use_type = if self.in_loop_depth > 0 {
                    UseType::InLoop
                } else if self.in_conditional_depth > 0 {
                    UseType::InCondition
                } else {
                    UseType::Read
                };
                self.record_use(var, use_type);
            }

            IrStatement::UseField { object, .. } => {
                // Object is being accessed
                let use_type = if self.in_loop_depth > 0 {
                    UseType::InLoop
                } else {
                    UseType::Read
                };
                self.record_use(object, use_type);
            }

            IrStatement::MoveField { object, to, .. } => {
                // Object field is being moved
                self.record_use(object, UseType::Read);
                self.record_use(to, UseType::Write);
            }

            IrStatement::BorrowField { object, to, .. } => {
                // Object field is being borrowed
                self.record_use(object, UseType::Read);
                self.record_use(to, UseType::Write);
            }

            IrStatement::EnterLoop => {
                self.in_loop_depth += 1;
            }

            IrStatement::ExitLoop => {
                if self.in_loop_depth > 0 {
                    self.in_loop_depth -= 1;
                }
            }

            IrStatement::If { then_branch, else_branch } => {
                self.in_conditional_depth += 1;

                // Analyze then branch
                self.collect_uses(then_branch);

                // Analyze else branch
                if let Some(else_stmts) = else_branch {
                    self.collect_uses(else_stmts);
                }

                self.in_conditional_depth -= 1;
            }

            // These don't use variables
            IrStatement::EnterScope |
            IrStatement::ExitScope |
            IrStatement::EnterUnsafe |
            IrStatement::ExitUnsafe |
            IrStatement::Drop(_) |
            IrStatement::ImplicitDrop { .. } => {}
        }
    }

    fn record_use(&mut self, var: &str, use_type: UseType) {
        // Skip special variables (temporaries, field accesses)
        if var.starts_with('_') || var.contains('.') {
            return;
        }

        debug_println!("LIVENESS: Recording use of '{}' at index {} (type: {:?})",
                      var, self.current_idx, use_type);

        let use_info = UseInfo {
            statement_idx: self.current_idx,
            use_type,
        };

        self.uses.entry(var.to_string()).or_default().push(use_info);
    }

    fn compute_last_uses(&self) -> HashMap<String, usize> {
        let mut last_uses = HashMap::new();

        for (var, uses) in &self.uses {
            // Conservative rules: DON'T compute last use if:

            // 1. Variable escapes scope
            if uses.iter().any(|u| matches!(u.use_type, UseType::Escape)) {
                debug_println!("LIVENESS: '{}' escapes scope - not clearing", var);
                continue;
            }

            // 2. Variable used in loop
            if uses.iter().any(|u| matches!(u.use_type, UseType::InLoop)) {
                debug_println!("LIVENESS: '{}' used in loop - not clearing", var);
                continue;
            }

            // 3. Variable passed to function (might be stored)
            if uses.iter().any(|u| matches!(u.use_type, UseType::FunctionArg)) {
                debug_println!("LIVENESS: '{}' passed to function - not clearing", var);
                continue;
            }

            // Conservative: find the LAST read/use
            let last_read_or_use = uses.iter()
                .filter(|u| matches!(u.use_type, UseType::Read | UseType::InCondition))
                .map(|u| u.statement_idx)
                .max();

            if let Some(last_idx) = last_read_or_use {
                debug_println!("LIVENESS: '{}' has last use (read) at statement {}", var, last_idx);
                last_uses.insert(var.clone(), last_idx);
            }
            // Note: Variables that are only written (never read) don't get a last use
            // Their borrows stay active until scope end (handled by drop order tracking)
        }

        last_uses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liveness_simple_sequence() {
        // This is a unit test for the liveness analyzer
        // Will be expanded once integrated with IR
        let mut analyzer = LivenessAnalyzer::new();
        assert_eq!(analyzer.uses.len(), 0);
    }

    #[test]
    fn test_conservative_loop() {
        // Variables used in loops should NOT have determinable last use
        let mut analyzer = LivenessAnalyzer::new();

        // Simulate: r used in loop
        analyzer.in_loop_depth = 1;
        analyzer.current_idx = 5;
        analyzer.record_use("r", UseType::InLoop);

        let last_uses = analyzer.compute_last_uses();

        // Should NOT have last use for r (used in loop)
        assert!(!last_uses.contains_key("r"));
    }

    #[test]
    fn test_escaped_variable() {
        // Variables that escape should NOT have determinable last use
        let mut analyzer = LivenessAnalyzer::new();

        analyzer.current_idx = 5;
        analyzer.record_use("r", UseType::Escape);

        let last_uses = analyzer.compute_last_uses();

        // Should NOT have last use for r (escaped)
        assert!(!last_uses.contains_key("r"));
    }

    #[test]
    fn test_function_argument() {
        // Variables passed to functions should NOT have determinable last use
        let mut analyzer = LivenessAnalyzer::new();

        analyzer.current_idx = 5;
        analyzer.record_use("r", UseType::FunctionArg);

        let last_uses = analyzer.compute_last_uses();

        // Should NOT have last use for r (passed to function)
        assert!(!last_uses.contains_key("r"));
    }

    #[test]
    fn test_simple_read_sequence() {
        // Simple read sequence should have determinable last use
        let mut analyzer = LivenessAnalyzer::new();

        analyzer.current_idx = 2;
        analyzer.record_use("r", UseType::Write);  // Created
        analyzer.current_idx = 3;
        analyzer.record_use("r", UseType::Read);   // Used
        analyzer.current_idx = 4;
        analyzer.record_use("r", UseType::Read);   // Last use

        let last_uses = analyzer.compute_last_uses();

        // Should have last use at index 4
        assert_eq!(last_uses.get("r"), Some(&4));
    }
}
