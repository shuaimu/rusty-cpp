use crate::ir::BorrowKind;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BorrowChecker {
    active_borrows: HashMap<String, ActiveBorrows>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct ActiveBorrows {
    immutable: HashSet<String>,
    mutable: Option<String>,
}

impl BorrowChecker {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            active_borrows: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn check_borrow(&self, target: &str, kind: &BorrowKind) -> Result<(), String> {
        if let Some(borrows) = self.active_borrows.get(target) {
            match kind {
                BorrowKind::Immutable => {
                    if borrows.mutable.is_some() {
                        return Err(format!(
                            "Cannot borrow '{}' as immutable while it's mutably borrowed",
                            target
                        ));
                    }
                }
                BorrowKind::Mutable => {
                    if !borrows.immutable.is_empty() {
                        return Err(format!(
                            "Cannot borrow '{}' as mutable while it has {} immutable borrow(s)",
                            target,
                            borrows.immutable.len()
                        ));
                    }
                    if borrows.mutable.is_some() {
                        return Err(format!(
                            "Cannot borrow '{}' as mutable while it's already mutably borrowed",
                            target
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_borrow(&mut self, target: String, borrower: String, kind: BorrowKind) {
        let borrows = self.active_borrows.entry(target).or_default();

        match kind {
            BorrowKind::Immutable => {
                borrows.immutable.insert(borrower);
            }
            BorrowKind::Mutable => {
                borrows.mutable = Some(borrower);
            }
        }
    }

    #[allow(dead_code)]
    pub fn release_borrow(&mut self, target: &str, borrower: &str) {
        if let Some(borrows) = self.active_borrows.get_mut(target) {
            borrows.immutable.remove(borrower);
            if borrows.mutable.as_ref() == Some(&borrower.to_string()) {
                borrows.mutable = None;
            }

            // Remove entry if no active borrows
            if borrows.immutable.is_empty() && borrows.mutable.is_none() {
                self.active_borrows.remove(target);
            }
        }
    }
}
